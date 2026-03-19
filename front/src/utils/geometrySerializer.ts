import type { GeometryBlock, GeometryDiff, SimulateRequest } from 'src/types'

export function serializeModifications(modifications: GeometryBlock[]): GeometryDiff {
  return { modifications }
}

export function computeSimulationKey(params: SimulateRequest): string {
  const sorted = JSON.stringify(params, Object.keys(params).sort())
  let hash = 0
  for (let i = 0; i < sorted.length; i++) {
    hash = ((hash << 5) - hash + sorted.charCodeAt(i)) | 0
  }
  return `sim_${(hash >>> 0).toString(36)}`
}
