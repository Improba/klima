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
    #[serde(alias = "surfaceType")]
    pub surface_type: String,
}

impl GeometryBlock {
    pub fn position(&self) -> [f64; 3] {
        [self.x, self.y, self.z]
    }
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

    // TODO: load ONNX model via `ort` and run real inference
    let n = req.geometry.len();
    let _positions: Vec<[f64; 3]> = req.geometry.iter().map(|b| b.position()).collect();
    let _surfaces: Vec<&str> = req.geometry.iter().map(|b| b.surface_type.as_str()).collect();
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
