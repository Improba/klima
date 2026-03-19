import { ref } from 'vue'
import type { GeometryBlock, SurfaceType } from 'src/types'

export function useSurfaceEditor() {
  const modifications = ref<GeometryBlock[]>([])
  const activeTool = ref<string>('select')
  const activeSurfaceType = ref<SurfaceType>('herbe')

  function addModification(lon: number, lat: number, alt: number, surfaceType: SurfaceType) {
    modifications.value.push({
      x: lon,
      y: lat,
      z: alt,
      surface_type: surfaceType,
    })
  }

  function removeModification(index: number) {
    if (index >= 0 && index < modifications.value.length) {
      modifications.value.splice(index, 1)
    }
  }

  function clearModifications() {
    modifications.value = []
  }

  function getGeometryBlocks(): GeometryBlock[] {
    return [...modifications.value]
  }

  return {
    modifications,
    activeTool,
    activeSurfaceType,
    addModification,
    removeModification,
    clearModifications,
    getGeometryBlocks,
  }
}
