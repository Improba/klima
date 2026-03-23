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

export interface OsmBuildingBbox {
  west: number
  south: number
  east: number
  north: number
}

export interface SimulateRequest {
  wind_speed: number
  wind_direction: number
  sun_elevation: number
  t_ambient: number
  geometry: GeometryBlock[]
  project_id?: string
  scenario_id?: string
  /** Bbox WGS84 de la vue carte → backend Overpass + voxels bâtiment */
  osm_building_bbox?: OsmBuildingBbox
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

/** Grille d’occupancy sous-échantillonnée (alignée sur le canal 0 du tenseur d’inférence). */
export interface OccupancyGrid {
  stride: [number, number, number]
  dims: [number, number, number]
  /** Row-major : ix + dims[0] * (iy + dims[1] * iz) ; 0 = air, 1 = solide */
  cells: number[]
}

export interface SimulationResult {
  surface_temperatures: SurfaceTemperature[]
  wind_field: WindFieldSample[]
  metadata: {
    grid_resolution: [number, number, number]
    wind_subsample: [number, number, number]
    num_surface_points: number
    num_wind_samples: number
    inference_time_ms: number
    model_loaded: boolean
    t_ambient: number
    delta_t_range: [number, number]
    wind_speed_range: [number, number]
    /** m par couche Z (souvent 2), aligné backend `Z_METERS_PER_VOXEL` */
    z_meters_per_voxel?: number
  }
  occupancy?: OccupancyGrid | null
}

/** Contexte pour l’advection des particules (occupancy + résolution grille). */
export interface WindParticlesContext {
  occupancy?: OccupancyGrid | null
  gridResolution: [number, number, number]
  zMetersPerVoxel?: number
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
