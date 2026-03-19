use anyhow::Result;
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;

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

    Ok(())
}
