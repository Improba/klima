<template>
  <div class="viewer-root">
    <div ref="canvasArea" class="canvas-area">
      <div ref="cesiumContainer" class="canvas-element cesium-container" />
    </div>
    <div v-if="ionOsmBuildingsHint" class="map-hint text-caption">
      {{ ionOsmBuildingsHint }}
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, onBeforeUnmount, watch } from 'vue'
import {
  Viewer,
  Cartesian2,
  Cartesian3,
  Cesium3DTileset,
  Color,
  createOsmBuildingsAsync,
  createWorldTerrainAsync,
  type Entity,
  Math as CesiumMath,
  defined,
  Rectangle,
  ScreenSpaceEventHandler,
  ScreenSpaceEventType,
  UrlTemplateImageryProvider,
} from 'cesium'
import type { GeometryBlock, SimulationResult, WindParticlesContext } from 'src/types'
import { useThermalOverlay } from 'src/composables/useThermalOverlay'
import { useWindParticles } from 'src/composables/useWindParticles'
import { gridToGeo } from 'src/utils/overlayGrid'
import { useSimulationStore } from 'src/stores/simulation'

defineOptions({ name: 'CesiumViewer' })

function windParticlesContextFromResult(r: SimulationResult): WindParticlesContext {
  return {
    occupancy: r.occupancy ?? undefined,
    gridResolution: r.metadata.grid_resolution,
    zMetersPerVoxel: r.metadata.z_meters_per_voxel,
  }
}

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
/** Message si le tileset Ion « OSM Buildings » n’a pas pu être chargé (jeton / asset 96188). */
const ionOsmBuildingsHint = ref('')
let viewer: Viewer | null = null
let ionOsmTileset: Cesium3DTileset | null = null
let handler: ScreenSpaceEventHandler | null = null
let resizeObserver: ResizeObserver | null = null
/** Rectangle cyan : emprise WGS84 envoyée au backend (Overpass ≠ tileset Ion, mais même zone). */
let inferenceBboxEntity: Entity | undefined

const { applyOverlay, clearOverlay } = useThermalOverlay()
const { startAnimation, stopAnimation } = useWindParticles()
const simStore = useSimulationStore()

/** Emprise WGS84 pour Overpass : borne la taille pour éviter les timeouts. */
function clampOsmBbox(west: number, south: number, east: number, north: number) {
  const maxSpan = 0.22
  const cx = (west + east) / 2
  const cy = (south + north) / 2
  let lonSpan = east - west
  let latSpan = north - south
  if (lonSpan > maxSpan) {
    lonSpan = maxSpan
    west = cx - lonSpan / 2
    east = cx + lonSpan / 2
  }
  if (latSpan > maxSpan) {
    latSpan = maxSpan
    south = cy - latSpan / 2
    north = cy + latSpan / 2
  }
  return { west, south, east, north }
}

function syncOsmBboxFromView() {
  if (!viewer) return
  const rect = viewer.camera.computeViewRectangle(viewer.scene.globe.ellipsoid)
  if (!defined(rect)) return
  let west = CesiumMath.toDegrees(rect.west)
  let south = CesiumMath.toDegrees(rect.south)
  let east = CesiumMath.toDegrees(rect.east)
  let north = CesiumMath.toDegrees(rect.north)
  if (![west, south, east, north].every(Number.isFinite)) return
  if (east <= west || north <= south) return
  if (east - west > 350) return
  const b = clampOsmBbox(west, south, east, north)
  simStore.setOsmBbox(b)
  syncInferenceBboxOverlay()
}

function syncInferenceBboxOverlay() {
  if (!viewer) return
  if (inferenceBboxEntity) {
    viewer.entities.remove(inferenceBboxEntity)
    inferenceBboxEntity = undefined
  }
  if (!simStore.includeOsmBuildings || !simStore.osmBbox) return
  const { west, south, east, north } = simStore.osmBbox
  inferenceBboxEntity = viewer.entities.add({
    rectangle: {
      coordinates: Rectangle.fromDegrees(west, south, east, north),
      material: Color.CYAN.withAlpha(0.06),
      outline: true,
      outlineColor: Color.CYAN.withAlpha(0.85),
      outlineWidth: 2,
      height: 0,
    },
  })
}

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

  // Sans MNT, l’ellipsoïde est « plat » alors que les bâtiments Ion ont des hauteurs réelles → effet « flottants ».
  try {
    viewer.terrainProvider = await createWorldTerrainAsync({
      requestVertexNormals: true,
    })
    viewer.scene.globe.depthTestAgainstTerrain = true
  } catch (e) {
    console.warn(
      'Cesium World Terrain indisponible (jeton Ion ou asset 1) — bâtiments peuvent sembler en l’air :',
      e,
    )
  }

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

  ionOsmBuildingsHint.value = ''
  ionOsmTileset = null
  const token = import.meta.env.VITE_CESIUM_ION_TOKEN?.trim() ?? ''
  if (!token) {
    ionOsmBuildingsHint.value =
      'Jeton Cesium Ion manquant : ajoutez CESIUM_ION_TOKEN dans la racine .env (voir AGENTS.md) pour les bâtiments 3D.'
  }
  try {
    const osmBuildings = await createOsmBuildingsAsync()
    viewer.scene.primitives.add(osmBuildings)
    ionOsmTileset = osmBuildings
    osmBuildings.show = simStore.includeOsmBuildings
    ionOsmBuildingsHint.value = ''
  } catch (err) {
    ionOsmTileset = null
    if (!ionOsmBuildingsHint.value) {
      ionOsmBuildingsHint.value =
        'Bâtiments 3D Ion indisponibles (vérifiez le jeton et l’accès à l’asset OSM Buildings dans Ion).'
    }
    console.warn(
      'Cesium Ion OSM Buildings : échec (souvent 404 si le jeton ne voit pas l’asset 96188 — Ion → Asset Depot).',
      err,
    )
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

  viewer.camera.moveEnd.addEventListener(syncOsmBboxFromView)
  syncOsmBboxFromView()

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
      startAnimation(viewer, result.wind_field, windParticlesContextFromResult(result))
      focusCameraOnPoints(viewer, geoPointsFromSimulation(result), { duration: 1.35 })
    } else {
      clearOverlay(viewer)
      stopAnimation(viewer)
    }
  },
)

watch(
  () => [simStore.osmBbox, simStore.includeOsmBuildings] as const,
  () => {
    syncInferenceBboxOverlay()
    if (ionOsmTileset) ionOsmTileset.show = simStore.includeOsmBuildings
  },
  { deep: true },
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
    viewer.camera.moveEnd.removeEventListener(syncOsmBboxFromView)
    if (inferenceBboxEntity) {
      viewer.entities.remove(inferenceBboxEntity)
      inferenceBboxEntity = undefined
    }
    clearOverlay(viewer)
    syncGeometryMarkers(viewer, undefined)
  }
  handler?.destroy()
  handler = null
  viewer?.destroy()
  viewer = null
  ionOsmTileset = null
})
</script>

<style scoped>
.map-hint {
  position: absolute;
  left: 12px;
  right: 12px;
  bottom: 10px;
  z-index: 2;
  padding: 8px 10px;
  border-radius: 6px;
  background: rgba(12, 18, 24, 0.82);
  color: rgba(255, 255, 255, 0.88);
  border: 1px solid rgba(0, 232, 255, 0.35);
  pointer-events: none;
  max-width: 42rem;
}

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
