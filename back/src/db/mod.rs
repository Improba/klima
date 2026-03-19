use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::postgres::PgPoolOptions;
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

// ── Models ──────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct Project {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct Scenario {
    pub id: Uuid,
    pub project_id: Uuid,
    pub name: String,
    pub geometry: sqlx::types::Json<serde_json::Value>,
    pub metadata: Option<sqlx::types::Json<serde_json::Value>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct Simulation {
    pub id: Uuid,
    pub project_id: Uuid,
    pub scenario_id: Option<Uuid>,
    pub params: sqlx::types::Json<serde_json::Value>,
    pub result: Option<Vec<u8>>,
    pub status: String,
    pub created_at: DateTime<Utc>,
}

// ── Pool & migrations ───────────────────────────────────────────────────────

pub async fn create_pool(database_url: &str) -> Result<PgPool> {
    let pool = PgPoolOptions::new()
        .max_connections(10)
        .connect(database_url)
        .await?;
    Ok(pool)
}

pub async fn run_migrations(pool: &PgPool) -> Result<()> {
    sqlx::raw_sql(
        "
        CREATE TABLE IF NOT EXISTS projects (
            id          UUID PRIMARY KEY,
            name        TEXT NOT NULL,
            description TEXT,
            created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
            updated_at  TIMESTAMPTZ NOT NULL DEFAULT now()
        );

        CREATE TABLE IF NOT EXISTS scenarios (
            id          UUID PRIMARY KEY,
            project_id  UUID NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
            name        TEXT NOT NULL,
            geometry    JSONB NOT NULL,
            metadata    JSONB,
            created_at  TIMESTAMPTZ NOT NULL DEFAULT now()
        );

        CREATE TABLE IF NOT EXISTS simulations (
            id          UUID PRIMARY KEY,
            project_id  UUID NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
            scenario_id UUID REFERENCES scenarios(id) ON DELETE SET NULL,
            params      JSONB NOT NULL,
            result      BYTEA,
            status      TEXT NOT NULL DEFAULT 'pending',
            created_at  TIMESTAMPTZ NOT NULL DEFAULT now()
        );
        ",
    )
    .execute(pool)
    .await?;

    // Existing DBs may have been created with `description NOT NULL`; keep the column optional.
    let _ = sqlx::query("ALTER TABLE projects ALTER COLUMN description DROP NOT NULL")
        .execute(pool)
        .await;

    Ok(())
}

// ── Projects CRUD ───────────────────────────────────────────────────────────

pub async fn create_project(
    pool: &PgPool,
    name: &str,
    description: Option<&str>,
) -> std::result::Result<Project, sqlx::Error> {
    let id = Uuid::new_v4();
    sqlx::query_as::<_, Project>(
        "INSERT INTO projects (id, name, description) VALUES ($1, $2, $3) RETURNING *",
    )
    .bind(id)
    .bind(name)
    .bind(description)
    .fetch_one(pool)
    .await
}

pub async fn get_project(
    pool: &PgPool,
    id: Uuid,
) -> std::result::Result<Option<Project>, sqlx::Error> {
    sqlx::query_as::<_, Project>("SELECT * FROM projects WHERE id = $1")
        .bind(id)
        .fetch_optional(pool)
        .await
}

pub async fn list_projects(
    pool: &PgPool,
    limit: i64,
    offset: i64,
) -> std::result::Result<Vec<Project>, sqlx::Error> {
    sqlx::query_as::<_, Project>(
        "SELECT * FROM projects ORDER BY created_at DESC LIMIT $1 OFFSET $2",
    )
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await
}

pub async fn update_project(
    pool: &PgPool,
    id: Uuid,
    name: &str,
    description: Option<&str>,
) -> std::result::Result<Option<Project>, sqlx::Error> {
    sqlx::query_as::<_, Project>(
        "UPDATE projects SET name = $2, description = $3, updated_at = now() \
         WHERE id = $1 RETURNING *",
    )
    .bind(id)
    .bind(name)
    .bind(description)
    .fetch_optional(pool)
    .await
}

pub async fn delete_project(pool: &PgPool, id: Uuid) -> std::result::Result<bool, sqlx::Error> {
    let result = sqlx::query("DELETE FROM projects WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(result.rows_affected() > 0)
}

// ── Scenarios CRUD ──────────────────────────────────────────────────────────

pub async fn create_scenario(
    pool: &PgPool,
    project_id: Uuid,
    name: &str,
    geometry: serde_json::Value,
    metadata: Option<serde_json::Value>,
) -> std::result::Result<Scenario, sqlx::Error> {
    let id = Uuid::new_v4();
    sqlx::query_as::<_, Scenario>(
        "INSERT INTO scenarios (id, project_id, name, geometry, metadata) \
         VALUES ($1, $2, $3, $4, $5) RETURNING *",
    )
    .bind(id)
    .bind(project_id)
    .bind(name)
    .bind(sqlx::types::Json(&geometry))
    .bind(metadata.map(sqlx::types::Json))
    .fetch_one(pool)
    .await
}

pub async fn list_scenarios(
    pool: &PgPool,
    project_id: Uuid,
) -> std::result::Result<Vec<Scenario>, sqlx::Error> {
    sqlx::query_as::<_, Scenario>(
        "SELECT * FROM scenarios WHERE project_id = $1 ORDER BY created_at DESC",
    )
    .bind(project_id)
    .fetch_all(pool)
    .await
}

pub async fn get_scenario(
    pool: &PgPool,
    id: Uuid,
) -> std::result::Result<Option<Scenario>, sqlx::Error> {
    sqlx::query_as::<_, Scenario>("SELECT * FROM scenarios WHERE id = $1")
        .bind(id)
        .fetch_optional(pool)
        .await
}

pub async fn delete_scenario(pool: &PgPool, id: Uuid) -> std::result::Result<bool, sqlx::Error> {
    let result = sqlx::query("DELETE FROM scenarios WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(result.rows_affected() > 0)
}

// ── Simulations CRUD ────────────────────────────────────────────────────────

pub async fn create_simulation(
    pool: &PgPool,
    project_id: Uuid,
    scenario_id: Option<Uuid>,
    params: serde_json::Value,
) -> std::result::Result<Simulation, sqlx::Error> {
    let id = Uuid::new_v4();
    sqlx::query_as::<_, Simulation>(
        "INSERT INTO simulations (id, project_id, scenario_id, params) \
         VALUES ($1, $2, $3, $4) RETURNING *",
    )
    .bind(id)
    .bind(project_id)
    .bind(scenario_id)
    .bind(sqlx::types::Json(&params))
    .fetch_one(pool)
    .await
}

pub async fn get_simulation(
    pool: &PgPool,
    id: Uuid,
) -> std::result::Result<Option<Simulation>, sqlx::Error> {
    sqlx::query_as::<_, Simulation>("SELECT * FROM simulations WHERE id = $1")
        .bind(id)
        .fetch_optional(pool)
        .await
}

pub async fn list_simulations(
    pool: &PgPool,
    project_id: Uuid,
    limit: i64,
    offset: i64,
) -> std::result::Result<Vec<Simulation>, sqlx::Error> {
    sqlx::query_as::<_, Simulation>(
        "SELECT id, project_id, scenario_id, params, result, status, created_at \
         FROM simulations WHERE project_id = $1 ORDER BY created_at DESC LIMIT $2 OFFSET $3",
    )
    .bind(project_id)
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await
}

pub async fn update_simulation_result(
    pool: &PgPool,
    id: Uuid,
    result_bytes: &[u8],
    status: &str,
) -> std::result::Result<bool, sqlx::Error> {
    let result = sqlx::query(
        "UPDATE simulations SET result = $2, status = $3 WHERE id = $1",
    )
    .bind(id)
    .bind(result_bytes)
    .bind(status)
    .execute(pool)
    .await?;
    Ok(result.rows_affected() > 0)
}
