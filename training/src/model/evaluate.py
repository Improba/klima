"""Evaluation metrics for the Klima FNO model.

Computes MAE, RMSE, and R² for temperature (ΔT) and wind speed (|v|).
"""

from __future__ import annotations

from typing import Any, Dict

import torch
from torch.utils.data import DataLoader


@torch.no_grad()
def evaluate(
    model: torch.nn.Module,
    dataloader: DataLoader,
    device: torch.device,
) -> Dict[str, float]:
    """Compute evaluation metrics over a full dataset.

    Parameters
    ----------
    model : torch.nn.Module
        The trained model.
    dataloader : DataLoader
        Validation or test dataloader.
    device : torch.device
        Device for inference.

    Returns
    -------
    dict
        Keys: ``loss_total``, ``mae_temp``, ``rmse_temp``, ``r2_temp``,
        ``mae_wind``, ``rmse_wind``, ``r2_wind``.
    """
    model.eval()

    sum_se_temp = 0.0
    sum_ae_temp = 0.0
    sum_ss_tot_temp = 0.0
    sum_ss_res_temp = 0.0
    sum_target_temp = 0.0

    sum_se_wind = 0.0
    sum_ae_wind = 0.0
    sum_ss_tot_wind = 0.0
    sum_ss_res_wind = 0.0
    sum_target_wind = 0.0

    total_loss = 0.0
    n_samples = 0
    count_voxels_temp = 0
    count_voxels_wind = 0

    from .loss import PINNLoss
    criterion = PINNLoss()

    for batch in dataloader:
        inp = batch["input"].to(device, non_blocking=True)
        tgt = batch["output"].to(device, non_blocking=True)
        mask_air = batch["mask_air"].to(device, non_blocking=True)
        mask_solid = batch["mask_solid"].to(device, non_blocking=True)
        mask_surface = batch["mask_surface"].to(device, non_blocking=True)
        alpha_field = batch["alpha_field"].to(device, non_blocking=True)

        pred = model(inp)

        losses = criterion(pred, tgt, mask_air, mask_solid, mask_surface, alpha_field)
        total_loss += losses["total"].item() * inp.shape[0]
        n_samples += inp.shape[0]

        n_air = mask_air.sum().item()

        # Temperature (channel 0) — air voxels only
        dt_pred = pred[:, 0:1] * mask_air
        dt_tgt = tgt[:, 0:1] * mask_air
        diff_t = (dt_pred - dt_tgt)
        sum_se_temp += (diff_t ** 2).sum().item()
        sum_ae_temp += diff_t.abs().sum().item()
        count_voxels_temp += n_air
        sum_target_temp += dt_tgt.sum().item()

        # Wind speed magnitude — air voxels only
        pred_speed = torch.sqrt(
            pred[:, 1:2] ** 2 + pred[:, 2:3] ** 2 + pred[:, 3:4] ** 2 + 1e-8
        ) * mask_air
        tgt_speed = torch.sqrt(
            tgt[:, 1:2] ** 2 + tgt[:, 2:3] ** 2 + tgt[:, 3:4] ** 2 + 1e-8
        ) * mask_air
        diff_w = pred_speed - tgt_speed
        sum_se_wind += (diff_w ** 2).sum().item()
        sum_ae_wind += diff_w.abs().sum().item()
        count_voxels_wind += n_air
        sum_target_wind += tgt_speed.sum().item()

    count_voxels_temp = max(count_voxels_temp, 1)
    count_voxels_wind = max(count_voxels_wind, 1)

    mae_temp = sum_ae_temp / count_voxels_temp
    rmse_temp = (sum_se_temp / count_voxels_temp) ** 0.5

    mae_wind = sum_ae_wind / count_voxels_wind
    rmse_wind = (sum_se_wind / count_voxels_wind) ** 0.5

    # R² requires a second pass for the mean (or use running stats)
    mean_temp = sum_target_temp / count_voxels_temp
    mean_wind = sum_target_wind / count_voxels_wind

    ss_res_temp = sum_se_temp
    ss_res_wind = sum_se_wind

    ss_tot_temp = 0.0
    ss_tot_wind = 0.0
    for batch in dataloader:
        tgt = batch["output"].to(device, non_blocking=True)
        mask_air = batch["mask_air"].to(device, non_blocking=True)

        dt_tgt = tgt[:, 0:1]
        ss_tot_temp += (((dt_tgt - mean_temp) ** 2) * mask_air).sum().item()

        tgt_speed = torch.sqrt(
            tgt[:, 1:2] ** 2 + tgt[:, 2:3] ** 2 + tgt[:, 3:4] ** 2 + 1e-8
        )
        ss_tot_wind += (((tgt_speed - mean_wind) ** 2) * mask_air).sum().item()

    r2_temp = 1.0 - ss_res_temp / max(ss_tot_temp, 1e-8)
    r2_wind = 1.0 - ss_res_wind / max(ss_tot_wind, 1e-8)

    return {
        "loss_total": total_loss / max(n_samples, 1),
        "mae_temp": mae_temp,
        "rmse_temp": rmse_temp,
        "r2_temp": r2_temp,
        "mae_wind": mae_wind,
        "rmse_wind": rmse_wind,
        "r2_wind": r2_wind,
    }
