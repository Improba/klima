import { ref } from 'vue'
import { defineStore } from 'pinia'
import type { SimulationResult, GeometryBlock } from 'src/types'
import { useApi } from 'src/composables/useApi'

export const useSimulationStore = defineStore('simulation', () => {
  const lastResult = ref<SimulationResult | null>(null)
  const isSimulating = ref(false)
  const geometry = ref<GeometryBlock[]>([])
  const params = ref({
    windSpeed: 5,
    windDirection: 180,
    sunElevation: 45,
    tAmbient: 30,
  })

  const api = useApi()

  async function runSimulation(projectId?: string, scenarioId?: string) {
    isSimulating.value = true
    try {
      const res = await api.simulate({
        wind_speed: params.value.windSpeed,
        wind_direction: params.value.windDirection,
        sun_elevation: params.value.sunElevation,
        t_ambient: params.value.tAmbient,
        geometry: geometry.value,
        project_id: projectId,
        scenario_id: scenarioId,
      })
      lastResult.value = res
      return res
    } finally {
      isSimulating.value = false
    }
  }

  function clearResult() {
    lastResult.value = null
  }

  function resetForProject() {
    lastResult.value = null
    geometry.value = []
  }

  return {
    lastResult,
    isSimulating,
    geometry,
    params,
    runSimulation,
    clearResult,
    resetForProject,
  }
})
