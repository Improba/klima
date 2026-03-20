"""Synthetic urban microclimate fields with explicit PDE structure.

We build supervision targets from:

1. **Temperature** — solution of Laplace's equation :math:`\\nabla^2 T = 0` in air
   with Dirichlet data on solids and on the domain boundary (isothermal far-field).
   This is the steady conductive limit in a fixed medium (no sources), a standard
   elliptic model for thermal equilibrium outside of strong advection.

2. **Velocity** — a **divergence-free** field :math:`\\mathbf{v} = \\nabla \\times \\mathbf{B}`
   with smoothed Gaussian random :math:`\\mathbf{B}`, plus a horizontal mean wind
   aligned with the meteorological wind direction. Thus :math:`\\nabla\\cdot\\mathbf{v}=0`
   in the continuum limit (discretisation introduces small residuals).

The 15-channel input layout matches ``training.src.model.encoding.encode_input``.
Outputs are :math:`\\Delta T = T - T_\\mathrm{amb}` and :math:`(v_x, v_y, v_z)`.
"""

from __future__ import annotations

import math
from typing import Dict, Tuple

import numpy as np
from scipy.ndimage import gaussian_filter

from training.src.model.encoding import encode_input


def random_building_geometry(
    nx: int,
    ny: int,
    nz: int,
    rng: np.random.Generator,
    n_buildings: Tuple[int, int] = (2, 6),
    max_height_ratio: float = 0.65,
) -> Tuple[np.ndarray, np.ndarray]:
    """Occupancy (1 = solid) and surface-type labels (0..5) on the grid."""
    occ = np.zeros((nx, ny, nz), dtype=np.float32)
    surf = np.zeros((nx, ny, nz), dtype=np.int32)

    n = rng.integers(n_buildings[0], n_buildings[1] + 1)
    z_top = max(2, int(nz * max_height_ratio))

    for _ in range(int(n)):
        w = rng.integers(max(4, nx // 12), max(5, nx // 4))
        d = rng.integers(max(4, ny // 12), max(5, ny // 4))
        h = rng.integers(max(2, nz // 8), z_top)
        x0 = rng.integers(1, max(2, nx - w - 1))
        y0 = rng.integers(1, max(2, ny - d - 1))
        z0 = 0
        occ[x0 : x0 + w, y0 : y0 + d, z0 : z0 + h] = 1.0
        st = int(rng.integers(0, 6))
        surf[x0 : x0 + w, y0 : y0 + d, z0 : z0 + h] = st

    surf = np.where(occ > 0.5, surf, 0)
    return occ, surf


def masks_from_occupancy(occ: np.ndarray) -> Tuple[np.ndarray, np.ndarray, np.ndarray]:
    """Air / solid / surface (air voxel touching solid, 6-neighbourhood)."""
    air = (occ < 0.5).astype(np.float32)
    solid = (occ > 0.5).astype(np.float32)

    rolled = [
        np.roll(occ, 1, 0),
        np.roll(occ, -1, 0),
        np.roll(occ, 1, 1),
        np.roll(occ, -1, 1),
        np.roll(occ, 1, 2),
        np.roll(occ, -1, 2),
    ]
    neigh_solid = sum(r > 0.5 for r in rolled).astype(np.float32)
    surface = ((air > 0.5) & (neigh_solid > 0)).astype(np.float32)
    return air, solid, surface


def jacobi_laplace_air(
    air: np.ndarray,
    T_solid: np.ndarray,
    t_amb: float,
    n_iter: int = 1200,
) -> np.ndarray:
    """Harmonic temperature in air; fixed Dirichlet values in solids and on outer boundary."""
    nx, ny, nz = air.shape
    air_b = air > 0.5
    solid_b = ~air_b

    T = np.full((nx, ny, nz), t_amb, dtype=np.float64)
    T[solid_b] = T_solid[solid_b]

    for _ in range(n_iter):
        pad = np.pad(T, 1, mode="constant", constant_values=t_amb)
        neigh = (
            pad[2:, 1:-1, 1:-1]
            + pad[:-2, 1:-1, 1:-1]
            + pad[1:-1, 2:, 1:-1]
            + pad[1:-1, :-2, 1:-1]
            + pad[1:-1, 1:-1, 2:]
            + pad[1:-1, 1:-1, :-2]
        ) * (1.0 / 6.0)
        T_new = T.copy()
        T_new[air_b] = neigh[air_b]
        T_new[solid_b] = T_solid[solid_b]
        T = T_new

    return T.astype(np.float32)


def divergence_free_wind(
    nx: int,
    ny: int,
    nz: int,
    dx: float,
    air: np.ndarray,
    wind_speed: float,
    wind_dir_deg: float,
    rng: np.random.Generator,
    smooth_sigma: float = 2.2,
) -> Tuple[np.ndarray, np.ndarray, np.ndarray]:
    """v = curl(B) + U_infty with B smoothed Gaussian noise; v zeroed in solids."""
    Bx = gaussian_filter(rng.standard_normal((nx, ny, nz)), sigma=smooth_sigma)
    By = gaussian_filter(rng.standard_normal((nx, ny, nz)), sigma=smooth_sigma)
    Bz = gaussian_filter(rng.standard_normal((nx, ny, nz)), sigma=smooth_sigma)

    dBz_dy = np.gradient(Bz, dx, axis=1)
    dBy_dz = np.gradient(By, dx, axis=2)
    dBx_dz = np.gradient(Bx, dx, axis=2)
    dBz_dx = np.gradient(Bz, dx, axis=0)
    dBy_dx = np.gradient(By, dx, axis=0)
    dBx_dy = np.gradient(Bx, dx, axis=1)

    vx = dBz_dy - dBy_dz
    vy = dBx_dz - dBz_dx
    vz = dBy_dx - dBx_dy

    air_b = air > 0.5
    th = math.radians(wind_dir_deg)
    ux = float(wind_speed) * math.sin(th)
    uy = float(wind_speed) * math.cos(th)

    vx[air_b] += ux
    vy[air_b] += uy

    m = air > 0.5
    vx = np.where(m, vx, 0.0)
    vy = np.where(m, vy, 0.0)
    vz = np.where(m, vz, 0.0)

    spd = np.sqrt(vx * vx + vy * vy + vz * vz)
    med = float(np.median(spd[m]))
    if med > 1e-8:
        scale = float(wind_speed) / med
        vx *= scale
        vy *= scale
        vz *= scale

    return vx.astype(np.float32), vy.astype(np.float32), vz.astype(np.float32)


def physical_props_random(
    occ: np.ndarray,
    surf: np.ndarray,
    rng: np.random.Generator,
) -> Dict[str, np.ndarray]:
    """Albedo, emissivity, z0: defaults in air; random on solids (surface-type aware offset)."""
    nx, ny, nz = occ.shape
    albedo = np.full((nx, ny, nz), 0.15, dtype=np.float32)
    emissivity = np.full((nx, ny, nz), 0.9, dtype=np.float32)
    roughness = np.full((nx, ny, nz), 0.03, dtype=np.float32)

    m = occ > 0.5
    n = int(np.sum(m))
    if n > 0:
        base_a = rng.uniform(0.12, 0.55, size=n).astype(np.float32)
        st = surf[m].astype(np.int32)
        albedo[m] = np.clip(base_a + 0.04 * st.astype(np.float32), 0.05, 0.85)
        emissivity[m] = rng.uniform(0.85, 0.97, size=n).astype(np.float32)
        roughness[m] = rng.uniform(0.02, 0.95, size=n).astype(np.float32)

    return {"albedo": albedo, "emissivity": emissivity, "roughness_z0": roughness}


def alpha_field(air: np.ndarray, k_air: float = 2.2e-5, k_solid: float = 1.5e-6) -> np.ndarray:
    """Thermal diffusivity scale (m^2/s order); used in PINN diffusion term."""
    a = np.full(air.shape, k_solid, dtype=np.float32)
    a[air > 0.5] = k_air
    return a


def synthesize_sample(
    nx: int,
    ny: int,
    nz: int,
    dx: float,
    rng: np.random.Generator,
) -> Dict[str, np.ndarray]:
    """One full training pair: input (15,C), output (4,C), masks, alpha."""
    occ, surf = random_building_geometry(nx, ny, nz, rng)
    air, solid, surface = masks_from_occupancy(occ)

    t_amb = float(rng.uniform(18.0, 28.0))
    T_solid = np.full((nx, ny, nz), t_amb, dtype=np.float32)
    m = occ > 0.5
    T_solid[m] += rng.normal(0.0, 2.5, size=np.sum(m)).astype(np.float32)
    T_solid[m] += rng.uniform(0.0, 5.0, size=np.sum(m)).astype(np.float32)

    T = jacobi_laplace_air(air, T_solid, t_amb)
    dT = (T - t_amb).astype(np.float32)

    wind_speed = float(rng.uniform(1.5, 12.0))
    wind_dir = float(rng.uniform(0.0, 360.0))
    sun_el = float(rng.uniform(15.0, 75.0))

    vx, vy, vz = divergence_free_wind(nx, ny, nz, dx, air, wind_speed, wind_dir, rng)

    props = physical_props_random(occ, surf, rng)
    inp_t = encode_input(
        occ,
        surf,
        props,
        wind_speed=wind_speed,
        wind_dir=wind_dir,
        sun_elevation=sun_el,
        t_ambient=t_amb,
    )
    inp = inp_t.squeeze(0).cpu().numpy().astype(np.float32)

    out = np.stack([dT, vx, vy, vz], axis=0).astype(np.float32)
    alpha = alpha_field(air)

    return {
        "input": inp,
        "output": out,
        "mask_air": air.reshape(1, nx, ny, nz).astype(np.float32),
        "mask_solid": solid.reshape(1, nx, ny, nz).astype(np.float32),
        "mask_surface": surface.reshape(1, nx, ny, nz).astype(np.float32),
        "alpha_field": alpha.reshape(1, nx, ny, nz).astype(np.float32),
    }
