pub mod routes;

use axum::{Router, routing::get};
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::core::Broker;
use crate::datasource::DataSourceRegistry;
use crate::strategy::registry::StrategyRegistry;

/// Shared application state for the API.
pub struct AppState {
    pub brokers: Vec<Arc<dyn Broker>>,
    pub data_sources: DataSourceRegistry,
    pub strategies: StrategyRegistry,
}

/// Start the embedded Web dashboard.
pub async fn start_dashboard(state: Arc<Mutex<AppState>>, port: u16) -> anyhow::Result<()> {
    let app = Router::new()
        .route("/", get(routes::dashboard::index))
        .route("/api/status", get(routes::dashboard::status))
        .route("/api/portfolio", get(routes::dashboard::portfolio))
        .route("/api/strategies", get(routes::dashboard::strategies_list))
        .route("/api/brokers", get(routes::dashboard::brokers_list))
        .route("/api/health", get(health))
        .with_state(state);

    let addr = format!("0.0.0.0:{}", port);
    log::info!("Dashboard starting on http://{}", addr);

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

/// Health check endpoint.
async fn health() -> &'static str {
    "OK"
}
