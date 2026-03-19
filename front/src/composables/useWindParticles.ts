import {
  Viewer,
  Cartesian3,
  Color,
  PointPrimitiveCollection,
  Math as CesiumMath,
} from 'cesium'
import type { WindFieldSample } from 'src/types'

interface Particle {
  position: Cartesian3
  velocity: [number, number, number]
  age: number
  maxAge: number
}

const PARTICLE_COUNT = 800
const BASE_MAX_AGE = 120
const SPEED_SCALE = 0.00001

export function useWindParticles() {
  let animationFrame: number | null = null
  let particles: Particle[] = []
  let pointCollection: PointPrimitiveCollection | null = null

  function interpolateWind(
    x: number,
    y: number,
    z: number,
    windField: WindFieldSample[],
  ): [number, number, number] {
    if (windField.length === 0) return [0, 0, 0]

    let totalWeight = 0
    let wx = 0
    let wy = 0
    let wz = 0

    const k = Math.min(windField.length, 4)
    const sorted = windField
      .map((s) => ({
        sample: s,
        dist: Math.sqrt((s.x - x) ** 2 + (s.y - y) ** 2 + (s.z - z) ** 2),
      }))
      .sort((a, b) => a.dist - b.dist)
      .slice(0, k)

    for (const { sample, dist } of sorted) {
      const w = 1 / (dist + 0.0001)
      wx += sample.vx * w
      wy += sample.vy * w
      wz += sample.vz * w
      totalWeight += w
    }

    return [wx / totalWeight, wy / totalWeight, wz / totalWeight]
  }

  function spawnParticle(windField: WindFieldSample[]): Particle {
    const sample = windField[Math.floor(Math.random() * windField.length)]
    const jitter = () => (Math.random() - 0.5) * 0.001
    return {
      position: Cartesian3.fromDegrees(
        sample.x + jitter(),
        sample.y + jitter(),
        sample.z + Math.random() * 50,
      ),
      velocity: [sample.vx, sample.vy, sample.vz],
      age: 0,
      maxAge: BASE_MAX_AGE + Math.random() * 60,
    }
  }

  function startAnimation(viewer: Viewer, windField: WindFieldSample[]) {
    stopAnimation(viewer)

    if (windField.length === 0) return

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

        const ellipsoid = viewer.scene.globe.ellipsoid
        const cartographic = ellipsoid.cartesianToCartographic(p.position)
        const lonDeg = CesiumMath.toDegrees(cartographic.longitude)
        const latDeg = CesiumMath.toDegrees(cartographic.latitude)
        const alt = cartographic.height

        const [vx, vy, vz] = interpolateWind(lonDeg, latDeg, alt, windField)
        p.velocity = [vx, vy, vz]

        const newLon = lonDeg + vx * SPEED_SCALE
        const newLat = latDeg + vy * SPEED_SCALE
        const newAlt = Math.max(0, alt + vz * SPEED_SCALE * 100)

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
    interpolateWind,
  }
}
