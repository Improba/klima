"""Input tensor encoding for the Klima FNO.

Assembles the 15-channel input tensor from geometry, surface properties,
and meteorological boundary conditions.
"""

from __future__ import annotations

import math
import torch
import numpy as np
from typing import Dict, Tuple


SURFACE_TYPES = ("bitume", "herbe", "eau", "gravier", "vegetation", "batiment")


def encode_input(
    geometry_grid: np.ndarray,
    surface_types: np.ndarray,
    physical_props: Dict[str, np.ndarray],
    wind_speed: float,
    wind_dir: float,
    sun_elevation: float,
    t_ambient: float,
) -> torch.Tensor:
    """Build the 15-channel input tensor from scene components.

    Channel layout
    --------------
    0       : occupancy (binary — 1 = solid, 0 = air)
    1–6     : surface type one-hot (bitume, herbe, eau, gravier, vegetation, batiment)
    7       : albedo
    8       : emissivity
    9       : roughness z0
    10      : wind speed (broadcast)
    11      : sin(wind_dir) (broadcast)
    12      : cos(wind_dir) (broadcast)
    13      : sun elevation (broadcast)
    14      : ambient temperature (broadcast)

    Parameters
    ----------
    geometry_grid : np.ndarray
        Binary occupancy grid of shape ``(nx, ny, nz)`` — 1 = solid.
    surface_types : np.ndarray
        Integer grid ``(nx, ny, nz)`` with values in ``[0, 5]`` indexing
        into :data:`SURFACE_TYPES`. Ignored where occupancy == 0.
    physical_props : dict
        Must contain keys ``"albedo"``, ``"emissivity"``, ``"roughness_z0"``,
        each an ``(nx, ny, nz)`` float array.
    wind_speed : float
        Reference wind speed (m/s).
    wind_dir : float
        Wind direction in degrees (meteorological convention, 0 = N).
    sun_elevation : float
        Solar elevation angle in degrees.
    t_ambient : float
        Ambient temperature (°C or K — must match training data convention).

    Returns
    -------
    torch.Tensor
        Shape ``(1, 15, nx, ny, nz)``, dtype float32.
    """
    nx, ny, nz = geometry_grid.shape
    channels = np.zeros((15, nx, ny, nz), dtype=np.float32)

    # Channel 0: occupancy
    channels[0] = geometry_grid.astype(np.float32)

    # Channels 1-6: surface type one-hot
    for idx in range(len(SURFACE_TYPES)):
        channels[1 + idx] = (surface_types == idx).astype(np.float32)

    # Channels 7-9: physical properties
    channels[7] = physical_props["albedo"].astype(np.float32)
    channels[8] = physical_props["emissivity"].astype(np.float32)
    channels[9] = physical_props["roughness_z0"].astype(np.float32)

    # Channels 10-14: broadcast scalar meteorological conditions
    wind_dir_rad = math.radians(wind_dir)
    channels[10] = wind_speed
    channels[11] = math.sin(wind_dir_rad)
    channels[12] = math.cos(wind_dir_rad)
    channels[13] = sun_elevation
    channels[14] = t_ambient

    tensor = torch.from_numpy(channels).unsqueeze(0)  # (1, 15, nx, ny, nz)
    return tensor
