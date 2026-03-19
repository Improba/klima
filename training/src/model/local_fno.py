"""Local Fourier Neural Operator (FNO) for 3D urban microclimate prediction.

Handles variable-resolution inputs by truncating spectral modes in the
Fourier domain — the core advantage of FNO over grid-fixed architectures.
"""

from __future__ import annotations

import torch
import torch.nn as nn
from typing import Tuple


class SpectralConv3d(nn.Module):
    """Fourier layer: FFT -> learned spectral weights -> IFFT.

    Operates in the spectral domain with a fixed number of modes per axis,
    allowing the layer to accept any spatial resolution at inference time.
    """

    def __init__(
        self,
        in_channels: int,
        out_channels: int,
        modes1: int,
        modes2: int,
        modes3: int,
    ) -> None:
        super().__init__()
        self.in_channels = in_channels
        self.out_channels = out_channels
        self.modes1 = modes1
        self.modes2 = modes2
        self.modes3 = modes3

        scale = 1.0 / (in_channels * out_channels)
        self.weights1 = nn.Parameter(
            scale * torch.randn(in_channels, out_channels, modes1, modes2, modes3, dtype=torch.cfloat)
        )
        self.weights2 = nn.Parameter(
            scale * torch.randn(in_channels, out_channels, modes1, modes2, modes3, dtype=torch.cfloat)
        )
        self.weights3 = nn.Parameter(
            scale * torch.randn(in_channels, out_channels, modes1, modes2, modes3, dtype=torch.cfloat)
        )
        self.weights4 = nn.Parameter(
            scale * torch.randn(in_channels, out_channels, modes1, modes2, modes3, dtype=torch.cfloat)
        )

    @staticmethod
    def _compl_mul3d(a: torch.Tensor, b: torch.Tensor) -> torch.Tensor:
        """Complex multiplication via einsum: (batch, in, x, y, z) x (in, out, x, y, z) -> (batch, out, x, y, z)."""
        return torch.einsum("bixyz,ioxyz->boxyz", a, b)

    def forward(self, x: torch.Tensor) -> torch.Tensor:
        batch_size = x.shape[0]
        nx, ny, nz = x.shape[2], x.shape[3], x.shape[4]

        x_ft = torch.fft.rfftn(x, dim=[-3, -2, -1])

        nz_freq = nz // 2 + 1
        out_ft = torch.zeros(
            batch_size, self.out_channels, nx, ny, nz_freq,
            dtype=torch.cfloat, device=x.device,
        )

        # Clamp modes to actual frequency dimensions
        m1 = min(self.modes1, x_ft.shape[-3] // 2)
        m2 = min(self.modes2, x_ft.shape[-2] // 2)
        m3 = min(self.modes3, x_ft.shape[-1])

        # Truncate weight tensors when modes are clamped
        w1 = self.weights1[:, :, :m1, :m2, :m3]
        w2 = self.weights2[:, :, :m1, :m2, :m3]
        w3 = self.weights3[:, :, :m1, :m2, :m3]
        w4 = self.weights4[:, :, :m1, :m2, :m3]

        # Four quadrants in the (kx, ky) plane (kz is real-FFT so only positive)
        out_ft[:, :, :m1, :m2, :m3] = self._compl_mul3d(
            x_ft[:, :, :m1, :m2, :m3], w1
        )
        out_ft[:, :, -m1:, :m2, :m3] = self._compl_mul3d(
            x_ft[:, :, -m1:, :m2, :m3], w2
        )
        out_ft[:, :, :m1, -m2:, :m3] = self._compl_mul3d(
            x_ft[:, :, :m1, -m2:, :m3], w3
        )
        out_ft[:, :, -m1:, -m2:, :m3] = self._compl_mul3d(
            x_ft[:, :, -m1:, -m2:, :m3], w4
        )

        return torch.fft.irfftn(out_ft, s=(nx, ny, nz), dim=[-3, -2, -1])


class FNOBlock3d(nn.Module):
    """One FNO block: SpectralConv3d + Conv3d(1x1x1) residual + GELU."""

    def __init__(
        self,
        width: int,
        modes1: int,
        modes2: int,
        modes3: int,
    ) -> None:
        super().__init__()
        self.spectral_conv = SpectralConv3d(width, width, modes1, modes2, modes3)
        self.residual_conv = nn.Conv3d(width, width, kernel_size=1)
        self.norm = nn.InstanceNorm3d(width)
        self.activation = nn.GELU()

    def forward(self, x: torch.Tensor) -> torch.Tensor:
        spectral = self.spectral_conv(x)
        residual = self.residual_conv(x)
        out = self.norm(spectral + residual)
        return self.activation(out)


class LocalFNO3d(nn.Module):
    """Full Local-FNO model for urban microclimate prediction.

    Accepts variable-resolution inputs thanks to the spectral convolution
    operating on a fixed (truncated) number of Fourier modes.

    Parameters
    ----------
    in_channels : int
        Number of input channels (default 15).
    out_channels : int
        Number of output channels (default 4: ΔT, vx, vy, vz).
    modes : tuple of int
        Number of Fourier modes per spatial axis.
    width : int
        Hidden channel width throughout FNO blocks.
    num_layers : int
        Number of stacked FNO blocks.
    """

    def __init__(
        self,
        in_channels: int = 15,
        out_channels: int = 4,
        modes: Tuple[int, int, int] = (16, 16, 8),
        width: int = 64,
        num_layers: int = 4,
    ) -> None:
        super().__init__()
        self.in_channels = in_channels
        self.out_channels = out_channels
        self.width = width

        self.lifting = nn.Conv3d(in_channels, width, kernel_size=1)

        self.fno_blocks = nn.ModuleList([
            FNOBlock3d(width, modes[0], modes[1], modes[2])
            for _ in range(num_layers)
        ])

        self.projection = nn.Sequential(
            nn.Conv3d(width, width // 2, kernel_size=1),
            nn.GELU(),
            nn.Conv3d(width // 2, out_channels, kernel_size=1),
        )

    def forward(self, x: torch.Tensor) -> torch.Tensor:
        """Forward pass.

        Parameters
        ----------
        x : torch.Tensor
            Shape ``(batch, in_channels, nx, ny, nz)`` — spatial dims can vary.

        Returns
        -------
        torch.Tensor
            Shape ``(batch, out_channels, nx, ny, nz)``.
        """
        x = self.lifting(x)
        for block in self.fno_blocks:
            x = block(x)
        return self.projection(x)
