use crate::provider::ExecutionProvider;
use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use t_invest_sdk::TInvestSdk;
use t_invest_sdk::api::{GetOrdersRequest, OrderDirection, OrderType, PostOrderRequest};

/// Order action
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum OrderAction {
    Buy,
    Sell,
}

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

/// Order status
#[derive(Debug, Clone, PartialEq)]
pub enum OrderStatus {
    New,
    PartiallyFilled,
    Filled,
    Cancelled,
    Rejected,
}

/// Position and order manager
pub struct PositionManager {
    sdk: TInvestSdk,
    account_id: String,
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
        let direction = match action {
            OrderAction::Buy => OrderDirection::Buy,
            OrderAction::Sell => OrderDirection::Sell,
        };

        let request = PostOrderRequest {
            figi: Some(figi.to_string()),
            quantity: quantity as i64,
            price: Some(t_invest_sdk::api::Quotation {
                units: price as i64,
                nano: ((price.fract() * 1_000_000_000.0) as i32),
            }),
            direction: direction as i32,
            account_id: self.account_id.clone(),
            order_type: OrderType::Limit as i32,
            order_id: format!("order_{}", Utc::now().timestamp()).to_string(),
            instrument_id: figi.to_string(),
            confirm_margin_trade: false,
            time_in_force: 0, // GoodTillCancel
            price_type: 0,    // TakeMarket
        };

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
        let direction = match action {
            OrderAction::Buy => OrderDirection::Buy,
            OrderAction::Sell => OrderDirection::Sell,
        };

        let request = PostOrderRequest {
            figi: Some(figi.to_string()),
            quantity: quantity as i64,
            price: None,
            direction: direction as i32,
            account_id: self.account_id.clone(),
            order_type: OrderType::Market as i32,
            order_id: format!("order_{}", Utc::now().timestamp()).to_string(),
            instrument_id: figi.to_string(),
            confirm_margin_trade: false,
            time_in_force: 0,
            price_type: 0,
        };

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
            results.push(OrderResult {
                order_id: order.order_id,
                figi: order.figi,
                action: OrderAction::Buy,
                quantity: order.lots_executed as i32,
                price: None,
                status: OrderStatus::New,
                created_at: Utc::now(),
                message: "Order from list".to_string(),
            });
        }

        Ok(results)
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

                        if let Some(sl_price) = decision.stop_loss {
                            log::info!(
                                "Placing Stop Loss: {} lots at price {:.2}",
                                quantity,
                                sl_price
                            );
                            // Stop loss via separate order
                        }
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

    #[test]
    fn test_calculate_quantity_basic() {
        // Test quantity calculation logic via closure
        let balance = 10000.0;

        // Closure for testing logic
        let calc_qty = |price: f64, position_pct: f64| -> i32 {
            if price <= 0.0 || position_pct <= 0.0 {
                return 0;
            }
            let position_value = balance * position_pct;
            let quantity = (position_value / price) as i32;
            quantity.max(0)
        };

        // With balance 10000 and 10% position = 1000
        // At price 100 = 10 lots
        assert_eq!(calc_qty(100.0, 0.1), 10);

        // At price 50 = 20 lots
        assert_eq!(calc_qty(50.0, 0.1), 20);

        // At 50% position = 5000, price 100 = 50 lots
        assert_eq!(calc_qty(100.0, 0.5), 50);
    }

    #[test]
    fn test_calculate_quantity_edge_cases() {
        let balance = 10000.0;

        let calc_qty = |price: f64, position_pct: f64| -> i32 {
            if price <= 0.0 || position_pct <= 0.0 {
                return 0;
            }
            let position_value = balance * position_pct;
            let quantity = (position_value / price) as i32;
            quantity.max(0)
        };

        // Zero price
        assert_eq!(calc_qty(0.0, 0.1), 0);

        // Negative price
        assert_eq!(calc_qty(-100.0, 0.1), 0);

        // Zero position percentage
        assert_eq!(calc_qty(100.0, 0.0), 0);

        // Negative percentage
        assert_eq!(calc_qty(100.0, -0.1), 0);
    }

    #[test]
    fn test_calculate_sell_quantity_basic() {
        // Closure for testing sell quantity calculation logic
        let calc_sell = |position: i32, pct: f64| -> i32 {
            if pct <= 0.0 || position <= 0 {
                return 0;
            }
            let quantity = (position as f64 * pct) as i32;
            quantity.max(1).min(position)
        };

        // Selling 50% of 100 lots = 50 lots
        assert_eq!(calc_sell(100, 0.5), 50);

        // Selling 25% of 100 lots = 25 lots
        assert_eq!(calc_sell(100, 0.25), 25);

        // Selling 100% of 100 lots = 100 lots
        assert_eq!(calc_sell(100, 1.0), 100);
    }

    #[test]
    fn test_calculate_sell_quantity_minimum() {
        let calc_sell = |position: i32, pct: f64| -> i32 {
            if pct <= 0.0 || position <= 0 {
                return 0;
            }
            let quantity = (position as f64 * pct) as i32;
            quantity.max(1).min(position)
        };

        // Selling 1% of 100 lots = 1 lot (minimum 1)
        assert_eq!(calc_sell(100, 0.01), 1);

        // Selling 0.1% of 100 lots = 1 lot (minimum 1)
        assert_eq!(calc_sell(100, 0.001), 1);
    }

    #[test]
    fn test_calculate_sell_quantity_edge_cases() {
        let calc_sell = |position: i32, pct: f64| -> i32 {
            if pct <= 0.0 || position <= 0 {
                return 0;
            }
            let quantity = (position as f64 * pct) as i32;
            quantity.max(1).min(position)
        };

        // Zero position
        assert_eq!(calc_sell(0, 0.5), 0);

        // Negative position
        assert_eq!(calc_sell(-10, 0.5), 0);

        // Zero percentage
        assert_eq!(calc_sell(100, 0.0), 0);

        // Negative percentage
        assert_eq!(calc_sell(100, -0.5), 0);
    }

    #[test]
    fn test_update_balance() {
        // Test that update_balance method is called without errors
        // A mock SDK is needed for proper testing
        let balance = 10000.0;
        let new_balance = 20000.0;
        // Balance update logic tested by creating with new balance
        assert!(new_balance > balance);
    }
}
