/**
 * Fallback 3D “buildings” without Cesium Ion: OSM ways tagged building=* via Overpass,
 * shown as extruded polygons. Quality is lower than Cesium OSM Buildings (Ion tileset).
 *
 * Nécessite un proxy dev `/overpass` → https://overpass-api.de/api/interpreter (à ajouter dans
 * quasar.config si vous branchez ce composable).
 */
import {
  Cartesian3,
  Color,
  Math as CesiumMath,
  PolygonHierarchy,
  Viewer,
  type Entity,
  type Rectangle,
} from 'cesium'

const MAX_SPAN_DEG = 0.028
const MIN_SPAN_DEG = 0.0004
const DEBOUNCE_MS = 950
const MAX_WAYS = 220
const FETCH_TIMEOUT_MS = 28_000

interface OverpassWay {
  type: string
  id: number
  tags?: Record<string, string>
  geometry?: { lat: number; lon: number }[]
}

interface OverpassResponse {
  elements?: OverpassWay[]
}

function parseBuildingHeight(tags?: Record<string, string>): number {
  if (!tags) return 12
  const raw = tags.height ?? tags['building:height']
  if (raw) {
    const t = raw.trim()
    const m = /^([\d.]+)\s*m?$/i.exec(t)
    if (m) {
      const v = parseFloat(m[1])
      if (!Number.isNaN(v)) return clampHeight(v)
    }
    const n = parseFloat(t.replace(',', '.'))
    if (!Number.isNaN(n)) return clampHeight(n)
  }
  const lv = tags['building:levels']
  if (lv) {
    const n = parseInt(lv, 10)
    if (!Number.isNaN(n)) return clampHeight(n * 3.5)
  }
  return 12
}

function clampHeight(m: number): number {
  return Math.min(120, Math.max(4, m))
}

function rectangleToBboxDeg(rect: Rectangle): {
  south: number
  west: number
  north: number
  east: number
} {
  return {
    south: CesiumMath.toDegrees(rect.south),
    west: CesiumMath.toDegrees(rect.west),
    north: CesiumMath.toDegrees(rect.north),
    east: CesiumMath.toDegrees(rect.east),
  }
}

function normalizeBbox(b: {
  south: number
  west: number
  north: number
  east: number
}): { south: number; west: number; north: number; east: number } | null {
  let { south, west, north, east } = b
  if (north < south) [south, north] = [north, south]
  if (east < west) {
    return null
  }
  const latSpan = north - south
  const lonSpan = east - west
  if (latSpan < MIN_SPAN_DEG || lonSpan < MIN_SPAN_DEG) return null
  if (latSpan > MAX_SPAN_DEG || lonSpan > MAX_SPAN_DEG) {
    const cLat = (south + north) / 2
    const cLon = (west + east) / 2
    const half = MAX_SPAN_DEG / 2
    south = cLat - half
    north = cLat + half
    west = cLon - half
    east = cLon + half
  }
  return { south, west, north, east }
}

function buildOverpassQuery(b: { south: number; west: number; north: number; east: number }): string {
  return `[out:json][timeout:25];
(
  way["building"](${b.south},${b.west},${b.north},${b.east});
);
out geom;`
}

/**
 * Registers a debounced camera listener; returns dispose().
 */
export function attachExtrudedOsmBuildings(viewer: Viewer): () => void {
  const entities: Entity[] = []
  let timer: ReturnType<typeof setTimeout> | null = null
  let seq = 0
  let aborted = false

  function clearBuildings() {
    for (const e of entities) {
      viewer.entities.remove(e)
    }
    entities.length = 0
  }

  async function fetchAndDraw() {
    const rect = viewer.camera.computeViewRectangle(viewer.scene.globe.ellipsoid)
    if (!rect) return

    const bbox = normalizeBbox(rectangleToBboxDeg(rect))
    if (!bbox) return

    const my = ++seq
    const body = new URLSearchParams()
    body.set('data', buildOverpassQuery(bbox))

    const ctrl = new AbortController()
    const to = setTimeout(() => ctrl.abort(), FETCH_TIMEOUT_MS)

    try {
      const res = await fetch('/overpass', {
        method: 'POST',
        headers: { 'Content-Type': 'application/x-www-form-urlencoded' },
        body: body.toString(),
        signal: ctrl.signal,
      })
      clearTimeout(to)
      if (my !== seq || aborted) return
      if (!res.ok) {
        console.warn('Overpass buildings: HTTP', res.status)
        return
      }
      const json = (await res.json()) as OverpassResponse
      const els = json.elements?.filter((e) => e.type === 'way' && e.geometry?.length) ?? []
      clearBuildings()
      const slice = els.slice(0, MAX_WAYS)
      for (const w of slice) {
        const g = w.geometry!
        if (g.length < 3) continue
        const positions = g.map((p) => Cartesian3.fromDegrees(p.lon, p.lat, 0))
        const h = parseBuildingHeight(w.tags)
        const e = viewer.entities.add({
          polygon: {
            hierarchy: new PolygonHierarchy(positions),
            height: 0,
            extrudedHeight: h,
            fill: true,
            material: Color.fromCssColorString('#6b7a8f').withAlpha(0.42),
            outline: true,
            outlineColor: Color.fromCssColorString('#2c3540').withAlpha(0.55),
          },
        })
        entities.push(e)
      }
      if (els.length > MAX_WAYS) {
        console.info(
          `Overpass buildings: showing ${MAX_WAYS}/${els.length} in view (zoom in for detail)`,
        )
      }
    } catch (e) {
      clearTimeout(to)
      if (aborted) return
      if ((e as Error).name === 'AbortError') {
        console.warn('Overpass buildings: request timed out')
        return
      }
      console.warn('Overpass buildings unavailable:', e)
    }
  }

  function schedule() {
    if (timer) clearTimeout(timer)
    timer = setTimeout(() => {
      timer = null
      void fetchAndDraw()
    }, DEBOUNCE_MS)
  }

  const removeListener = viewer.camera.moveEnd.addEventListener(schedule)
  schedule()

  return () => {
    aborted = true
    if (timer) clearTimeout(timer)
    removeListener()
    clearBuildings()
  }
}
