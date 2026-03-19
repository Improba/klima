import {
  Viewer,
  Cartesian3,
  Color,
  PointPrimitiveCollection,
  Math as CesiumMath,
} from 'cesium'
import type { WindFieldSample } from 'src/types'
import {
  gridToGeo,
  OVERLAY_CELL_DEG,
  OVERLAY_ORIGIN_LAT,
  OVERLAY_ORIGIN_LON,
} from 'src/utils/overlayGrid'

/** Altitude d'affichage (m) par couche z du tenseur — le backend envoie des indices, pas des mètres. */
const ALT_METERS_PER_GRID_Z = 8

interface Particle {
  position: Cartesian3
  velocity: [number, number, number]
  age: number
  maxAge: number
  /** Indice z du champ vent (comme `WindFieldSample.z`), pour interpoler dans le même espace que l'API. */
  gridZ: number
}

interface WindGrid {
  data: Map<string, WindFieldSample>
  minX: number
  maxX: number
  minY: number
  maxY: number
  minZ: number
  maxZ: number
  stepX: number
  stepY: number
  stepZ: number
}

function buildWindGrid(windField: WindFieldSample[]): WindGrid {
  if (windField.length === 0) {
    return {
      data: new Map(),
      minX: 0, maxX: 0,
      minY: 0, maxY: 0,
      minZ: 0, maxZ: 0,
      stepX: 1, stepY: 1, stepZ: 1,
    }
  }

  let minX = Infinity, maxX = -Infinity
  let minY = Infinity, maxY = -Infinity
  let minZ = Infinity, maxZ = -Infinity

  for (const s of windField) {
    if (s.x < minX) minX = s.x
    if (s.x > maxX) maxX = s.x
    if (s.y < minY) minY = s.y
    if (s.y > maxY) maxY = s.y
    if (s.z < minZ) minZ = s.z
    if (s.z > maxZ) maxZ = s.z
  }

  const xVals = new Set<number>()
  const yVals = new Set<number>()
  const zVals = new Set<number>()
  for (const s of windField) {
    xVals.add(s.x)
    yVals.add(s.y)
    zVals.add(s.z)
  }

  const sortedX = [...xVals].sort((a, b) => a - b)
  const sortedY = [...yVals].sort((a, b) => a - b)
  const sortedZ = [...zVals].sort((a, b) => a - b)

  const stepX = sortedX.length > 1 ? sortedX[1] - sortedX[0] : 1
  const stepY = sortedY.length > 1 ? sortedY[1] - sortedY[0] : 1
  const stepZ = sortedZ.length > 1 ? sortedZ[1] - sortedZ[0] : 1

  const data = new Map<string, WindFieldSample>()
  for (const s of windField) {
    const key = `${s.x},${s.y},${s.z}`
    data.set(key, s)
  }

  return { data, minX, maxX, minY, maxY, minZ, maxZ, stepX, stepY, stepZ }
}

function gridKey(x: number, y: number, z: number): string {
  return `${x},${y},${z}`
}

function clamp(n: number, lo: number, hi: number): number {
  return Math.min(hi, Math.max(lo, n))
}

function interpolateWindFast(
  gx: number,
  gy: number,
  gz: number,
  grid: WindGrid,
): [number, number, number] {
  if (grid.data.size === 0) return [0, 0, 0]

  const sx = grid.stepX
  const sy = grid.stepY
  const sz = grid.stepZ

  const ggx = clamp(gx, grid.minX, grid.maxX)
  const ggy = clamp(gy, grid.minY, grid.maxY)
  const ggz = clamp(gz, grid.minZ, grid.maxZ)

  const ix = Math.floor((ggx - grid.minX) / sx)
  const iy = Math.floor((ggy - grid.minY) / sy)
  const iz = Math.floor((ggz - grid.minZ) / sz)

  const x0 = grid.minX + ix * sx
  const y0 = grid.minY + iy * sy
  const z0 = grid.minZ + iz * sz

  const x1 = x0 + sx
  const y1 = y0 + sy
  const z1 = z0 + sz

  const fx = sx > 0 ? Math.max(0, Math.min(1, (ggx - x0) / sx)) : 0
  const fy = sy > 0 ? Math.max(0, Math.min(1, (ggy - y0) / sy)) : 0
  const fz = sz > 0 ? Math.max(0, Math.min(1, (ggz - z0) / sz)) : 0

  let totalWeight = 0
  let wx = 0, wy = 0, wz = 0

  const corners: [number, number, number, number][] = [
    [x0, y0, z0, (1 - fx) * (1 - fy) * (1 - fz)],
    [x1, y0, z0, fx * (1 - fy) * (1 - fz)],
    [x0, y1, z0, (1 - fx) * fy * (1 - fz)],
    [x1, y1, z0, fx * fy * (1 - fz)],
    [x0, y0, z1, (1 - fx) * (1 - fy) * fz],
    [x1, y0, z1, fx * (1 - fy) * fz],
    [x0, y1, z1, (1 - fx) * fy * fz],
    [x1, y1, z1, fx * fy * fz],
  ]

  for (const [cx, cy, cz, w] of corners) {
    if (w < 1e-10) continue
    const sample = grid.data.get(gridKey(cx, cy, cz))
    if (sample) {
      wx += sample.vx * w
      wy += sample.vy * w
      wz += sample.vz * w
      totalWeight += w
    }
  }

  if (totalWeight === 0) {
    const nearest = grid.data.get(gridKey(x0, y0, z0))
    if (nearest) return [nearest.vx, nearest.vy, nearest.vz]
    return [0, 0, 0]
  }

  return [wx / totalWeight, wy / totalWeight, wz / totalWeight]
}

const PARTICLE_COUNT = 800
const BASE_MAX_AGE = 120
const SPEED_SCALE = 0.00001

export function useWindParticles() {
  let animationFrame: number | null = null
  let particles: Particle[] = []
  let pointCollection: PointPrimitiveCollection | null = null

  function spawnParticle(windField: WindFieldSample[]): Particle {
    const sample = windField[Math.floor(Math.random() * windField.length)]
    /** Jitter en indices grille (~fraction de cellule), pas en ° — sinon on sort du champ vent (mock 3×3). */
    const jx = (Math.random() - 0.5) * 0.45
    const jy = (Math.random() - 0.5) * 0.45
    const geo = gridToGeo(sample.x + jx, sample.y + jy)
    const alt =
      sample.z * ALT_METERS_PER_GRID_Z + (Math.random() - 0.5) * ALT_METERS_PER_GRID_Z
    return {
      position: Cartesian3.fromDegrees(
        geo.lon,
        geo.lat,
        Math.max(0, alt),
      ),
      velocity: [sample.vx, sample.vy, sample.vz],
      age: 0,
      maxAge: BASE_MAX_AGE + Math.random() * 60,
      gridZ: sample.z,
    }
  }

  function geoToGrid(lonDeg: number, latDeg: number): { gx: number; gy: number } {
    return {
      gx: (lonDeg - OVERLAY_ORIGIN_LON) / OVERLAY_CELL_DEG,
      gy: (latDeg - OVERLAY_ORIGIN_LAT) / OVERLAY_CELL_DEG,
    }
  }

  function startAnimation(viewer: Viewer, windField: WindFieldSample[]) {
    stopAnimation(viewer)

    if (windField.length === 0) return

    const grid = buildWindGrid(windField)

    pointCollection = new PointPrimitiveCollection()
    viewer.scene.primitives.add(pointCollection)

    particles = []
    for (let i = 0; i < PARTICLE_COUNT; i++) {
      const p = spawnParticle(windField)
      particles.push(p)
      pointCollection.add({
        position: p.position,
        pixelSize: 3,
        color: Color.WHITE.withAlpha(0.7),
      })
    }

    function animate() {
      if (!pointCollection) return

      const ellipsoid = viewer.scene.globe.ellipsoid

      for (let i = 0; i < particles.length; i++) {
        const p = particles[i]
        p.age++

        if (p.age >= p.maxAge) {
          const newP = spawnParticle(windField)
          particles[i] = newP
          const pp = pointCollection.get(i)
          pp.position = newP.position
          pp.color = Color.WHITE.withAlpha(0.7)
          continue
        }

        const cartographic = ellipsoid.cartesianToCartographic(p.position)
        const lonDeg = CesiumMath.toDegrees(cartographic.longitude)
        const latDeg = CesiumMath.toDegrees(cartographic.latitude)

        const { gx, gy } = geoToGrid(lonDeg, latDeg)
        const gz = p.gridZ

        const [vx, vy, vz] = interpolateWindFast(gx, gy, gz, grid)
        p.velocity = [vx, vy, vz]

        const newLon = lonDeg + vx * SPEED_SCALE
        const newLat = latDeg + vy * SPEED_SCALE
        // Hauteur suivant la couche du modèle (évite alt/2 confondu avec l'indice z).
        const newAlt = Math.max(0, p.gridZ * ALT_METERS_PER_GRID_Z)

        p.position = Cartesian3.fromDegrees(newLon, newLat, newAlt)

        const speed = Math.sqrt(vx * vx + vy * vy + vz * vz)
        const fade = 1 - p.age / p.maxAge
        const alpha = Math.min(0.9, fade * 0.8)
        const hue = Math.max(0, Math.min(1, speed / 15))
        const color = Color.fromHsl(0.55 - hue * 0.55, 0.8, 0.6, alpha)

        const pp = pointCollection.get(i)
        pp.position = p.position
        pp.color = color
        pp.pixelSize = 2 + speed * 0.3
      }

      // Cesium n’affiche pas les mises à jour de primitives tant que la scène n’est pas invalidée
      // (souvent avec requestRenderMode actif).
      viewer.scene.requestRender()

      animationFrame = requestAnimationFrame(animate)
    }

    animationFrame = requestAnimationFrame(animate)
  }

  function stopAnimation(viewer?: Viewer) {
    if (animationFrame !== null) {
      cancelAnimationFrame(animationFrame)
      animationFrame = null
    }
    if (pointCollection && viewer) {
      viewer.scene.primitives.remove(pointCollection)
      pointCollection = null
    }
    particles = []
  }

  return {
    startAnimation,
    stopAnimation,
  }
}
