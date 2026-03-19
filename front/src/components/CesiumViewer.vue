<template>
  <div ref="cesiumContainer" class="cesium-wrapper" />
</template>

<script setup lang="ts">
import { ref, onMounted, onBeforeUnmount, watch } from 'vue'
import {
  Viewer,
  Terrain,
  Cartesian3,
  createOsmBuildingsAsync,
  Math as CesiumMath,
  Color,
  defined,
  ScreenSpaceEventHandler,
  ScreenSpaceEventType,
} from 'cesium'
import 'cesium/Build/Cesium/Widgets/widgets.css'

const props = defineProps<{
  windSpeed: number
  windDirection: number
  sunElevation: number
}>()

const cesiumContainer = ref<HTMLElement>()
let viewer: Viewer | null = null
let handler: ScreenSpaceEventHandler | null = null

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
  handler.setInputAction((event: { position: { x: number; y: number } }) => {
    if (!viewer) return
    const picked = viewer.scene.pick(event.position)
    if (defined(picked)) {
      console.log('Picked entity:', picked)
    }

    const cartesian = viewer.scene.pickPosition(event.position)
    if (defined(cartesian)) {
      const cartographic = viewer.scene.globe.ellipsoid.cartesianToCartographic(cartesian)
      console.log(
        'Clicked:',
        CesiumMath.toDegrees(cartographic.longitude).toFixed(6),
        CesiumMath.toDegrees(cartographic.latitude).toFixed(6),
      )
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

onBeforeUnmount(() => {
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
