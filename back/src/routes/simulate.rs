use axum::extract::State;
use axum::routing::post;
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::sync::Arc;
use std::time::Instant;
use uuid::Uuid;

use crate::db;
use crate::error::AppError;
use crate::inference::fno_client;
use crate::inference::postprocessor::{self, SimulationResult};
use crate::inference::preprocessor::{self, GeometryBlock};
use ndarray::ArrayD;
use crate::AppState;

#[derive(Clone, Deserialize, Serialize)]
pub struct SimulateRequest {
    pub project_id: Option<Uuid>,
    pub scenario_id: Option<Uuid>,
    pub wind_speed: f64,
    pub wind_direction: f64,
    pub sun_elevation: f64,
    pub t_ambient: Option<f64>,
    pub geometry: Vec<GeometryBlock>,
}

async fn simulate(
    State(state): State<Arc<AppState>>,
    Json(req): Json<SimulateRequest>,
) -> Result<Json<SimulationResult>, AppError> {
    let t_ambient = req.t_ambient.unwrap_or(20.0);

    tracing::info!(
        "Simulation request: wind={} m/s, dir={}°, sun={}°, {} blocks",
        req.wind_speed,
        req.wind_direction,
        req.sun_elevation,
        req.geometry.len()
    );

    let cache_key = simulation_cache_key(&req);
    if let Some(cached) = state.cache.get(&cache_key) {
        tracing::debug!("Cache hit for key {}", cache_key);
        return Ok(Json(cached));
    }

    let tensor = preprocessor::preprocess_geometry(
        &req.geometry,
        req.wind_speed,
        req.wind_direction,
        req.sun_elevation,
        t_ambient,
    );
    let tensor_dyn: ArrayD<f32> = tensor.into_dyn();

    let t_infer = Instant::now();
    let mut output: Option<ArrayD<f32>> = None;
    let mut model_loaded = false;

    if let Some(url) = state.fno_infer_url.as_deref() {
        match fno_client::predict(url, &tensor_dyn).await {
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
        state.onnx.predict(tensor_dyn).await?
    } else {
        tracing::debug!("No FNO URL success and no ONNX — returning mock data");
        let mock = generate_mock_result(&req.geometry, t_ambient);
        state.cache.insert(&cache_key, mock.clone());
        return Ok(Json(mock));
    };

    let inference_time_ms = t_infer.elapsed().as_millis() as u64;

    let result = postprocessor::postprocess(&output, t_ambient, inference_time_ms, model_loaded);

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

fn mock_wind_samples(center_x: f64, center_y: f64) -> Vec<postprocessor::WindFieldSample> {
    let mut out = Vec::with_capacity(9);
    for dx in -1..=1 {
        for dy in -1..=1 {
            out.push(postprocessor::WindFieldSample {
                x: center_x + f64::from(dx),
                y: center_y + f64::from(dy),
                z: 0.0,
                vx: 1.0,
                vy: 0.35,
                vz: 0.0,
            });
        }
    }
    out
}

fn generate_mock_result(geometry: &[GeometryBlock], t_ambient: f64) -> SimulationResult {
    let surface_temperatures: Vec<_> = geometry
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

    let (cx, cy) = if surface_temperatures.is_empty() {
        (0.0, 0.0)
    } else {
        let sx: f64 = surface_temperatures.iter().map(|s| s.lon).sum();
        let sy: f64 = surface_temperatures.iter().map(|s| s.lat).sum();
        let n = surface_temperatures.len() as f64;
        (sx / n, sy / n)
    };
    let wind_field = mock_wind_samples(cx, cy);

    SimulationResult {
        metadata: postprocessor::ResultMetadata {
            grid_resolution: [256, 256, 64],
            wind_subsample: [64, 64, 16],
            num_surface_points: surface_temperatures.len(),
            num_wind_samples: wind_field.len(),
            inference_time_ms: 0,
            model_loaded: false,
            t_ambient,
            delta_t_range: [2.0, 2.0],
            wind_speed_range: [1.0, 1.0],
        },
        surface_temperatures,
        wind_field,
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
        };
        let mut b = a.clone();
        b.geometry.reverse();
        assert_eq!(simulation_cache_key(&a), simulation_cache_key(&b));
    }
}
