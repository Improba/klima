<template>
  <q-page class="cesium-page">
    <q-inner-loading :showing="projectStore.loading" color="cyan" />
    <CesiumViewer
      :wind-speed="simStore.params.windSpeed"
      :wind-direction="simStore.params.windDirection"
      :sun-elevation="simStore.params.sunElevation"
      :simulation-result="simStore.lastResult"
    />
  </q-page>
</template>

<script setup lang="ts">
import { onMounted, watch } from 'vue'
import CesiumViewer from 'components/CesiumViewer.vue'
import { useProjectStore } from 'src/stores/project'
import { useSimulationStore } from 'src/stores/simulation'

const props = defineProps<{ id: string }>()

const projectStore = useProjectStore()
const simStore = useSimulationStore()

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
