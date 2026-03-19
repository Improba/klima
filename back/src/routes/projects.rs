use axum::extract::{Path, Query, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::Deserialize;
use std::sync::Arc;
use uuid::Uuid;

use crate::db;
use crate::error::AppError;
use crate::AppState;

#[derive(Deserialize)]
pub struct CreateProjectRequest {
    pub name: String,
    pub description: Option<String>,
}

#[derive(Deserialize)]
pub struct UpdateProjectRequest {
    pub name: String,
    pub description: Option<String>,
}

#[derive(Deserialize)]
pub struct ListParams {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

async fn create_project(
    State(state): State<Arc<AppState>>,
    Json(body): Json<CreateProjectRequest>,
) -> Result<Json<db::Project>, AppError> {
    let project = db::create_project(&state.pool, &body.name, body.description.as_deref()).await?;
    Ok(Json(project))
}

async fn list_projects(
    State(state): State<Arc<AppState>>,
    Query(params): Query<ListParams>,
) -> Result<Json<Vec<db::Project>>, AppError> {
    let limit = params.limit.unwrap_or(50).min(200);
    let offset = params.offset.unwrap_or(0);
    let projects = db::list_projects(&state.pool, limit, offset).await?;
    Ok(Json(projects))
}

async fn get_project(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> Result<Json<db::Project>, AppError> {
    db::get_project(&state.pool, id)
        .await?
        .map(Json)
        .ok_or_else(|| AppError::NotFound(format!("Project {} not found", id)))
}

async fn update_project(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
    Json(body): Json<UpdateProjectRequest>,
) -> Result<Json<db::Project>, AppError> {
    db::update_project(&state.pool, id, &body.name, body.description.as_deref())
        .await?
        .map(Json)
        .ok_or_else(|| AppError::NotFound(format!("Project {} not found", id)))
}

async fn delete_project(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, AppError> {
    let deleted = db::delete_project(&state.pool, id).await?;
    if deleted {
        Ok(Json(serde_json::json!({"deleted": true})))
    } else {
        Err(AppError::NotFound(format!("Project {} not found", id)))
    }
}

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/projects", post(create_project).get(list_projects))
        .route(
            "/projects/{id}",
            get(get_project).put(update_project).delete(delete_project),
        )
}
