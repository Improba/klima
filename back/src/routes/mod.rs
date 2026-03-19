pub mod health;
pub mod projects;
pub mod scenarios;
pub mod simulate;
pub mod simulations;

use axum::Router;
use std::sync::Arc;

use crate::AppState;

pub fn api_router() -> Router<Arc<AppState>> {
    Router::new()
        .merge(health::router())
        .merge(simulate::router())
        .merge(projects::router())
        .merge(scenarios::router())
        .merge(simulations::router())
}
