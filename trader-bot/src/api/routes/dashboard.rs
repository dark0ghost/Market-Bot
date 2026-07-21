use axum::{Json, extract::State};
use serde::Serialize;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::api::AppState;

#[derive(Serialize)]
pub struct StatusResponse {
    status: String,
    version: String,
    uptime_secs: u64,
    brokers_count: usize,
    strategies_count: usize,
    data_sources: Vec<String>,
}

#[derive(Serialize)]
pub struct BrokerInfo {
    name: String,
    kind: String,
    account_id: String,
}

#[derive(Serialize)]
pub struct DashboardData {
    total_balance: f64,
    available_balance: f64,
    total_pnl: f64,
    positions_count: usize,
    active_orders: usize,
    brokers: Vec<BrokerInfo>,
}

pub async fn index() -> &'static str {
    "AI Trade Bot Dashboard\n\nEndpoints:\n  /api/status\n  /api/portfolio\n  /api/strategies\n  /api/brokers\n  /api/health"
}

pub async fn status(State(state): State<Arc<Mutex<AppState>>>) -> Json<StatusResponse> {
    let guard = state.lock().await;
    let sources: Vec<String> = guard.data_sources.list_names();

    Json(StatusResponse {
        status: "running".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        uptime_secs: 0,
        brokers_count: guard.brokers.len(),
        strategies_count: guard.strategies.count(),
        data_sources: sources,
    })
}

pub async fn portfolio(State(state): State<Arc<Mutex<AppState>>>) -> Json<Vec<serde_json::Value>> {
    let guard = state.lock().await;
    let mut results = Vec::new();

    for broker in &guard.brokers {
        if let Ok(portfolio) = broker.portfolio().await {
            if let Ok(val) = serde_json::to_value(&portfolio) {
                results.push(val);
            }
        }
    }

    Json(results)
}

pub async fn strategies_list(State(state): State<Arc<Mutex<AppState>>>) -> Json<Vec<String>> {
    let guard = state.lock().await;
    Json(guard.strategies.list_names())
}

pub async fn brokers_list(State(state): State<Arc<Mutex<AppState>>>) -> Json<Vec<BrokerInfo>> {
    let guard = state.lock().await;
    let brokers: Vec<BrokerInfo> = guard
        .brokers
        .iter()
        .map(|b| BrokerInfo {
            name: b.name().to_string(),
            kind: format!("{:?}", b.broker_kind()),
            account_id: b.account_id().to_string(),
        })
        .collect();
    Json(brokers)
}
