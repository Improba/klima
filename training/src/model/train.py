"""Training loop for the Klima Local-FNO model.

Supports mixed-precision, gradient clipping, early stopping,
TensorBoard logging, and checkpoint management.
"""

from __future__ import annotations

import argparse
import json
import os
import time
from pathlib import Path
from typing import Any, Dict

import torch
import yaml
from torch.cuda.amp import GradScaler, autocast
from torch.utils.data import DataLoader
from torch.utils.tensorboard import SummaryWriter

from .dataloader import CFDDataset
from .evaluate import evaluate
from .local_fno import LocalFNO3d
from .loss import PINNLoss


def load_config(path: str) -> Dict[str, Any]:
    with open(path, "r") as f:
        return yaml.safe_load(f)


def train(config: Dict[str, Any]) -> None:
    """Run the full training pipeline from a config dict."""
    device = torch.device("cuda" if torch.cuda.is_available() else "cpu")

    # --- Data ---
    train_ds = CFDDataset(
        config["data"]["hdf5_path"],
        split="train",
        normalize=config["data"].get("normalize", True),
        augment=config["data"].get("augment", True),
    )
    val_ds = CFDDataset(
        config["data"]["hdf5_path"],
        split="val",
        normalize=config["data"].get("normalize", True),
        augment=False,
    )

    tc = config["training"]
    train_loader = DataLoader(
        train_ds,
        batch_size=tc["batch_size"],
        shuffle=True,
        num_workers=tc.get("num_workers", 4),
        pin_memory=tc.get("pin_memory", True),
        drop_last=True,
    )
    val_loader = DataLoader(
        val_ds,
        batch_size=tc["batch_size"],
        shuffle=False,
        num_workers=tc.get("num_workers", 4),
        pin_memory=tc.get("pin_memory", True),
    )

    # --- Model ---
    mc = config["model"]
    model = LocalFNO3d(
        in_channels=mc["in_channels"],
        out_channels=mc["out_channels"],
        modes=tuple(mc["modes"]),
        width=mc["width"],
        num_layers=mc["num_layers"],
    ).to(device)

    # --- Loss ---
    lc = config["loss"]
    criterion = PINNLoss(
        lambda_temp=lc["lambda_temp"],
        lambda_wind=lc["lambda_wind"],
        lambda_div=lc["lambda_div"],
        lambda_noslip=lc["lambda_noslip"],
        lambda_diffusion=lc["lambda_diffusion"],
        dx=config["domain"]["dx"],
    )

    # --- Optimizer + scheduler ---
    optimizer = torch.optim.AdamW(
        model.parameters(),
        lr=tc["lr"],
        weight_decay=tc["weight_decay"],
    )
    scheduler = torch.optim.lr_scheduler.OneCycleLR(
        optimizer,
        max_lr=tc["lr"],
        epochs=tc["epochs"],
        steps_per_epoch=len(train_loader),
    )
    scaler = GradScaler()

    # --- Logging ---
    log_cfg = config.get("logging", {})
    tb_dir = log_cfg.get("tensorboard_dir", "runs/")
    ckpt_dir = Path(log_cfg.get("checkpoint_dir", "checkpoints/"))
    ckpt_dir.mkdir(parents=True, exist_ok=True)
    writer = SummaryWriter(log_dir=tb_dir)
    log_every = log_cfg.get("log_every", 10)

    # --- Training ---
    best_val_loss = float("inf")
    patience_counter = 0
    global_step = 0

    for epoch in range(tc["epochs"]):
        model.train()
        epoch_loss = 0.0
        t_start = time.time()

        for batch_idx, batch in enumerate(train_loader):
            inp = batch["input"].to(device, non_blocking=True)
            tgt = batch["output"].to(device, non_blocking=True)
            mask_air = batch["mask_air"].to(device, non_blocking=True)
            mask_solid = batch["mask_solid"].to(device, non_blocking=True)
            mask_surface = batch["mask_surface"].to(device, non_blocking=True)
            alpha_field = batch["alpha_field"].to(device, non_blocking=True)

            optimizer.zero_grad(set_to_none=True)

            with autocast(device_type=device.type):
                pred = model(inp)
                losses = criterion(pred, tgt, mask_air, mask_solid, mask_surface, alpha_field)
                loss = losses["total"]

            scaler.scale(loss).backward()
            scaler.unscale_(optimizer)
            torch.nn.utils.clip_grad_norm_(model.parameters(), tc["gradient_clip"])
            scaler.step(optimizer)
            scaler.update()
            scheduler.step()

            epoch_loss += loss.item()
            global_step += 1

            if global_step % log_every == 0:
                writer.add_scalar("train/loss_total", loss.item(), global_step)
                for key in ("temp", "wind", "div", "noslip", "diffusion"):
                    writer.add_scalar(f"train/loss_{key}", losses[key].item(), global_step)
                writer.add_scalar("train/lr", scheduler.get_last_lr()[0], global_step)

        epoch_loss /= max(len(train_loader), 1)
        elapsed = time.time() - t_start

        # --- Validation ---
        val_metrics = evaluate(model, val_loader, device)
        val_loss = val_metrics["loss_total"]

        writer.add_scalar("val/loss_total", val_loss, epoch)
        writer.add_scalar("val/mae_temp", val_metrics["mae_temp"], epoch)
        writer.add_scalar("val/rmse_temp", val_metrics["rmse_temp"], epoch)
        writer.add_scalar("val/r2_temp", val_metrics["r2_temp"], epoch)
        writer.add_scalar("val/mae_wind", val_metrics["mae_wind"], epoch)

        print(
            f"Epoch {epoch+1:03d}/{tc['epochs']} | "
            f"train_loss={epoch_loss:.4e} | val_loss={val_loss:.4e} | "
            f"MAE_T={val_metrics['mae_temp']:.4f} | "
            f"R2_T={val_metrics['r2_temp']:.4f} | "
            f"{elapsed:.1f}s"
        )

        # --- Checkpoint & early stopping ---
        if val_loss < best_val_loss:
            best_val_loss = val_loss
            patience_counter = 0
            torch.save(
                {
                    "epoch": epoch,
                    "model_state_dict": model.state_dict(),
                    "optimizer_state_dict": optimizer.state_dict(),
                    "val_loss": val_loss,
                    "config": config,
                },
                ckpt_dir / "best_model.pt",
            )
        else:
            patience_counter += 1
            if patience_counter >= tc["early_stopping_patience"]:
                print(f"Early stopping at epoch {epoch+1}")
                break

        torch.save(
            {
                "epoch": epoch,
                "model_state_dict": model.state_dict(),
                "optimizer_state_dict": optimizer.state_dict(),
                "val_loss": val_loss,
                "config": config,
            },
            ckpt_dir / "last_model.pt",
        )

    writer.close()

    # Save normalisation params
    norm_params = train_ds.get_norm_params()
    norm_dict = {k: v.tolist() for k, v in norm_params.items()}
    with open(ckpt_dir / "norm_params.json", "w") as f:
        json.dump(norm_dict, f, indent=2)

    print(f"Training complete. Best val loss: {best_val_loss:.4e}")


def main() -> None:
    parser = argparse.ArgumentParser(description="Train Klima Local-FNO")
    parser.add_argument("--config", type=str, default="training/configs/default.yaml")
    args = parser.parse_args()
    config = load_config(args.config)
    train(config)


if __name__ == "__main__":
    main()
