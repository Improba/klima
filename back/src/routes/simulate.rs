use axum::extract::State;
use axum::routing::post;
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::sync::Arc;
use std::time::Instant;

use ndarray::ArrayD;
use uuid::Uuid;

use crate::db;
use crate::error::AppError;
use crate::inference::fno_client;
use crate::inference::osm_buildings;
use crate::inference::overlay_geo;
use crate::inference::postprocessor::{self, SimulationResult, WindFieldSample};
use crate::inference::preprocessor::{self, GeometryBlock};
use crate::AppState;

#[derive(Clone, Deserialize, Serialize)]
pub struct OsmBuildingBbox {
    pub west: f64,
    pub south: f64,
    pub east: f64,
    pub north: f64,
}

#[derive(Clone, Deserialize, Serialize)]
pub struct SimulateRequest {
    pub project_id: Option<Uuid>,
    pub scenario_id: Option<Uuid>,
    pub wind_speed: f64,
    pub wind_direction: f64,
    pub sun_elevation: f64,
    pub t_ambient: Option<f64>,
    pub geometry: Vec<GeometryBlock>,
    /// Si présent : récupération Overpass `building=*` dans la bbox puis rasterisation en voxels « batiment ».
    #[serde(default)]
    pub osm_building_bbox: Option<OsmBuildingBbox>,
}

async fn simulate(
    State(state): State<Arc<AppState>>,
    Json(req): Json<SimulateRequest>,
) -> Result<Json<SimulationResult>, AppError> {
    let t_ambient = req.t_ambient.unwrap_or(20.0);

    tracing::info!(
        "Simulation request: wind={} m/s, dir={}°, sun={}°, {} blocks, osm_bbox={}",
        req.wind_speed,
        req.wind_direction,
        req.sun_elevation,
        req.geometry.len(),
        req.osm_building_bbox.is_some()
    );

    let cache_key = simulation_cache_key(&req);
    if let Some(cached) = state.cache.get(&cache_key) {
        tracing::debug!("Cache hit for key {}", cache_key);
        return Ok(Json(cached));
    }

    let osm_ways_storage = if let Some(ref bb) = req.osm_building_bbox {
        match osm_buildings::fetch_building_ways(
            &state.http_client,
            bb.west,
            bb.south,
            bb.east,
            bb.north,
        )
        .await
        {
            Ok(v) if !v.is_empty() => {
                tracing::info!("OSM buildings: {} ways in bbox", v.len());
                Some(v)
            }
            Ok(_) => {
                tracing::debug!("OSM buildings: empty result for bbox");
                None
            }
            Err(e) => {
                tracing::warn!("OSM Overpass failed (simulation without OSM solids): {}", e);
                None
            }
        }
    } else {
        None
    };
    let osm_slice = osm_ways_storage.as_deref();

    let tensor = preprocessor::preprocess_geometry(
        &req.geometry,
        req.wind_speed,
        req.wind_direction,
        req.sun_elevation,
        t_ambient,
        osm_slice,
    );
    let tensor_dyn: ArrayD<f32> = tensor.into_dyn();
    let occupancy = postprocessor::pack_occupancy(&tensor_dyn, [8, 8, 4]);

    let t_infer = Instant::now();
    let mut output: Option<ArrayD<f32>> = None;
    let mut model_loaded = false;

    if let Some(url) = state.fno_infer_url.as_deref() {
        match fno_client::predict(&state.http_client, url, &tensor_dyn).await {
            Ok(o) => {
                output = Some(o);
                model_loaded = true;
            }
            Err(e) => tracing::warn!(
                "FNO sidecar inference failed ({}); trying ONNX or mock",
                e
            ),
        }
    }

    let output = if let Some(o) = output {
        o
    } else if state.onnx.is_loaded() {
        model_loaded = true;
        state.onnx.predict(tensor_dyn.clone()).await?
    } else {
        tracing::debug!("No FNO URL success and no ONNX — returning mock data");
        let mock = generate_mock_result(&req, &tensor_dyn, t_ambient, occupancy.clone());
        state.cache.insert(&cache_key, mock.clone());
        return Ok(Json(mock));
    };

    let inference_time_ms = t_infer.elapsed().as_millis() as u64;

    let result = postprocessor::postprocess(
        &output,
        occupancy,
        t_ambient,
        inference_time_ms,
        model_loaded,
    );

    state.cache.insert(&cache_key, result.clone());

    if let Some(project_id) = req.project_id {
        let params = serde_json::json!({
            "wind_speed": req.wind_speed,
            "wind_direction": req.wind_direction,
            "sun_elevation": req.sun_elevation,
            "t_ambient": t_ambient,
            "geometry": req.geometry,
        });
        let result_bytes = serde_json::to_vec(&result).unwrap_or_default();
        if let Ok(sim) =
            db::create_simulation(&state.pool, project_id, req.scenario_id, params).await
        {
            let _ =
                db::update_simulation_result(&state.pool, sim.id, &result_bytes, "completed")
                    .await;
        }
    }

    Ok(Json(result))
}

/// Cache key from physics + geometry only (not `project_id` / `scenario_id`).
/// Geometry order is normalized (index sort, no full geometry clone). Preimage is
/// SHA-256 over little-endian scalars + UTF-8 surface types for stable keys across Rust versions.
fn simulation_cache_key(req: &SimulateRequest) -> String {
    let t_ambient = req.t_ambient.unwrap_or(20.0);
    let mut order: Vec<usize> = (0..req.geometry.len()).collect();
    order.sort_by(|&i, &j| {
        let a = &req.geometry[i];
        let b = &req.geometry[j];
        a.x
            .total_cmp(&b.x)
            .then_with(|| a.y.total_cmp(&b.y))
            .then_with(|| a.z.total_cmp(&b.z))
            .then_with(|| a.surface_type.cmp(&b.surface_type))
    });

    let mut buf = Vec::new();
    cache_push_f64(&mut buf, req.wind_speed);
    cache_push_f64(&mut buf, req.wind_direction);
    cache_push_f64(&mut buf, req.sun_elevation);
    cache_push_f64(&mut buf, t_ambient);
    if let Some(ref bb) = req.osm_building_bbox {
        cache_push_f64(&mut buf, bb.west);
        cache_push_f64(&mut buf, bb.south);
        cache_push_f64(&mut buf, bb.east);
        cache_push_f64(&mut buf, bb.north);
    }
    for &i in &order {
        let b = &req.geometry[i];
        cache_push_f64(&mut buf, b.x);
        cache_push_f64(&mut buf, b.y);
        cache_push_f64(&mut buf, b.z);
        cache_push_str(&mut buf, &b.surface_type);
    }
    format!("{:x}", Sha256::digest(&buf))
}

fn cache_push_f64(buf: &mut Vec<u8>, v: f64) {
    buf.extend_from_slice(&v.to_le_bytes());
}

fn cache_push_str(buf: &mut Vec<u8>, s: &str) {
    let b = s.as_bytes();
    buf.extend_from_slice(&(b.len() as u64).to_le_bytes());
    buf.extend_from_slice(b);
}

/// Must match `ORIGIN_*` and `CELL_SIZE_DEG` in `front/src/composables/useThermalOverlay.ts`
/// and `useWindParticles.ts`. API `lon`/`lat` on surface points and `x`/`y` on wind samples are
/// grid indices, not WGS84 — the viewer maps them to degrees around this origin.
const OVERLAY_ORIGIN_LON: f64 = 2.3400;
const OVERLAY_ORIGIN_LAT: f64 = 48.8500;
const OVERLAY_CELL_DEG: f64 = 0.00002;

fn wgs84_to_overlay_grid(lon: f64, lat: f64) -> (f64, f64) {
    let gx = ((lon - OVERLAY_ORIGIN_LON) / OVERLAY_CELL_DEG).round();
    let gy = ((lat - OVERLAY_ORIGIN_LAT) / OVERLAY_CELL_DEG).round();
    (gx, gy)
}

/// Vent mock : même sous-échantillonnage que `postprocessor::postprocess`, vitesse nulle dans les solides.
/// Composantes alignées sur `training/src/data/synthetic_physics.py` et `encoding.encode_input` :
/// `vx = V·sin(dir)`, `vy = V·cos(dir)` avec `dir` en degrés, convention identique au canal 11–12 du tenseur.
fn mock_wind_field_from_occupancy(
    input: &ArrayD<f32>,
    wind_speed: f64,
    wind_direction_deg: f64,
) -> Vec<WindFieldSample> {
    let shape = input.shape();
    if shape.len() != 5 {
        return vec![];
    }
    let nx = shape[2];
    let ny = shape[3];
    let nz = shape[4];
    if nx == 0 || ny == 0 || nz == 0 {
        return vec![];
    }

    let (target_wx, target_wy, target_wz) = (64usize, 64usize, 16usize);
    let step_x = (nx / target_wx.min(nx)).max(1);
    let step_y = (ny / target_wy.min(ny)).max(1);
    let step_z = (nz / target_wz.min(nz)).max(1);

    let rad = wind_direction_deg.to_radians();
    let base_vx = wind_speed * rad.sin();
    let base_vy = wind_speed * rad.cos();

    let mut wind_field = Vec::new();
    let mut ix = 0usize;
    while ix < nx {
        let mut iy = 0usize;
        while iy < ny {
            let mut iz = 0usize;
            while iz < nz {
                let solid = input[[0, 0, ix, iy, iz]] > 0.5;
                let (vx, vy, vz) = if solid {
                    (0.0, 0.0, 0.0)
                } else {
                    (base_vx, base_vy, 0.0)
                };
                wind_field.push(WindFieldSample {
                    x: ix as f64,
                    y: iy as f64,
                    z: iz as f64,
                    vx,
                    vy,
                    vz,
                });
                iz += step_z;
            }
            iy += step_y;
        }
        ix += step_x;
    }
    wind_field
}

fn generate_mock_result(
    req: &SimulateRequest,
    tensor: &ArrayD<f32>,
    t_ambient: f64,
    occupancy: Option<postprocessor::OccupancyGrid>,
) -> SimulationResult {
    let surface_temperatures: Vec<_> = req
        .geometry
        .iter()
        .map(|b| {
            let (gx, gy) = wgs84_to_overlay_grid(b.x, b.y);
            postprocessor::SurfaceTemperature {
                lon: gx,
                lat: gy,
                alt: b.z,
                temperature: t_ambient + 2.0,
            }
        })
        .collect();

    let wind_field = mock_wind_field_from_occupancy(tensor, req.wind_speed, req.wind_direction);

    let shape = tensor.shape();
    let (nx, ny, nz) = if shape.len() == 5 {
        (shape[2], shape[3], shape[4])
    } else {
        (256, 256, 64)
    };

    let wind_speed_range = if wind_field.is_empty() {
        [0.0, 0.0]
    } else {
        let mut min_ws = f64::MAX;
        let mut max_ws = f64::MIN;
        for w in &wind_field {
            let speed = (w.vx * w.vx + w.vy * w.vy + w.vz * w.vz).sqrt();
            if speed < min_ws {
                min_ws = speed;
            }
            if speed > max_ws {
                max_ws = speed;
            }
        }
        [min_ws, max_ws]
    };

    SimulationResult {
        metadata: postprocessor::ResultMetadata {
            grid_resolution: [nx, ny, nz],
            wind_subsample: [64usize.min(nx), 64usize.min(ny), 16usize.min(nz)],
            num_surface_points: surface_temperatures.len(),
            num_wind_samples: wind_field.len(),
            inference_time_ms: 0,
            model_loaded: false,
            t_ambient,
            delta_t_range: [2.0, 2.0],
            wind_speed_range,
            z_meters_per_voxel: overlay_geo::Z_METERS_PER_VOXEL,
        },
        surface_temperatures,
        wind_field,
        occupancy,
    }
}

pub fn router() -> Router<Arc<AppState>> {
    Router::new().route("/simulate", post(simulate))
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    fn sample_block(x: f64, y: f64, z: f64) -> GeometryBlock {
        GeometryBlock {
            x,
            y,
            z,
            surface_type: "batiment".into(),
        }
    }

    #[test]
    fn cache_key_ignores_project_and_scenario_ids() {
        let a = SimulateRequest {
            project_id: Some(Uuid::nil()),
            scenario_id: None,
            wind_speed: 2.0,
            wind_direction: 180.0,
            sun_elevation: 45.0,
            t_ambient: Some(19.0),
            geometry: vec![sample_block(1.0, 2.0, 0.0)],
            osm_building_bbox: None,
        };
        let mut b = a.clone();
        b.project_id = None;
        b.scenario_id = Some(Uuid::nil());
        assert_eq!(simulation_cache_key(&a), simulation_cache_key(&b));
    }

    #[test]
    fn cache_key_stable_under_geometry_permutation() {
        let a = SimulateRequest {
            project_id: None,
            scenario_id: None,
            wind_speed: 1.0,
            wind_direction: 0.0,
            sun_elevation: 10.0,
            t_ambient: None,
            geometry: vec![sample_block(0.0, 1.0, 0.0), sample_block(1.0, 0.0, 0.0)],
            osm_building_bbox: None,
        };
        let mut b = a.clone();
        b.geometry.reverse();
        assert_eq!(simulation_cache_key(&a), simulation_cache_key(&b));
    }
}
