"""Rasterise GeoJSON polygons onto a 3D voxel grid.

Converts vector geometry (e.g. from interactive map edits) into voxel
updates that can be applied to the input tensor before re-running inference.
"""

from __future__ import annotations

from typing import Any, Dict, List, Optional, Tuple

import numpy as np

try:
    from rasterio.features import rasterize
    from shapely.geometry import shape
except ImportError:
    rasterize = None  # type: ignore[assignment]
    shape = None  # type: ignore[assignment]


VoxelUpdate = Tuple[int, int, int, str, Dict[str, float]]


def geojson_to_voxel_update(
    geojson_polygon: Dict[str, Any],
    grid_shape: Tuple[int, int, int],
    transform: Any,
    new_surface_type: str,
    physical_properties: Dict[str, float],
    height_m: Optional[float] = None,
    dz: float = 2.0,
) -> List[VoxelUpdate]:
    """Rasterize a GeoJSON polygon onto the voxel grid.

    Parameters
    ----------
    geojson_polygon : dict
        GeoJSON geometry (Polygon or MultiPolygon).
    grid_shape : tuple of int
        ``(nx, ny, nz)`` of the target voxel grid.
    transform : rasterio.transform.Affine
        Affine transform mapping pixel (col, row) to CRS coordinates.
    new_surface_type : str
        Surface type label (e.g. ``"herbe"``, ``"bitume"``).
    physical_properties : dict
        Must contain ``"albedo"``, ``"emissivity"``, ``"roughness_z0"``.
    height_m : float, optional
        Height of the solid geometry in metres. If ``None``, only the
        ground layer (k=0) is updated.
    dz : float
        Vertical resolution in metres.

    Returns
    -------
    list of (i, j, k, surface_type, properties)
        Each element describes one voxel update.
    """
    if rasterize is None or shape is None:
        raise ImportError(
            "rasterio and shapely are required for GeoJSON rasterisation. "
            "Install them with: pip install rasterio shapely"
        )

    nx, ny, nz = grid_shape

    geom = shape(geojson_polygon)
    mask_2d = rasterize(
        [(geom, 1)],
        out_shape=(ny, nx),
        transform=transform,
        fill=0,
        dtype=np.uint8,
    )

    k_max = 0
    if height_m is not None and height_m > 0:
        k_max = min(int(np.ceil(height_m / dz)) - 1, nz - 1)

    updates: List[VoxelUpdate] = []
    rows, cols = np.where(mask_2d == 1)
    for row, col in zip(rows, cols):
        i = int(col)
        j = int(row)
        if 0 <= i < nx and 0 <= j < ny:
            for k in range(k_max + 1):
                updates.append((i, j, k, new_surface_type, dict(physical_properties)))

    return updates
