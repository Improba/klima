use axum::{extract::State, routing::post, Json, Router};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::AppState;

#[derive(Deserialize)]
pub struct SimulateRequest {
    pub wind_speed: f64,
    pub wind_direction: f64,
    pub sun_elevation: f64,
    pub geometry: Vec<GeometryBlock>,
}

#[derive(Deserialize)]
pub struct GeometryBlock {
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub surface_type: String,
}

#[derive(Serialize)]
pub struct SimulateResponse {
    pub status: String,
    pub message: String,
    pub temperatures: Vec<f64>,
    pub wind_vectors: Vec<[f64; 3]>,
}

async fn simulate(
    State(_state): State<Arc<AppState>>,
    Json(req): Json<SimulateRequest>,
) -> Json<SimulateResponse> {
    tracing::info!(
        "Simulation request: wind={} m/s, dir={}°, sun={}°, {} blocks",
        req.wind_speed,
        req.wind_direction,
        req.sun_elevation,
        req.geometry.len()
    );

    // TODO: Load ONNX model via `ort` and run inference
    // For now, return a placeholder response
    let n = req.geometry.len();
    Json(SimulateResponse {
        status: "ok".into(),
        message: "Surrogate model not yet loaded — returning placeholder data".into(),
        temperatures: vec![25.0; n],
        wind_vectors: vec![[0.0, 0.0, 0.0]; n],
    })
}

pub fn router() -> Router<Arc<AppState>> {
    Router::new().route("/simulate", post(simulate))
}
