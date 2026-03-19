use axum::extract::{Path, Query, State};
use axum::routing::get;
use axum::{Json, Router};
use serde::Deserialize;
use std::sync::Arc;
use uuid::Uuid;

use crate::db;
use crate::error::AppError;
use crate::AppState;

#[derive(Deserialize)]
pub struct ListQuery {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

async fn get_simulation(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> Result<Json<db::Simulation>, AppError> {
    db::get_simulation(&state.pool, id)
        .await?
        .map(Json)
        .ok_or_else(|| AppError::NotFound(format!("Simulation {} not found", id)))
}

async fn list_simulations(
    State(state): State<Arc<AppState>>,
    Path(project_id): Path<Uuid>,
    Query(query): Query<ListQuery>,
) -> Result<Json<Vec<db::Simulation>>, AppError> {
    let limit = query.limit.unwrap_or(50);
    let offset = query.offset.unwrap_or(0);
    let simulations = db::list_simulations(&state.pool, project_id, limit, offset).await?;
    Ok(Json(simulations))
}

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/simulations/{id}", get(get_simulation))
        .route(
            "/projects/{project_id}/simulations",
            get(list_simulations),
        )
}
