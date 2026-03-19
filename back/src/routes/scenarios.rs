use axum::extract::{Path, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::Deserialize;
use std::sync::Arc;
use uuid::Uuid;

use crate::db;
use crate::error::AppError;
use crate::AppState;

#[derive(Deserialize)]
pub struct CreateScenarioRequest {
    pub name: String,
    pub geometry: serde_json::Value,
    pub metadata: Option<serde_json::Value>,
}

async fn create_scenario(
    State(state): State<Arc<AppState>>,
    Path(project_id): Path<Uuid>,
    Json(body): Json<CreateScenarioRequest>,
) -> Result<Json<db::Scenario>, AppError> {
    db::get_project(&state.pool, project_id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Project {} not found", project_id)))?;

    let scenario =
        db::create_scenario(&state.pool, project_id, &body.name, body.geometry, body.metadata)
            .await?;
    Ok(Json(scenario))
}

async fn list_scenarios(
    State(state): State<Arc<AppState>>,
    Path(project_id): Path<Uuid>,
) -> Result<Json<Vec<db::Scenario>>, AppError> {
    let scenarios = db::list_scenarios(&state.pool, project_id).await?;
    Ok(Json(scenarios))
}

async fn get_scenario(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> Result<Json<db::Scenario>, AppError> {
    db::get_scenario(&state.pool, id)
        .await?
        .map(Json)
        .ok_or_else(|| AppError::NotFound(format!("Scenario {} not found", id)))
}

async fn delete_scenario(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, AppError> {
    let deleted = db::delete_scenario(&state.pool, id).await?;
    if deleted {
        Ok(Json(serde_json::json!({"deleted": true})))
    } else {
        Err(AppError::NotFound(format!("Scenario {} not found", id)))
    }
}

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route(
            "/projects/{project_id}/scenarios",
            post(create_scenario).get(list_scenarios),
        )
        .route("/scenarios/{id}", get(get_scenario).delete(delete_scenario))
}
