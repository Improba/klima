pub mod health;
pub mod simulate;

use axum::Router;
use std::sync::Arc;

use crate::AppState;

pub fn api_router() -> Router<Arc<AppState>> {
    Router::new()
        .merge(health::router())
        .merge(simulate::router())
}
