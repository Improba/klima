import { ref } from 'vue'
import { defineStore } from 'pinia'
import type { SimulationResult, GeometryBlock, OsmBuildingBbox, SimulateRequest } from 'src/types'
import { useApi } from 'src/composables/useApi'

export const useSimulationStore = defineStore('simulation', () => {
  const lastResult = ref<SimulationResult | null>(null)
  const isSimulating = ref(false)
  const geometry = ref<GeometryBlock[]>([])
  /** Dernière emprise caméra (WGS84), mise à jour par CesiumViewer */
  const osmBbox = ref<OsmBuildingBbox | null>(null)
  /** Bâtiments OSM : affichage 3D (Cesium Ion) + bbox Overpass côté API */
  const includeOsmBuildings = ref(true)
  const params = ref({
    windSpeed: 5,
    windDirection: 180,
    sunElevation: 45,
    tAmbient: 30,
  })

  const api = useApi()

  function setOsmBbox(bbox: OsmBuildingBbox | null) {
    osmBbox.value = bbox
  }

  async function runSimulation(projectId?: string, scenarioId?: string) {
    isSimulating.value = true
    try {
      const body: SimulateRequest = {
        wind_speed: params.value.windSpeed,
        wind_direction: params.value.windDirection,
        sun_elevation: params.value.sunElevation,
        t_ambient: params.value.tAmbient,
        geometry: geometry.value,
        project_id: projectId,
        scenario_id: scenarioId,
      }
      if (includeOsmBuildings.value && osmBbox.value) {
        body.osm_building_bbox = { ...osmBbox.value }
      }
      const res = await api.simulate(body)
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
    osmBbox.value = null
  }

  return {
    lastResult,
    isSimulating,
    geometry,
    osmBbox,
    includeOsmBuildings,
    params,
    setOsmBbox,
    runSimulation,
    clearResult,
    resetForProject,
  }
})
