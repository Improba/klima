<template>
  <q-page class="cesium-page">
    <CesiumViewer
      :wind-speed="windSpeed"
      :wind-direction="windDirection"
      :sun-elevation="sunElevation"
    />
  </q-page>
</template>

<script setup lang="ts">
import CesiumViewer from 'components/CesiumViewer.vue'

defineProps<{
  windSpeed: number
  windDirection: number
  sunElevation: number
}>()
</script>

<style scoped lang="scss">
.cesium-page {
  padding: 0;
  height: 100%;
}
</style>
