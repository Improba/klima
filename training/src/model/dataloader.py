"""HDF5 dataset and data augmentation for CFD simulation pairs.

Expected HDF5 layout::

    /train/0/input   — (15, nx, ny, nz) float32
    /train/0/output  — (4, nx, ny, nz) float32
    /train/0/mask_air     — (1, nx, ny, nz) uint8
    /train/0/mask_solid   — (1, nx, ny, nz) uint8
    /train/0/mask_surface — (1, nx, ny, nz) uint8
    /train/0/alpha_field  — (1, nx, ny, nz) float32
    /train/1/...
    /val/0/...
"""

from __future__ import annotations

import random
from typing import Dict, Optional, Tuple

import h5py
import numpy as np
import torch
from torch.utils.data import Dataset


class CFDDataset(Dataset):
    """Load HDF5 dataset of CFD simulation pairs.

    Parameters
    ----------
    hdf5_path : str
        Path to the HDF5 file.
    split : str
        One of ``"train"`` or ``"val"``.
    normalize : bool
        Whether to apply per-channel z-score normalisation.
    augment : bool
        Whether to apply random rotations and flips.
    """

    def __init__(
        self,
        hdf5_path: str,
        split: str = "train",
        normalize: bool = True,
        augment: bool = True,
    ) -> None:
        super().__init__()
        self.hdf5_path = hdf5_path
        self.split = split
        self.normalize = normalize
        self.augment = augment and (split == "train")

        with h5py.File(hdf5_path, "r") as f:
            self.length = len(f[split])
            if normalize:
                self.input_mean = torch.from_numpy(np.array(f.attrs.get("input_mean", np.zeros(15, dtype=np.float32))))
                self.input_std = torch.from_numpy(np.array(f.attrs.get("input_std", np.ones(15, dtype=np.float32))))
                self.output_mean = torch.from_numpy(np.array(f.attrs.get("output_mean", np.zeros(4, dtype=np.float32))))
                self.output_std = torch.from_numpy(np.array(f.attrs.get("output_std", np.ones(4, dtype=np.float32))))
            else:
                self.input_mean = torch.zeros(15)
                self.input_std = torch.ones(15)
                self.output_mean = torch.zeros(4)
                self.output_std = torch.ones(4)

    def __len__(self) -> int:
        return self.length

    def __getitem__(self, idx: int) -> Dict[str, torch.Tensor]:
        with h5py.File(self.hdf5_path, "r") as f:
            grp = f[f"{self.split}/{idx}"]
            inp = torch.from_numpy(np.array(grp["input"], dtype=np.float32))
            out = torch.from_numpy(np.array(grp["output"], dtype=np.float32))
            mask_air = torch.from_numpy(np.array(grp["mask_air"], dtype=np.float32))
            mask_solid = torch.from_numpy(np.array(grp["mask_solid"], dtype=np.float32))
            mask_surface = torch.from_numpy(np.array(grp["mask_surface"], dtype=np.float32))
            alpha_field = torch.from_numpy(np.array(grp["alpha_field"], dtype=np.float32))

        if self.normalize:
            mu_in = self.input_mean[:, None, None, None]
            std_in = self.input_std[:, None, None, None].clamp(min=1e-8)
            mu_out = self.output_mean[:, None, None, None]
            std_out = self.output_std[:, None, None, None].clamp(min=1e-8)
            inp = (inp - mu_in) / std_in
            out = (out - mu_out) / std_out

        if self.augment:
            k = random.randint(0, 3)
            if k > 0:
                inp, out = self.rotate_sample(inp, out, k)
                mask_air = torch.rot90(mask_air, k, dims=[1, 2])
                mask_solid = torch.rot90(mask_solid, k, dims=[1, 2])
                mask_surface = torch.rot90(mask_surface, k, dims=[1, 2])
                alpha_field = torch.rot90(alpha_field, k, dims=[1, 2])
            if random.random() > 0.5:
                inp, out = self._flip_sample(inp, out)
                mask_air = torch.flip(mask_air, dims=[1])
                mask_solid = torch.flip(mask_solid, dims=[1])
                mask_surface = torch.flip(mask_surface, dims=[1])
                alpha_field = torch.flip(alpha_field, dims=[1])

        return {
            "input": inp,
            "output": out,
            "mask_air": mask_air,
            "mask_solid": mask_solid,
            "mask_surface": mask_surface,
            "alpha_field": alpha_field,
        }

    @staticmethod
    def rotate_sample(
        input_tensor: torch.Tensor,
        output_tensor: torch.Tensor,
        k_rotations: int,
    ) -> Tuple[torch.Tensor, torch.Tensor]:
        """Rotate grid by k*90 degrees and co-rotate wind vectors + direction encoding.

        Rotation is in the XY plane (dims 1, 2 of the C, X, Y, Z tensor).
        Wind velocity channels (output 1=vx, 2=vy) are rotated accordingly.
        Wind direction encoding channels (input 11=sin θ, 12=cos θ) are also rotated.
        """
        inp = torch.rot90(input_tensor, k_rotations, dims=[1, 2])
        out = torch.rot90(output_tensor, k_rotations, dims=[1, 2])

        angle = k_rotations * (np.pi / 2.0)
        cos_a, sin_a = np.cos(angle), np.sin(angle)

        # Rotate wind velocity in output: vx' = cos*vx - sin*vy, vy' = sin*vx + cos*vy
        vx = out[1].clone()
        vy = out[2].clone()
        out[1] = cos_a * vx - sin_a * vy
        out[2] = sin_a * vx + cos_a * vy

        # Rotate wind direction encoding in input (channels 11, 12)
        sin_dir = inp[11].clone()
        cos_dir = inp[12].clone()
        inp[11] = cos_a * sin_dir - sin_a * cos_dir
        inp[12] = sin_a * sin_dir + cos_a * cos_dir

        return inp, out

    @staticmethod
    def _flip_sample(
        input_tensor: torch.Tensor,
        output_tensor: torch.Tensor,
    ) -> Tuple[torch.Tensor, torch.Tensor]:
        """Flip along the X axis and negate vx / adjust wind direction."""
        inp = torch.flip(input_tensor, dims=[1])
        out = torch.flip(output_tensor, dims=[1])

        out[1] = -out[1]  # negate vx
        inp[11] = -inp[11]  # negate sin(θ)

        return inp, out

    def get_norm_params(self) -> Dict[str, torch.Tensor]:
        """Return normalisation statistics."""
        return {
            "input_mean": self.input_mean,
            "input_std": self.input_std,
            "output_mean": self.output_mean,
            "output_std": self.output_std,
        }
