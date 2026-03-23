//! Grille d’overlay alignée sur `simulate.rs` (mock), `front/src/utils/overlayGrid.ts`
//! et les viewers thermique / vent.

pub const OVERLAY_ORIGIN_LON: f64 = 2.3400;
pub const OVERLAY_ORIGIN_LAT: f64 = 48.8500;
pub const OVERLAY_CELL_DEG: f64 = 0.00002;
/// ~2 m vertical par couche (cohérent avec `domain.dx` typique à l’entraînement).
pub const Z_METERS_PER_VOXEL: f64 = 2.0;

pub const GRID_NX: usize = 256;
pub const GRID_NY: usize = 256;
pub const GRID_NZ: usize = 64;

/// Indices de grille horizontaux depuis WGS84 (x = lon, y = lat dans l’API).
#[inline]
pub fn grid_ix_iy(lon: f64, lat: f64) -> (usize, usize) {
    let gx = ((lon - OVERLAY_ORIGIN_LON) / OVERLAY_CELL_DEG).round();
    let gy = ((lat - OVERLAY_ORIGIN_LAT) / OVERLAY_CELL_DEG).round();
    let ix = gx.clamp(0.0, (GRID_NX - 1) as f64) as usize;
    let iy = gy.clamp(0.0, (GRID_NY - 1) as f64) as usize;
    (ix, iy)
}

/// Altitude (m, ellipsoïde Cesium) → indice vertical.
#[inline]
pub fn grid_iz(alt_m: f64) -> usize {
    let z = (alt_m.max(0.0) / Z_METERS_PER_VOXEL).round();
    z.clamp(0.0, (GRID_NZ - 1) as f64) as usize
}
