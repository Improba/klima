/** Aligné avec `back/src/routes/simulate.rs` (mock) et le post-processeur ONNX. */
export const OVERLAY_ORIGIN_LON = 2.34
export const OVERLAY_ORIGIN_LAT = 48.85
export const OVERLAY_CELL_DEG = 0.00002 // ~2 m vers Paris

export function gridToGeo(gridX: number, gridY: number): { lon: number; lat: number } {
  return {
    lon: OVERLAY_ORIGIN_LON + gridX * OVERLAY_CELL_DEG,
    lat: OVERLAY_ORIGIN_LAT + gridY * OVERLAY_CELL_DEG,
  }
}
