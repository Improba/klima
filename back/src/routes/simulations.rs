use axum::extract::{Path, State};
use axum::routing::get;
use axum::{Json, Router};
use std::sync::Arc;
use uuid::Uuid;

use crate::db;
use crate::error::AppError;
use crate::AppState;

async fn get_simulation(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> Result<Json<db::Simulation>, AppError> {
    db::get_simulation(&state.pool, id)
        .await?
        .map(Json)
        .ok_or_else(|| AppError::NotFound(format!("Simulation {} not found", id)))
}

pub fn router() -> Router<Arc<AppState>> {
    Router::new().route("/simulations/{id}", get(get_simulation))
}
