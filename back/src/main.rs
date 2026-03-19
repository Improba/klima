mod cache;
mod db;
mod error;
mod inference;
mod routes;

use axum::Router;
use sqlx::PgPool;
use std::net::SocketAddr;
use std::sync::Arc;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;

use cache::SimulationCache;
use inference::OnnxService;

pub struct AppState {
    pub pool: PgPool,
    pub onnx: OnnxService,
    pub cache: SimulationCache,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "klima_api=debug,tower_http=debug".into()),
        )
        .init();

    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://klima:klima@localhost:5432/klima".into());

    tracing::info!("Connecting to PostgreSQL...");
    let pool = db::create_pool(&database_url).await?;
    db::run_migrations(&pool).await?;
    tracing::info!("Database ready");

    let model_path = std::env::var("KLIMA_MODEL_PATH").ok();
    let norm_path = std::env::var("KLIMA_NORM_PATH").ok();
    let onnx = OnnxService::new(model_path.as_deref(), norm_path.as_deref());
    if onnx.is_loaded() {
        tracing::info!("ONNX inference service ready");
    } else {
        tracing::warn!("No ONNX model loaded — simulate endpoint will return mock data");
    }

    let cache_capacity: usize = std::env::var("KLIMA_CACHE_SIZE")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(128);
    let cache = SimulationCache::new(cache_capacity);

    let state = Arc::new(AppState { pool, onnx, cache });

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        .nest("/api", routes::api_router())
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    tracing::info!("Klima API listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
