<template>
  <div class="viewer-root">
    <div ref="canvasArea" class="canvas-area">
      <div ref="cesiumContainer" class="canvas-element cesium-container" />
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, onBeforeUnmount, watch } from 'vue'
import {
  Viewer,
  Cartesian2,
  Cartesian3,
  Color,
  createOsmBuildingsAsync,
  type Entity,
  Math as CesiumMath,
  defined,
  Rectangle,
  ScreenSpaceEventHandler,
  ScreenSpaceEventType,
  UrlTemplateImageryProvider,
} from 'cesium'
import type { GeometryBlock, SimulationResult } from 'src/types'
import { useThermalOverlay } from 'src/composables/useThermalOverlay'
import { useWindParticles } from 'src/composables/useWindParticles'
import { gridToGeo } from 'src/utils/overlayGrid'

const DEFAULT_LON = 2.3522
const DEFAULT_LAT = 48.8566
const DEFAULT_ALT = 1500

const props = defineProps<{
  windSpeed: number
  windDirection: number
  sunElevation: number
  simulationResult?: SimulationResult | null
  /** Points dessinés (WGS84) — centrage caméra au 1er clic et au chargement. */
  geometry?: GeometryBlock[]
  activeTool: string
  activeSurfaceType: string
}>()

function geoPointsFromSimulation(result: SimulationResult): { lon: number; lat: number }[] {
  const pts: { lon: number; lat: number }[] = []
  for (const t of result.surface_temperatures) {
    const g = gridToGeo(t.lon, t.lat)
    pts.push({ lon: g.lon, lat: g.lat })
  }
  if (pts.length === 0) {
    for (const w of result.wind_field) {
      const g = gridToGeo(w.x, w.y)
      pts.push({ lon: g.lon, lat: g.lat })
    }
  }
  return pts
}

function geoPointsFromGeometry(blocks: GeometryBlock[]): { lon: number; lat: number }[] {
  return blocks.map((b) => ({ lon: b.x, lat: b.y }))
}

function surfaceTypeColor(surfaceType: string): Color {
  switch (surfaceType) {
    case 'herbe':
      return Color.fromCssColorString('#2ecc71')
    case 'bitume':
      return Color.fromCssColorString('#95a5a6')
    case 'eau':
      return Color.fromCssColorString('#3498db')
    case 'gravier':
      return Color.fromCssColorString('#b7950b')
    case 'vegetation':
      return Color.fromCssColorString('#1e8449')
    case 'batiment':
      return Color.fromCssColorString('#e67e22')
    default:
      return Color.CYAN
  }
}

let sketchEntities: Entity[] = []

function syncGeometryMarkers(viewer: Viewer, blocks: GeometryBlock[] | undefined) {
  for (const e of sketchEntities) {
    viewer.entities.remove(e)
  }
  sketchEntities = []
  if (!blocks?.length) return

  for (const b of blocks) {
    const c = surfaceTypeColor(b.surface_type)
    const entity = viewer.entities.add({
      position: Cartesian3.fromDegrees(b.x, b.y, b.z),
      point: {
        pixelSize: 14,
        color: c.withAlpha(0.95),
        outlineColor: Color.BLACK.withAlpha(0.55),
        outlineWidth: 2,
        disableDepthTestDistance: Number.POSITIVE_INFINITY,
      },
    })
    sketchEntities.push(entity)
  }
}

function focusCameraOnPoints(
  v: Viewer,
  points: { lon: number; lat: number }[],
  opts?: { duration?: number },
) {
  if (points.length === 0) return

  let minLon = Infinity
  let maxLon = -Infinity
  let minLat = Infinity
  let maxLat = -Infinity
  for (const p of points) {
    minLon = Math.min(minLon, p.lon)
    maxLon = Math.max(maxLon, p.lon)
    minLat = Math.min(minLat, p.lat)
    maxLat = Math.max(maxLat, p.lat)
  }

  const spanLon = maxLon - minLon
  const spanLat = maxLat - minLat
  /** Marge minimale (~140 m) + fraction de l’emprise — dézoom confortable. */
  const minPad = 0.00125
  const lonPad = Math.max(minPad, spanLon * 0.55)
  /** Plus de marge au sud qu’au nord : avec un pitch oblique, la zone d’intérêt paraissait trop bas dans le cadre. */
  const latPadNorth = Math.max(minPad * 0.85, spanLat * 0.42)
  const latPadSouth = Math.max(minPad * 1.35, spanLat * 0.62)

  const west = minLon - lonPad
  const east = maxLon + lonPad
  const south = minLat - latPadSouth
  const north = maxLat + latPadNorth

  const rect = Rectangle.fromDegrees(west, south, east, north)
  const duration = opts?.duration ?? 1.35

  if (duration <= 0) {
    v.camera.setView({ destination: rect })
  } else {
    void v.camera.flyTo({ destination: rect, duration })
  }
}

const emit = defineEmits<{
  'map-click': [payload: { lon: number; lat: number; alt: number }]
}>()

const cesiumContainer = ref<HTMLElement>()
const canvasArea = ref<HTMLElement>()
let viewer: Viewer | null = null
let handler: ScreenSpaceEventHandler | null = null
let resizeObserver: ResizeObserver | null = null

const { applyOverlay, clearOverlay } = useThermalOverlay()
const { startAnimation, stopAnimation } = useWindParticles()

onMounted(async () => {
  if (!cesiumContainer.value) return

  viewer = new Viewer(cesiumContainer.value, {
    animation: false,
    timeline: false,
    baseLayerPicker: false,
    geocoder: false,
    homeButton: false,
    sceneModePicker: false,
    navigationHelpButton: false,
    fullscreenButton: false,
    vrButton: false,
    infoBox: false,
    selectionIndicator: false,
    imageryProvider: false,
    skyBox: false,
    contextOptions: {
      webgl: {
        alpha: true,
      },
    },
  })

  viewer.scene.backgroundColor = Color.BLACK
  viewer.scene.globe.baseColor = Color.BLACK
  // Sinon les PointPrimitive mis à jour par requestAnimationFrame ne se redessinent pas (vue figée).
  viewer.scene.requestRenderMode = false
  if (viewer.scene.sun) viewer.scene.sun.show = false
  if (viewer.scene.moon) viewer.scene.moon.show = false
  if (viewer.scene.skyAtmosphere) viewer.scene.skyAtmosphere.show = false

  viewer.scene.globe.enableLighting = props.sunElevation > 0

  try {
    const osm = new UrlTemplateImageryProvider({
      url: 'https://tile.openstreetmap.org/{z}/{x}/{y}.png',
      credit: 'Map tiles by OpenStreetMap, under ODbL. Data by OpenStreetMap, under ODbL.',
    })
    const baseLayer = viewer.imageryLayers.addImageryProvider(osm)
    baseLayer.brightness = 0.28
    baseLayer.contrast = 1.15
    baseLayer.saturation = 0.12
    baseLayer.gamma = 0.85
    baseLayer.alpha = 0.85
  } catch (e) {
    console.warn('Failed to load OSM imagery', e)
  }

  try {
    const osmBuildings = await createOsmBuildingsAsync()
    viewer.scene.primitives.add(osmBuildings)
  } catch (err) {
    console.warn('OSM Buildings unavailable:', err)
  }

  if (props.simulationResult) {
    focusCameraOnPoints(viewer, geoPointsFromSimulation(props.simulationResult), { duration: 0 })
    applyOverlay(viewer, props.simulationResult.surface_temperatures)
    startAnimation(viewer, props.simulationResult.wind_field)
  } else if (props.geometry && props.geometry.length > 0) {
    focusCameraOnPoints(viewer, geoPointsFromGeometry(props.geometry), { duration: 0 })
  } else {
    viewer.camera.setView({
      destination: Cartesian3.fromDegrees(DEFAULT_LON, DEFAULT_LAT, DEFAULT_ALT),
      orientation: {
        heading: CesiumMath.toRadians(0),
        pitch: CesiumMath.toRadians(-45),
        roll: 0,
      },
    })
  }

  syncGeometryMarkers(viewer, props.geometry)

  setupClickHandler()

  if (canvasArea.value) {
    resizeObserver = new ResizeObserver(() => {
      viewer?.resize()
      viewer?.scene.requestRender()
    })
    resizeObserver.observe(canvasArea.value)
  }
})

function setupClickHandler() {
  if (!viewer) return

  handler = new ScreenSpaceEventHandler(viewer.scene.canvas)
  handler.setInputAction((event: { position: Cartesian2 }) => {
    if (!viewer) return

    const cartesian = viewer.scene.pickPosition(event.position)
    if (defined(cartesian)) {
      const cartographic = viewer.scene.globe.ellipsoid.cartesianToCartographic(cartesian)
      const lon = CesiumMath.toDegrees(cartographic.longitude)
      const lat = CesiumMath.toDegrees(cartographic.latitude)
      const alt = cartographic.height

      if (props.activeTool === 'brush') {
        emit('map-click', { lon, lat, alt })
      }
    }
  }, ScreenSpaceEventType.LEFT_CLICK)
}

watch(
  () => props.sunElevation,
  (elevation) => {
    if (!viewer) return
    viewer.scene.globe.enableLighting = elevation > 0
  },
)

watch(
  () => props.simulationResult,
  (result) => {
    if (!viewer) return
    if (result) {
      applyOverlay(viewer, result.surface_temperatures)
      startAnimation(viewer, result.wind_field)
      focusCameraOnPoints(viewer, geoPointsFromSimulation(result), { duration: 1.35 })
    } else {
      clearOverlay(viewer)
      stopAnimation(viewer)
    }
  },
)

watch(
  () => props.geometry,
  (blocks, prevBlocks) => {
    if (!viewer) return
    syncGeometryMarkers(viewer, blocks)
    const len = blocks?.length ?? 0
    const prevLen = prevBlocks?.length ?? 0
    if (len === 1 && prevLen === 0 && blocks?.length) {
      focusCameraOnPoints(viewer, geoPointsFromGeometry(blocks), { duration: 1.2 })
    }
  },
  { deep: true },
)

function getViewer(): Viewer | null {
  return viewer
}

defineExpose({ getViewer })

onBeforeUnmount(() => {
  resizeObserver?.disconnect()
  resizeObserver = null
  stopAnimation(viewer ?? undefined)
  if (viewer) {
    clearOverlay(viewer)
    syncGeometryMarkers(viewer, undefined)
  }
  handler?.destroy()
  handler = null
  viewer?.destroy()
  viewer = null
})
</script>

<style scoped>
.viewer-root {
  position: absolute;
  inset: 0;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

.canvas-area {
  flex: 1;
  position: relative;
  min-height: 0;
  overflow: hidden;
}

.canvas-element {
  position: absolute;
  inset: 0;
  overflow: hidden;
}
</style>
