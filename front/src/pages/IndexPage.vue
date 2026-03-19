<template>
  <q-page class="cesium-page">
    <q-inner-loading :showing="projectStore.loading" color="cyan" />
    <CesiumViewer
      :wind-speed="simStore.params.windSpeed"
      :wind-direction="simStore.params.windDirection"
      :sun-elevation="simStore.params.sunElevation"
      :simulation-result="simStore.lastResult"
    />
    <EditorToolbar
      @tool-change="surfaceEditor.activeTool.value = $event"
      @surface-type-change="surfaceEditor.activeSurfaceType.value = $event"
    />
    <TimeSlider @update:sun-elevation="simStore.params.sunElevation = $event" />
    <ResultLegend :result="simStore.lastResult" />
  </q-page>
</template>

<script setup lang="ts">
import { onMounted, watch } from 'vue'
import CesiumViewer from 'components/CesiumViewer.vue'
import EditorToolbar from 'components/EditorToolbar.vue'
import TimeSlider from 'components/TimeSlider.vue'
import ResultLegend from 'components/ResultLegend.vue'
import { useProjectStore } from 'src/stores/project'
import { useSimulationStore } from 'src/stores/simulation'
import { useSurfaceEditor } from 'src/composables/useSurfaceEditor'

const props = defineProps<{ id: string }>()

const projectStore = useProjectStore()
const simStore = useSimulationStore()
const surfaceEditor = useSurfaceEditor()

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
</script>

<style scoped lang="scss">
.cesium-page {
  padding: 0;
  height: 100%;
}
</style>
