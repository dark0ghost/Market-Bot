use anyhow::Result;
use crate::execution::position_manager::{OrderAction, OrderResult, PositionManager};

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

    async fn get_orders(&self) -> Result<Vec<OrderResult>> {
        self.get_orders().await
    }
}
