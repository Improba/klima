import {
  Viewer,
  Color,
  Entity,
  Cartesian3,
  Rectangle,
  Math as CesiumMath,
} from 'cesium'
import type { SurfaceTemperature } from 'src/types'

const ORIGIN_LON = 2.3400
const ORIGIN_LAT = 48.8500
const CELL_SIZE_DEG = 0.00002 // ~2m at Paris latitude

function gridToGeo(gridX: number, gridY: number): { lon: number; lat: number } {
  return {
    lon: ORIGIN_LON + gridX * CELL_SIZE_DEG,
    lat: ORIGIN_LAT + gridY * CELL_SIZE_DEG,
  }
}

export function useThermalOverlay() {
  let entities: Entity[] = []

  function temperatureToColor(temp: number, min: number, max: number): Color {
    const range = max - min
    if (range === 0) return Color.WHITE

    const t = Math.max(0, Math.min(1, (temp - min) / range))

    let r: number, g: number, b: number
    if (t < 0.25) {
      const s = t / 0.25
      r = 0
      g = s
      b = 1
    } else if (t < 0.5) {
      const s = (t - 0.25) / 0.25
      r = 0
      g = 1
      b = 1 - s
    } else if (t < 0.75) {
      const s = (t - 0.5) / 0.25
      r = s
      g = 1
      b = 0
    } else {
      const s = (t - 0.75) / 0.25
      r = 1
      g = 1 - s
      b = 0
    }

    return new Color(r, g, b, 0.6)
  }

  function applyOverlay(viewer: Viewer, temperatures: SurfaceTemperature[]) {
    clearOverlay(viewer)

    if (temperatures.length === 0) return

    let min = Infinity
    let max = -Infinity
    for (const t of temperatures) {
      if (t.temperature < min) min = t.temperature
      if (t.temperature > max) max = t.temperature
    }

    for (const sample of temperatures) {
      const color = temperatureToColor(sample.temperature, min, max)
      const geo = gridToGeo(sample.lon, sample.lat)
      const west = CesiumMath.toRadians(geo.lon - CELL_SIZE_DEG)
      const south = CesiumMath.toRadians(geo.lat - CELL_SIZE_DEG)
      const east = CesiumMath.toRadians(geo.lon + CELL_SIZE_DEG)
      const north = CesiumMath.toRadians(geo.lat + CELL_SIZE_DEG)

      const entity = viewer.entities.add({
        rectangle: {
          coordinates: new Rectangle(west, south, east, north),
          material: color,
          height: sample.alt,
          classificationType: undefined,
        },
      })
      entities.push(entity)
    }
  }

  function clearOverlay(viewer: Viewer) {
    for (const entity of entities) {
      viewer.entities.remove(entity)
    }
    entities = []
  }

  return {
    applyOverlay,
    clearOverlay,
    temperatureToColor,
  }
}
