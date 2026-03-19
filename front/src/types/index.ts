export interface Project {
  id: string
  name: string
  description: string | null
  created_at: string
  updated_at: string
}

export interface Scenario {
  id: string
  project_id: string
  name: string
  geometry: GeometryDiff
  metadata: Record<string, unknown> | null
  created_at: string
}

export interface GeometryBlock {
  x: number
  y: number
  z: number
  surface_type: string
}

export interface GeometryDiff {
  modifications: GeometryBlock[]
}

export interface SimulateRequest {
  wind_speed: number
  wind_direction: number
  sun_elevation: number
  t_ambient: number
  geometry: GeometryBlock[]
  project_id?: string
  scenario_id?: string
}

export interface SurfaceTemperature {
  lon: number
  lat: number
  alt: number
  temperature: number
}

export interface WindFieldSample {
  x: number
  y: number
  z: number
  vx: number
  vy: number
  vz: number
}

export interface SimulationResult {
  surface_temperatures: SurfaceTemperature[]
  wind_field: WindFieldSample[]
  metadata: {
    inference_time_ms: number
    model_loaded: boolean
    t_ambient: number
    delta_t_range: [number, number]
    wind_speed_range: [number, number]
  }
}

export interface SimulateResponse {
  id: string
  status: string
  result: SimulationResult
}

export interface Simulation {
  id: string
  project_id: string
  scenario_id: string | null
  params: Record<string, unknown>
  status: string
  created_at: string
}

export type SurfaceType =
  | 'bitume'
  | 'herbe'
  | 'eau'
  | 'gravier'
  | 'vegetation'
  | 'batiment'
