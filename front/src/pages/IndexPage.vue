<template>
  <q-page class="cesium-page">
    <q-inner-loading :showing="projectStore.loading" color="cyan" />
    <CesiumViewer
      ref="cesiumViewerRef"
      :wind-speed="simStore.params.windSpeed"
      :wind-direction="simStore.params.windDirection"
      :sun-elevation="simStore.params.sunElevation"
      :simulation-result="simStore.lastResult"
      :active-tool="surfaceEditor.activeTool.value"
      :active-surface-type="surfaceEditor.activeSurfaceType.value"
      @map-click="onMapClick"
    />
    <EditorToolbar
      @tool-change="surfaceEditor.activeTool.value = $event"
      @surface-type-change="surfaceEditor.activeSurfaceType.value = $event"
    />
    <TimeSlider @update:sun-elevation="simStore.params.sunElevation = $event" />
    <ResultLegend :result="simStore.lastResult" />
    <div v-if="simStore.lastResult" class="export-actions">
      <q-btn flat round icon="photo_camera" color="white" size="sm" @click="takeScreenshot">
        <q-tooltip>Screenshot PNG</q-tooltip>
      </q-btn>
      <q-btn flat round icon="download" color="white" size="sm" @click="downloadCSV">
        <q-tooltip>Export CSV</q-tooltip>
      </q-btn>
    </div>
  </q-page>
</template>

<script setup lang="ts">
import { ref, onMounted, watch } from 'vue'
import CesiumViewer from 'components/CesiumViewer.vue'
import EditorToolbar from 'components/EditorToolbar.vue'
import TimeSlider from 'components/TimeSlider.vue'
import ResultLegend from 'components/ResultLegend.vue'
import { useProjectStore } from 'src/stores/project'
import { useSimulationStore } from 'src/stores/simulation'
import { useSurfaceEditor } from 'src/composables/useSurfaceEditor'
import { useExport } from 'src/composables/useExport'
import type { SurfaceType } from 'src/types'

const props = defineProps<{ id: string }>()

const projectStore = useProjectStore()
const simStore = useSimulationStore()
const surfaceEditor = useSurfaceEditor()
const { exportScreenshot, exportCSV, downloadBlob, downloadText } = useExport()
const cesiumViewerRef = ref<InstanceType<typeof CesiumViewer> | null>(null)

onMounted(() => {
  projectStore.fetchProject(props.id)
})

watch(
  () => props.id,
  (newId) => {
    projectStore.fetchProject(newId)
    simStore.clearResult()
  },
)

function onMapClick(payload: { lon: number; lat: number; alt: number }) {
  const surfaceType = surfaceEditor.activeSurfaceType.value as SurfaceType
  surfaceEditor.addModification(payload.lon, payload.lat, payload.alt, surfaceType)
  simStore.geometry.push({
    x: payload.lon,
    y: payload.lat,
    z: payload.alt,
    surface_type: surfaceType,
  })
}

async function takeScreenshot() {
  const viewer = cesiumViewerRef.value?.getViewer()
  if (!viewer) return
  const blob = await exportScreenshot(viewer)
  downloadBlob(blob, `klima-screenshot-${Date.now()}.png`)
}

function downloadCSV() {
  const result = simStore.lastResult
  if (!result) return
  const csv = exportCSV(result)
  downloadText(csv, `klima-export-${Date.now()}.csv`, 'text/csv')
}
</script>

<style scoped lang="scss">
.cesium-page {
  padding: 0;
  height: 100%;
}

.export-actions {
  position: absolute;
  top: 16px;
  left: 200px;
  z-index: 10;
  display: flex;
  gap: 4px;
  background: rgba(30, 30, 30, 0.92);
  border-radius: 8px;
  padding: 4px;
  backdrop-filter: blur(8px);
}
</style>
