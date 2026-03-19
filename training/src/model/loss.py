"""Physics-Informed Neural Network (PINN) loss for urban microclimate FNO.

Five loss terms enforce physical consistency:
1. Temperature MSE on air voxels
2. Wind velocity MSE on air voxels
3. Divergence-free constraint on air voxels (incompressibility)
4. No-slip condition on solid voxels
5. Heat diffusion equation on surface voxels
"""

from __future__ import annotations

import torch
import torch.nn as nn
from typing import Dict


def divergence_3d(
    vx: torch.Tensor,
    vy: torch.Tensor,
    vz: torch.Tensor,
    dx: float,
) -> torch.Tensor:
    """Compute divergence of a 3D velocity field using central differences.

    Parameters
    ----------
    vx, vy, vz : torch.Tensor
        Velocity components, each of shape ``(B, 1, X, Y, Z)``.
    dx : float
        Grid spacing (assumed isotropic).

    Returns
    -------
    torch.Tensor
        Divergence field of shape ``(B, 1, X, Y, Z)``.
        Boundary voxels use forward/backward differences; interior uses central.
    """
    dvx_dx = torch.zeros_like(vx)
    dvx_dx[:, :, 1:-1, :, :] = (vx[:, :, 2:, :, :] - vx[:, :, :-2, :, :]) / (2.0 * dx)
    dvx_dx[:, :, 0, :, :] = (vx[:, :, 1, :, :] - vx[:, :, 0, :, :]) / dx
    dvx_dx[:, :, -1, :, :] = (vx[:, :, -1, :, :] - vx[:, :, -2, :, :]) / dx

    dvy_dy = torch.zeros_like(vy)
    dvy_dy[:, :, :, 1:-1, :] = (vy[:, :, :, 2:, :] - vy[:, :, :, :-2, :]) / (2.0 * dx)
    dvy_dy[:, :, :, 0, :] = (vy[:, :, :, 1, :] - vy[:, :, :, 0, :]) / dx
    dvy_dy[:, :, :, -1, :] = (vy[:, :, :, -1, :] - vy[:, :, :, -2, :]) / dx

    dvz_dz = torch.zeros_like(vz)
    dvz_dz[:, :, :, :, 1:-1] = (vz[:, :, :, :, 2:] - vz[:, :, :, :, :-2]) / (2.0 * dx)
    dvz_dz[:, :, :, :, 0] = (vz[:, :, :, :, 1] - vz[:, :, :, :, 0]) / dx
    dvz_dz[:, :, :, :, -1] = (vz[:, :, :, :, -1] - vz[:, :, :, :, -2]) / dx

    return dvx_dx + dvy_dy + dvz_dz


def laplacian_3d(field: torch.Tensor, dx: float) -> torch.Tensor:
    """Compute the 3D Laplacian using second-order central differences.

    Parameters
    ----------
    field : torch.Tensor
        Scalar field of shape ``(B, 1, X, Y, Z)``.
    dx : float
        Grid spacing (assumed isotropic).

    Returns
    -------
    torch.Tensor
        Laplacian of shape ``(B, 1, X, Y, Z)``.
        Boundary voxels are zeroed out (insufficient stencil).
    """
    lap = torch.zeros_like(field)
    dx2 = dx * dx

    # ∂²f/∂x²
    lap[:, :, 1:-1, :, :] += (
        field[:, :, 2:, :, :] - 2.0 * field[:, :, 1:-1, :, :] + field[:, :, :-2, :, :]
    ) / dx2

    # ∂²f/∂y²
    lap[:, :, :, 1:-1, :] += (
        field[:, :, :, 2:, :] - 2.0 * field[:, :, :, 1:-1, :] + field[:, :, :, :-2, :]
    ) / dx2

    # ∂²f/∂z²
    lap[:, :, :, :, 1:-1] += (
        field[:, :, :, :, 2:] - 2.0 * field[:, :, :, :, 1:-1] + field[:, :, :, :, :-2]
    ) / dx2

    return lap


class PINNLoss(nn.Module):
    """Physics-informed loss combining data-fitting and PDE residuals.

    Parameters
    ----------
    lambda_temp : float
        Weight for temperature MSE term.
    lambda_wind : float
        Weight for wind velocity MSE term.
    lambda_div : float
        Weight for divergence-free penalty.
    lambda_noslip : float
        Weight for no-slip boundary condition.
    lambda_diffusion : float
        Weight for heat diffusion residual.
    dx : float
        Grid spacing in meters.
    """

    def __init__(
        self,
        lambda_temp: float = 1.0,
        lambda_wind: float = 1.0,
        lambda_div: float = 0.01,
        lambda_noslip: float = 10.0,
        lambda_diffusion: float = 0.1,
        dx: float = 2.0,
    ) -> None:
        super().__init__()
        self.lambda_temp = lambda_temp
        self.lambda_wind = lambda_wind
        self.lambda_div = lambda_div
        self.lambda_noslip = lambda_noslip
        self.lambda_diffusion = lambda_diffusion
        self.dx = dx

    def forward(
        self,
        pred: torch.Tensor,
        target: torch.Tensor,
        mask_air: torch.Tensor,
        mask_solid: torch.Tensor,
        mask_surface: torch.Tensor,
        alpha_field: torch.Tensor,
    ) -> Dict[str, torch.Tensor]:
        """Compute the composite PINN loss.

        Parameters
        ----------
        pred : torch.Tensor
            Predicted fields ``(B, 4, X, Y, Z)`` — channels: ΔT, vx, vy, vz.
        target : torch.Tensor
            Ground-truth fields, same shape.
        mask_air : torch.Tensor
            Binary mask ``(B, 1, X, Y, Z)`` — 1 where air.
        mask_solid : torch.Tensor
            Binary mask ``(B, 1, X, Y, Z)`` — 1 where solid.
        mask_surface : torch.Tensor
            Binary mask ``(B, 1, X, Y, Z)`` — 1 at surface voxels.
        alpha_field : torch.Tensor
            Thermal diffusivity ``(B, 1, X, Y, Z)`` per voxel.

        Returns
        -------
        dict
            Keys: ``total``, ``temp``, ``wind``, ``div``, ``noslip``, ``diffusion``.
        """
        pred_temp = pred[:, 0:1]
        pred_vx = pred[:, 1:2]
        pred_vy = pred[:, 2:3]
        pred_vz = pred[:, 3:4]

        tgt_temp = target[:, 0:1]
        tgt_vx = target[:, 1:2]
        tgt_vy = target[:, 2:3]
        tgt_vz = target[:, 3:4]

        n_air = mask_air.sum().clamp(min=1)
        n_solid = mask_solid.sum().clamp(min=1)
        n_surface = mask_surface.sum().clamp(min=1)

        # Term 1: MSE(ΔT) on air voxels
        loss_temp = ((pred_temp - tgt_temp) ** 2 * mask_air).sum() / n_air

        # Term 2: MSE(v) on air voxels
        loss_wind = (
            ((pred_vx - tgt_vx) ** 2 + (pred_vy - tgt_vy) ** 2 + (pred_vz - tgt_vz) ** 2)
            * mask_air
        ).sum() / n_air

        # Term 3: ||div(v)||² on air voxels
        div = divergence_3d(pred_vx, pred_vy, pred_vz, self.dx)
        loss_div = (div ** 2 * mask_air).sum() / n_air

        # Term 4: ||v||² on solid voxels (no-slip)
        loss_noslip = (
            (pred_vx ** 2 + pred_vy ** 2 + pred_vz ** 2) * mask_solid
        ).sum() / n_solid

        # Term 5: ||∂T/∂t - α∇²T||² on surface voxels
        # Steady-state approximation: ∂T/∂t ≈ ΔT_pred - ΔT_target
        lap_t = laplacian_3d(pred_temp, self.dx)
        dt_residual = (pred_temp - tgt_temp) - alpha_field * lap_t
        loss_diffusion = (dt_residual ** 2 * mask_surface).sum() / n_surface

        total = (
            self.lambda_temp * loss_temp
            + self.lambda_wind * loss_wind
            + self.lambda_div * loss_div
            + self.lambda_noslip * loss_noslip
            + self.lambda_diffusion * loss_diffusion
        )

        return {
            "total": total,
            "temp": loss_temp,
            "wind": loss_wind,
            "div": loss_div,
            "noslip": loss_noslip,
            "diffusion": loss_diffusion,
        }
