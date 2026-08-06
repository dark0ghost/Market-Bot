use crate::core::{OrderAction, StopOrderKind};
use crate::execution::position_manager::{OrderResult, PositionManager};
use anyhow::Result;

pub trait ExecutionProvider {
    async fn place_limit_order(
        &self,
        figi: &str,
        action: OrderAction,
        quantity: i32,
        price: f64,
    ) -> Result<OrderResult>;

    async fn place_market_order(
        &self,
        figi: &str,
        action: OrderAction,
        quantity: i32,
    ) -> Result<OrderResult>;

    /// Place a broker-side stop order. Default returns an error for providers
    /// that don't support native stops - callers fall back to in-memory tracking.
    async fn place_stop_order(
        &self,
        _figi: &str,
        _action: OrderAction,
        _quantity: i32,
        _stop_price: f64,
        _limit_price: Option<f64>,
        _kind: StopOrderKind,
    ) -> Result<OrderResult> {
        Err(anyhow::anyhow!(
            "stop orders not supported by this provider"
        ))
    }

    async fn get_orders(&self) -> Result<Vec<OrderResult>>;
}

impl ExecutionProvider for PositionManager {
    async fn place_limit_order(
        &self,
        figi: &str,
        action: OrderAction,
        quantity: i32,
        price: f64,
    ) -> Result<OrderResult> {
        self.place_limit_order(figi, action, quantity, price).await
    }

    async fn place_market_order(
        &self,
        figi: &str,
        action: OrderAction,
        quantity: i32,
    ) -> Result<OrderResult> {
        self.place_market_order(figi, action, quantity).await
    }

    async fn place_stop_order(
        &self,
        figi: &str,
        action: OrderAction,
        quantity: i32,
        stop_price: f64,
        limit_price: Option<f64>,
        kind: StopOrderKind,
    ) -> Result<OrderResult> {
        self.place_stop_order(figi, action, quantity, stop_price, limit_price, kind)
            .await
    }

    async fn get_orders(&self) -> Result<Vec<OrderResult>> {
        self.get_orders().await
    }
}
