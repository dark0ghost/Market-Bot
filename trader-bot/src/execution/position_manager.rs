use crate::core::{OrderAction, OrderStatus};
use crate::provider::ExecutionProvider;
use anyhow::Result;
use chrono::{DateTime, Utc};
use t_invest_sdk::TInvestSdk;
use t_invest_sdk::api::{
    GetOrdersRequest, OrderDirection, OrderExecutionReportStatus, OrderType, PostOrderRequest,
};

/// Order placement result
#[derive(Debug, Clone)]
pub struct OrderResult {
    pub order_id: String,
    pub figi: String,
    pub action: OrderAction,
    pub quantity: i32,
    pub price: Option<f64>,
    pub status: OrderStatus,
    pub created_at: DateTime<Utc>,
    pub message: String,
}

/// Position and order manager
pub struct PositionManager {
    sdk: TInvestSdk,
    account_id: String,
}

/// Build a Tinkoff `PostOrderRequest` - single source of truth for order placement
/// (was duplicated in `place_limit_order`, `place_market_order`, and `main.rs`).
#[allow(deprecated)]
fn build_post_order_request(
    figi: &str,
    action: OrderAction,
    quantity: i32,
    price: Option<f64>,
    order_type: OrderType,
    account_id: &str,
) -> PostOrderRequest {
    let direction = match action {
        OrderAction::Buy => OrderDirection::Buy,
        OrderAction::Sell => OrderDirection::Sell,
    };
    PostOrderRequest {
        figi: Some(figi.to_string()),
        quantity: quantity as i64,
        price: price.map(|p| t_invest_sdk::api::Quotation {
            units: p as i64,
            nano: ((p.fract() * 1_000_000_000.0) as i32),
        }),
        direction: direction as i32,
        account_id: account_id.to_string(),
        order_type: order_type as i32,
        order_id: format!(
            "order_{}_{}",
            Utc::now().timestamp_nanos_opt().unwrap_or(0),
            std::process::id()
        ),
        instrument_id: figi.to_string(),
        confirm_margin_trade: false,
        time_in_force: 0,
        price_type: 0,
    }
}

impl PositionManager {
    pub fn new(sdk: TInvestSdk, account_id: String) -> Self {
        PositionManager { sdk, account_id }
    }

    /// Place limit order
    pub async fn place_limit_order(
        &self,
        figi: &str,
        action: OrderAction,
        quantity: i32,
        price: f64,
    ) -> Result<OrderResult> {
        let request = build_post_order_request(
            figi,
            action.clone(),
            quantity,
            Some(price),
            OrderType::Limit,
            &self.account_id,
        );

        let response = self.sdk.orders().post_order(request).await?;
        let order_response = response.into_inner();

        Ok(OrderResult {
            order_id: order_response.order_id,
            figi: order_response.figi,
            action,
            quantity: order_response.lots_executed as i32,
            price: Some(price),
            status: OrderStatus::New,
            created_at: Utc::now(),
            message: "Order placed".to_string(),
        })
    }

    /// Place market order
    pub async fn place_market_order(
        &self,
        figi: &str,
        action: OrderAction,
        quantity: i32,
    ) -> Result<OrderResult> {
        let request = build_post_order_request(
            figi,
            action.clone(),
            quantity,
            None,
            OrderType::Market,
            &self.account_id,
        );

        let response = self.sdk.orders().post_order(request).await?;
        let order_response = response.into_inner();

        Ok(OrderResult {
            order_id: order_response.order_id,
            figi: order_response.figi,
            action,
            quantity: order_response.lots_executed as i32,
            price: None,
            status: OrderStatus::New,
            created_at: Utc::now(),
            message: "Market order placed".to_string(),
        })
    }

    /// Place a broker-side stop order (StopLoss or TakeProfit). Tinkoff executes it
    /// server-side, so the stop survives a bot crash - unlike an in-memory stop.
    pub async fn place_stop_order(
        &self,
        figi: &str,
        action: OrderAction,
        quantity: i32,
        stop_price: f64,
        limit_price: Option<f64>,
        kind: crate::core::StopOrderKind,
    ) -> Result<OrderResult> {
        use t_invest_sdk::api::*;

        let direction = match action {
            OrderAction::Buy => StopOrderDirection::Buy,
            OrderAction::Sell => StopOrderDirection::Sell,
        };
        let stop_order_type = match kind {
            crate::core::StopOrderKind::StopLoss => StopOrderType::StopLoss,
            crate::core::StopOrderKind::TakeProfit => StopOrderType::TakeProfit,
        };

        #[allow(deprecated)]
        let req = PostStopOrderRequest {
            figi: Some(figi.to_string()),
            quantity: quantity as i64,
            price: limit_price.map(|p| Quotation {
                units: p as i64,
                nano: ((p.fract() * 1_000_000_000.0) as i32),
            }),
            stop_price: Some(Quotation {
                units: stop_price as i64,
                nano: ((stop_price.fract() * 1_000_000_000.0) as i32),
            }),
            direction: direction as i32,
            account_id: self.account_id.clone(),
            expiration_type: StopOrderExpirationType::GoodTillCancel as i32,
            stop_order_type: stop_order_type as i32,
            expire_date: None,
            instrument_id: figi.to_string(),
            exchange_order_type: 0,
            take_profit_type: 0,
            trailing_data: None,
            price_type: 0,
            order_id: format!(
                "stop_{}_{}",
                Utc::now().timestamp_nanos_opt().unwrap_or(0),
                std::process::id()
            ),
            confirm_margin_trade: false,
            instant_execution: None,
        };

        let response = self.sdk.stop_orders().post_stop_order(req).await?;
        let r = response.into_inner();

        Ok(OrderResult {
            order_id: r.stop_order_id,
            figi: figi.to_string(),
            action,
            quantity,
            price: limit_price,
            status: OrderStatus::New,
            created_at: Utc::now(),
            message: format!("Stop {:?} placed", kind),
        })
    }

    /// Cancel an order by its broker-assigned id.
    pub async fn cancel_order(&self, order_id: &str) -> Result<()> {
        use t_invest_sdk::api::CancelOrderRequest;
        let req = CancelOrderRequest {
            account_id: self.account_id.clone(),
            order_id: order_id.to_string(),
            order_id_type: Some(0),
        };
        self.sdk.orders().cancel_order(req).await?;
        Ok(())
    }

    /// Get order list
    pub async fn get_orders(&self) -> Result<Vec<OrderResult>> {
        let request = GetOrdersRequest {
            account_id: self.account_id.clone(),
            advanced_filters: None,
        };

        let response = self.sdk.orders().get_orders(request).await?;
        let orders_response = response.into_inner();

        let mut results = Vec::new();
        for order in orders_response.orders {
            // Map the real direction instead of hardcoding Buy.
            let action = match OrderDirection::try_from(order.direction)
                .unwrap_or(OrderDirection::Unspecified)
            {
                OrderDirection::Sell => OrderAction::Sell,
                _ => OrderAction::Buy,
            };
            // Map the real execution status instead of hardcoding New.
            let status = map_tinkoff_status(order.execution_report_status);
            results.push(OrderResult {
                order_id: order.order_id,
                figi: order.figi,
                action,
                quantity: order.lots_requested as i32,
                price: None,
                status,
                created_at: Utc::now(),
                message: "Order from list".to_string(),
            });
        }

        Ok(results)
    }
}

/// Map a Tinkoff `OrderExecutionReportStatus` into our `OrderStatus`.
fn map_tinkoff_status(status: i32) -> OrderStatus {
    match OrderExecutionReportStatus::try_from(status)
        .unwrap_or(OrderExecutionReportStatus::ExecutionReportStatusUnspecified)
    {
        OrderExecutionReportStatus::ExecutionReportStatusFill => OrderStatus::Filled,
        OrderExecutionReportStatus::ExecutionReportStatusRejected => OrderStatus::Rejected,
        OrderExecutionReportStatus::ExecutionReportStatusCancelled => OrderStatus::Cancelled,
        OrderExecutionReportStatus::ExecutionReportStatusNew => OrderStatus::New,
        OrderExecutionReportStatus::ExecutionReportStatusPartiallyfill => {
            OrderStatus::PartiallyFilled
        }
        OrderExecutionReportStatus::ExecutionReportStatusUnspecified => OrderStatus::New,
    }
}

/// Extended manager for working with agent decisions
pub struct TradingExecutor<E: ExecutionProvider> {
    executor: E,
    available_balance: f64,
}

impl<E: ExecutionProvider> TradingExecutor<E> {
    pub fn new(executor: E, available_balance: f64) -> Self {
        TradingExecutor {
            executor,
            available_balance,
        }
    }

    /// Update available balance
    pub const fn update_balance(&mut self, balance: f64) {
        self.available_balance = balance;
    }

    /// Execute trading decision
    pub async fn execute_decision(
        &self,
        decision: &crate::agent::TradingDecision,
        instrument_uid: &str,
    ) -> Result<Vec<OrderResult>> {
        use crate::agent::Action;

        let mut results = Vec::new();

        match decision.action {
            Action::Buy => {
                if let Some(entry_price) = decision.entry_price {
                    let quantity = self.calculate_quantity(entry_price, decision.position_size_pct);

                    if quantity > 0 {
                        log::info!(
                            "Placing BUY order: {} lots at price {:.2} (total: {:.2})",
                            quantity,
                            entry_price,
                            quantity as f64 * entry_price
                        );

                        let order_result = self
                            .executor
                            .place_limit_order(
                                instrument_uid,
                                OrderAction::Buy,
                                quantity,
                                entry_price,
                            )
                            .await?;
                        results.push(order_result);

                        // Place broker-side SL/TP so stops survive a crash.
                        // A long BUY is protected by a Sell stop: SL below entry, TP above.
                        self.place_protection_orders(
                            instrument_uid,
                            OrderAction::Sell,
                            quantity,
                            decision.stop_loss,
                            decision.take_profit,
                        )
                        .await;
                    } else {
                        log::warn!("Calculated lot count is 0");
                    }
                }
            }
            Action::Sell => {
                if let Some(current_position) = decision.current_position {
                    // For selling, use current position
                    let quantity =
                        self.calculate_sell_quantity(current_position, decision.position_size_pct);

                    if quantity > 0 {
                        // Use current price for order
                        let price = decision.current_price;
                        log::info!(
                            "Placing SELL order: {} lots at price {:.2}",
                            quantity,
                            price
                        );

                        let order_result = self
                            .executor
                            .place_limit_order(instrument_uid, OrderAction::Sell, quantity, price)
                            .await?;
                        results.push(order_result);

                        // A short SELL is protected by a Buy stop: SL above entry, TP below.
                        self.place_protection_orders(
                            instrument_uid,
                            OrderAction::Buy,
                            quantity,
                            decision.stop_loss,
                            decision.take_profit,
                        )
                        .await;
                    }
                } else {
                    log::warn!("Cannot sell: current position not specified");
                }
            }
            Action::Hold => {
                log::info!("Decision for {}: HOLD - no action taken", decision.ticker);
            }
        }

        Ok(results)
    }

    /// Place broker-side StopLoss and TakeProfit for the given (already filled) entry.
    /// Errors are logged, not propagated - a failed stop placement must not roll back the entry,
    /// but the caller's in-memory tracker still keeps a fallback stop.
    async fn place_protection_orders(
        &self,
        instrument_uid: &str,
        close_action: OrderAction,
        quantity: i32,
        stop_loss: Option<f64>,
        take_profit: Option<f64>,
    ) {
        if let Some(sl_price) = stop_loss {
            log::info!(
                "Placing Stop Loss: {} lots @ {:.2} ({:?})",
                quantity,
                sl_price,
                close_action
            );
            if let Err(e) = self
                .executor
                .place_stop_order(
                    instrument_uid,
                    close_action.clone(),
                    quantity,
                    sl_price,
                    Some(sl_price), // stop-limit at the same price
                    crate::core::StopOrderKind::StopLoss,
                )
                .await
            {
                log::warn!(
                    "Broker-side stop-loss rejected (in-memory fallback active): {}",
                    e
                );
            }
        }
        if let Some(tp_price) = take_profit {
            log::info!(
                "Placing Take Profit: {} lots @ {:.2} ({:?})",
                quantity,
                tp_price,
                close_action
            );
            if let Err(e) = self
                .executor
                .place_stop_order(
                    instrument_uid,
                    close_action,
                    quantity,
                    tp_price,
                    Some(tp_price),
                    crate::core::StopOrderKind::TakeProfit,
                )
                .await
            {
                log::warn!(
                    "Broker-side take-profit rejected (in-memory fallback active): {}",
                    e
                );
            }
        }
    }

    /// Calculate number of lots to buy
    const fn calculate_quantity(&self, price: f64, position_pct: f64) -> i32 {
        if price <= 0.0 || position_pct <= 0.0 {
            return 0;
        }

        // Use actual available balance
        let position_value = self.available_balance * position_pct;
        let quantity = (position_value / price) as i32;

        if quantity < 0 { 0 } else { quantity }
    }

    /// Calculate number of lots to sell
    ///
    /// # Arguments
    /// * `current_position` - Current position in lots
    /// * `position_pct` - Position percentage to sell (0.0-1.0)
    ///
    /// # Returns
    /// Number of lots to sell
    const fn calculate_sell_quantity(&self, current_position: i32, position_pct: f64) -> i32 {
        if position_pct <= 0.0 || current_position <= 0 {
            return 0;
        }

        let quantity = (current_position as f64 * position_pct) as i32;
        if quantity < 1 {
            1
        } else if quantity > current_position {
            current_position
        } else {
            quantity
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_order_action_debug() {
        assert_eq!(format!("{:?}", OrderAction::Buy), "Buy");
        assert_eq!(format!("{:?}", OrderAction::Sell), "Sell");
    }

    #[test]
    fn test_order_status_partial_eq() {
        assert_eq!(OrderStatus::New, OrderStatus::New);
        assert_eq!(OrderStatus::Filled, OrderStatus::Filled);
        assert_ne!(OrderStatus::New, OrderStatus::Filled);
    }

    #[test]
    fn test_order_result_clone() {
        let order = OrderResult {
            order_id: "test_123".to_string(),
            figi: "BBG000B9XRY4".to_string(),
            action: OrderAction::Buy,
            quantity: 10,
            price: Some(150.50),
            status: OrderStatus::New,
            created_at: Utc::now(),
            message: "Test order".to_string(),
        };

        let cloned = order.clone();
        assert_eq!(cloned.order_id, order.order_id);
        assert_eq!(cloned.figi, order.figi);
        assert_eq!(cloned.action, order.action);
        assert_eq!(cloned.quantity, order.quantity);
        assert_eq!(cloned.price, order.price);
        assert_eq!(cloned.status, order.status);
    }
}

/// Tests for TradingExecutor
#[cfg(test)]
mod executor_tests {
    use super::*;
    use crate::provider::ExecutionProvider;
    use anyhow::Result;

    struct MockProvider;

    impl ExecutionProvider for MockProvider {
        async fn place_limit_order(
            &self,
            _figi: &str,
            action: OrderAction,
            quantity: i32,
            price: f64,
        ) -> Result<OrderResult> {
            Ok(OrderResult {
                order_id: "mock_1".to_string(),
                figi: "MOCK".to_string(),
                action,
                quantity,
                price: Some(price),
                status: OrderStatus::New,
                created_at: Utc::now(),
                message: "mock".to_string(),
            })
        }

        async fn place_market_order(
            &self,
            _figi: &str,
            action: OrderAction,
            quantity: i32,
        ) -> Result<OrderResult> {
            Ok(OrderResult {
                order_id: "mock_2".to_string(),
                figi: "MOCK".to_string(),
                action,
                quantity,
                price: None,
                status: OrderStatus::New,
                created_at: Utc::now(),
                message: "mock".to_string(),
            })
        }

        async fn get_orders(&self) -> Result<Vec<OrderResult>> {
            Ok(vec![])
        }
    }

    #[test]
    fn test_calculate_quantity_basic() {
        let executor = TradingExecutor::new(MockProvider, 10000.0);
        assert_eq!(executor.calculate_quantity(100.0, 0.1), 10);
        assert_eq!(executor.calculate_quantity(50.0, 0.1), 20);
        assert_eq!(executor.calculate_quantity(100.0, 0.5), 50);
    }

    #[test]
    fn test_calculate_quantity_edge_cases() {
        let executor = TradingExecutor::new(MockProvider, 10000.0);
        assert_eq!(executor.calculate_quantity(0.0, 0.1), 0);
        assert_eq!(executor.calculate_quantity(-100.0, 0.1), 0);
        assert_eq!(executor.calculate_quantity(100.0, 0.0), 0);
        assert_eq!(executor.calculate_quantity(100.0, -0.1), 0);
    }

    #[test]
    fn test_calculate_sell_quantity_basic() {
        let executor = TradingExecutor::new(MockProvider, 10000.0);
        assert_eq!(executor.calculate_sell_quantity(100, 0.5), 50);
        assert_eq!(executor.calculate_sell_quantity(100, 0.25), 25);
        assert_eq!(executor.calculate_sell_quantity(100, 1.0), 100);
    }

    #[test]
    fn test_calculate_sell_quantity_minimum() {
        let executor = TradingExecutor::new(MockProvider, 10000.0);
        assert_eq!(executor.calculate_sell_quantity(100, 0.01), 1);
        assert_eq!(executor.calculate_sell_quantity(100, 0.001), 1);
    }

    #[test]
    fn test_calculate_sell_quantity_edge_cases() {
        let executor = TradingExecutor::new(MockProvider, 10000.0);
        assert_eq!(executor.calculate_sell_quantity(0, 0.5), 0);
        assert_eq!(executor.calculate_sell_quantity(-10, 0.5), 0);
        assert_eq!(executor.calculate_sell_quantity(100, 0.0), 0);
        assert_eq!(executor.calculate_sell_quantity(100, -0.5), 0);
    }

    #[test]
    fn test_update_balance() {
        let mut executor = TradingExecutor::new(MockProvider, 10000.0);
        executor.update_balance(20000.0);
        assert_eq!(executor.available_balance, 20000.0);
    }
}
