import { ref } from 'vue'
import type {
  Project,
  Scenario,
  GeometryDiff,
  SimulateRequest,
  SimulationResult,
  Simulation,
} from 'src/types'

export function useApi() {
  const baseUrl = '/api'
  const loading = ref(false)
  const error = ref<string | null>(null)

  async function request<T>(path: string, options?: RequestInit): Promise<T> {
    loading.value = true
    error.value = null
    try {
      const res = await fetch(`${baseUrl}${path}`, {
        headers: { 'Content-Type': 'application/json' },
        ...options,
      })
      if (!res.ok) {
        const body = await res.text()
        throw new Error(body || `HTTP ${res.status}`)
      }
      if (res.status === 204) return undefined as unknown as T
      return (await res.json()) as T
    } catch (err) {
      const msg = err instanceof Error ? err.message : String(err)
      error.value = msg
      throw err
    } finally {
      loading.value = false
    }
  }

  async function getProjects(limit = 50, offset = 0): Promise<Project[]> {
    return request<Project[]>(`/projects?limit=${limit}&offset=${offset}`)
  }

  async function getProject(id: string): Promise<Project> {
    return request<Project>(`/projects/${id}`)
  }

  async function createProject(name: string, description?: string): Promise<Project> {
    return request<Project>('/projects', {
      method: 'POST',
      body: JSON.stringify({ name, description: description ?? null }),
    })
  }

  async function updateProject(id: string, name: string, description?: string): Promise<Project> {
    return request<Project>(`/projects/${id}`, {
      method: 'PUT',
      body: JSON.stringify({ name, description: description ?? null }),
    })
  }

  async function deleteProject(id: string): Promise<void> {
    return request<void>(`/projects/${id}`, { method: 'DELETE' })
  }

  async function getScenarios(projectId: string): Promise<Scenario[]> {
    return request<Scenario[]>(`/projects/${projectId}/scenarios`)
  }

  async function createScenario(
    projectId: string,
    name: string,
    geometry: GeometryDiff,
  ): Promise<Scenario> {
    return request<Scenario>(`/projects/${projectId}/scenarios`, {
      method: 'POST',
      body: JSON.stringify({ name, geometry }),
    })
  }

  async function deleteScenario(id: string): Promise<void> {
    return request<void>(`/scenarios/${id}`, { method: 'DELETE' })
  }

  async function simulate(req: SimulateRequest): Promise<SimulationResult> {
    return request<SimulationResult>('/simulate', {
      method: 'POST',
      body: JSON.stringify(req),
    })
  }

  async function getSimulation(id: string): Promise<Simulation> {
    return request<Simulation>(`/simulations/${id}`)
  }

  return {
    loading,
    error,
    getProjects,
    getProject,
    createProject,
    updateProject,
    deleteProject,
    getScenarios,
    createScenario,
    deleteScenario,
    simulate,
    getSimulation,
  }
}
