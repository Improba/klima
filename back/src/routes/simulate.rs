use axum::extract::State;
use axum::routing::post;
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::sync::Arc;
use std::time::Instant;
use uuid::Uuid;

use crate::db;
use crate::error::AppError;
use crate::inference::postprocessor::{self, SimulationResult};
use crate::inference::preprocessor::{self, GeometryBlock};
use crate::AppState;

#[derive(Deserialize, Serialize)]
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

    let cache_key = compute_cache_key(&req);
    if let Some(cached) = state.cache.get(&cache_key) {
        tracing::debug!("Cache hit for key {}", cache_key);
        return Ok(Json(cached));
    }

    let model_loaded = state.onnx.is_loaded();

    if !model_loaded {
        tracing::debug!("No ONNX model loaded — returning mock data");
        let mock = generate_mock_result(&req.geometry, t_ambient);
        state.cache.insert(&cache_key, mock.clone());
        return Ok(Json(mock));
    }

    let tensor = preprocessor::preprocess_geometry(
        &req.geometry,
        req.wind_speed,
        req.wind_direction,
        req.sun_elevation,
        t_ambient,
    );

    let start = Instant::now();
    let output = state.onnx.predict(tensor.into_dyn()).await?;
    let inference_time_ms = start.elapsed().as_millis() as u64;

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

fn compute_cache_key(req: &SimulateRequest) -> String {
    let mut hasher = DefaultHasher::new();
    let json = serde_json::to_string(req).unwrap_or_default();
    json.hash(&mut hasher);
    format!("{:x}", hasher.finish())
}

fn generate_mock_result(geometry: &[GeometryBlock], t_ambient: f64) -> SimulationResult {
    let surface_temperatures: Vec<_> = geometry
        .iter()
        .map(|b| postprocessor::SurfaceTemperature {
            lon: b.x,
            lat: b.y,
            alt: b.z,
            temperature: t_ambient + 2.0,
        })
        .collect();

    let wind_field = vec![postprocessor::WindFieldSample {
        x: 0.0,
        y: 0.0,
        z: 0.0,
        vx: 1.0,
        vy: 0.0,
        vz: 0.0,
    }];

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
