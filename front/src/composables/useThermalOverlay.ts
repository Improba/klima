import {
  Viewer,
  Color,
  Entity,
  Rectangle,
  ImageMaterialProperty,
} from 'cesium'
import type { SurfaceTemperature } from 'src/types'
import { gridToGeo, OVERLAY_CELL_DEG as CELL_SIZE_DEG } from 'src/utils/overlayGrid'

/** One grid cell is tiny from the default camera; expand so the heatmap stays visible. */
const MIN_RECT_SPAN_DEG = 0.004 // ~350–450 m near Paris

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
    let minGridX = Infinity
    let maxGridX = -Infinity
    let minGridY = Infinity
    let maxGridY = -Infinity

    for (const t of temperatures) {
      if (t.temperature < min) min = t.temperature
      if (t.temperature > max) max = t.temperature
      if (t.lon < minGridX) minGridX = t.lon
      if (t.lon > maxGridX) maxGridX = t.lon
      if (t.lat < minGridY) minGridY = t.lat
      if (t.lat > maxGridY) maxGridY = t.lat
    }

    const width = maxGridX - minGridX + 1
    const height = maxGridY - minGridY + 1

    const canvas = document.createElement('canvas')
    canvas.width = width
    canvas.height = height
    const ctx = canvas.getContext('2d')!

    ctx.clearRect(0, 0, width, height)

    for (const t of temperatures) {
      const color = temperatureToColor(t.temperature, min, max)
      const r = Math.round(color.red * 255)
      const g = Math.round(color.green * 255)
      const b = Math.round(color.blue * 255)
      ctx.fillStyle = `rgba(${r},${g},${b},0.7)`
      const px = t.lon - minGridX
      const py = maxGridY - t.lat // flip Y so north is up
      ctx.fillRect(px, py, 1, 1)
    }

    const westGeo = gridToGeo(minGridX, minGridY)
    const eastGeo = gridToGeo(maxGridX + 1, maxGridY + 1)

    let west = westGeo.lon - CELL_SIZE_DEG / 2
    let south = westGeo.lat - CELL_SIZE_DEG / 2
    let east = eastGeo.lon + CELL_SIZE_DEG / 2
    let north = eastGeo.lat + CELL_SIZE_DEG / 2

    const lonSpan = east - west
    const latSpan = north - south
    if (lonSpan < MIN_RECT_SPAN_DEG) {
      const mid = (west + east) / 2
      west = mid - MIN_RECT_SPAN_DEG / 2
      east = mid + MIN_RECT_SPAN_DEG / 2
    }
    if (latSpan < MIN_RECT_SPAN_DEG) {
      const mid = (south + north) / 2
      south = mid - MIN_RECT_SPAN_DEG / 2
      north = mid + MIN_RECT_SPAN_DEG / 2
    }

    const entity = viewer.entities.add({
      rectangle: {
        coordinates: Rectangle.fromDegrees(west, south, east, north),
        material: new ImageMaterialProperty({
          image: canvas,
          transparent: true,
        }),
        height: 0,
      },
    })
    entities.push(entity)
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
