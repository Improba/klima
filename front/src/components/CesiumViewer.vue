<template>
  <div ref="cesiumContainer" class="cesium-wrapper" />
</template>

<script setup lang="ts">
import { ref, onMounted, onBeforeUnmount, watch } from 'vue'
import {
  Viewer,
  Terrain,
  Cartesian2,
  Cartesian3,
  createOsmBuildingsAsync,
  Math as CesiumMath,
  defined,
  ScreenSpaceEventHandler,
  ScreenSpaceEventType,
} from 'cesium'
import 'cesium/Build/Cesium/Widgets/widgets.css'
import type { SimulationResult } from 'src/types'
import { useThermalOverlay } from 'src/composables/useThermalOverlay'
import { useWindParticles } from 'src/composables/useWindParticles'

const props = defineProps<{
  windSpeed: number
  windDirection: number
  sunElevation: number
  simulationResult?: SimulationResult | null
  activeTool: string
  activeSurfaceType: string
}>()

const emit = defineEmits<{
  'map-click': [payload: { lon: number; lat: number; alt: number }]
}>()

const cesiumContainer = ref<HTMLElement>()
let viewer: Viewer | null = null
let handler: ScreenSpaceEventHandler | null = null

const { applyOverlay, clearOverlay } = useThermalOverlay()
const { startAnimation, stopAnimation } = useWindParticles()

onMounted(async () => {
  if (!cesiumContainer.value) return

  viewer = new Viewer(cesiumContainer.value, {
    terrain: Terrain.fromWorldTerrain(),
    infoBox: false,
    selectionIndicator: false,
    timeline: false,
    animation: false,
    homeButton: false,
    geocoder: false,
    baseLayerPicker: false,
    sceneModePicker: false,
    navigationHelpButton: false,
    fullscreenButton: false,
    vrButton: false,
  })

  viewer.scene.globe.enableLighting = true

  try {
    const osmBuildings = await createOsmBuildingsAsync()
    viewer.scene.primitives.add(osmBuildings)
  } catch (err) {
    console.warn('OSM Buildings unavailable:', err)
  }

  // Default view: Paris center
  viewer.camera.flyTo({
    destination: Cartesian3.fromDegrees(2.3522, 48.8566, 1500),
    orientation: {
      heading: CesiumMath.toRadians(0),
      pitch: CesiumMath.toRadians(-45),
      roll: 0,
    },
  })

  setupClickHandler()
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
    } else {
      clearOverlay(viewer)
      stopAnimation(viewer)
    }
  },
)

function getViewer(): Viewer | null {
  return viewer
}

defineExpose({ getViewer })

onBeforeUnmount(() => {
  stopAnimation(viewer ?? undefined)
  if (viewer) clearOverlay(viewer)
  handler?.destroy()
  handler = null
  viewer?.destroy()
  viewer = null
})
</script>

<style scoped>
.cesium-wrapper {
  width: 100%;
  height: 100vh;
  margin: 0;
  padding: 0;
  overflow: hidden;
}
</style>
