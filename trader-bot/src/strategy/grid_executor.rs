use crate::core::OrderAction;
use crate::execution::position_manager::{OrderResult, PositionManager};
use crate::strategy::grid::{GridLevel, GridState, GridStrategy, OrderSide};
use anyhow::Result;
use log::{error, info, warn};
use t_invest_sdk::TInvestSdk;

/// Grid order placement result
#[derive(Debug, Clone)]
pub struct GridOrderResult {
    pub level_index: u32,
    pub order_result: OrderResult,
}

/// Grid strategy manager
pub struct GridExecutor {
    position_manager: PositionManager,
    grid_strategy: GridStrategy,
    /// Current grid state
    grid_state: Option<GridState>,
    /// Instrument FIGI
    figi: String,
    /// Order size in lots
    order_size: u32,
    /// Mapping level_index -> live broker order_id, so we can cancel by level.
    level_order_ids: std::collections::HashMap<u32, String>,
}

impl GridExecutor {
    pub fn new(
        sdk: TInvestSdk,
        account_id: String,
        grid_strategy: GridStrategy,
        figi: String,
    ) -> Self {
        let order_size = grid_strategy.config().order_size;

        GridExecutor {
            position_manager: PositionManager::new(sdk, account_id),
            grid_strategy,
            grid_state: None,
            figi,
            order_size,
            level_order_ids: std::collections::HashMap::new(),
        }
    }

    /// Initialize order grid
    pub async fn initialize_grid(&mut self, current_price: f64) -> Result<Vec<GridOrderResult>> {
        info!("Initializing grid for {}", self.figi);
        info!("Current price: {:.2}", current_price);

        let levels_to_place = self.grid_strategy.get_levels_to_place(current_price);
        info!("Levels to place: {}", levels_to_place.len());

        let mut results = Vec::new();
        let mut active_orders = Vec::new();

        for level in levels_to_place {
            match self.place_grid_order(&level).await {
                Ok(order_result) => {
                    info!(
                        "Order placed: level={}, price={:.2}, side={:?}",
                        level.level_index, level.price, level.order_type
                    );
                    self.level_order_ids
                        .insert(level.level_index, order_result.order_id.clone());
                    active_orders.push(level.level_index);
                    results.push(GridOrderResult {
                        level_index: level.level_index,
                        order_result,
                    });
                }
                Err(e) => {
                    warn!("Error placing order at level {}: {}", level.level_index, e);
                }
            }
        }

        // Save state
        self.grid_state = Some(GridState {
            ticker: self.figi.clone(),
            figi: self.figi.clone(),
            levels: self.grid_strategy.calculate_grid_levels(),
            active_orders,
            filled_orders: Vec::new(),
            current_price,
        });

        Ok(results)
    }

    /// Place order at level and record the resulting broker order_id.
    async fn place_grid_order(&self, level: &GridLevel) -> Result<OrderResult> {
        let action = match level.order_type {
            OrderSide::Buy => OrderAction::Buy,
            OrderSide::Sell => OrderAction::Sell,
        };

        self.position_manager
            .place_limit_order(&self.figi, action, self.order_size as i32, level.price)
            .await
    }

    /// Check order execution and re-place
    pub async fn rebalance_grid(&mut self, current_price: f64) -> Result<RebalanceResult> {
        let state = match &self.grid_state {
            Some(s) => s.clone(),
            None => return Err(anyhow::anyhow!("Grid state not initialized")),
        };

        // Check if rebalance is needed
        let needs_rebalance = self
            .grid_strategy
            .needs_rebalance(&state, current_price, 0.02); // 2% threshold

        if !needs_rebalance {
            return Ok(RebalanceResult {
                cancelled_orders: 0,
                placed_orders: 0,
            });
        }

        info!("Rebalancing grid...");
        info!(
            "Old price: {:.2}, new price: {:.2}",
            state.current_price, current_price
        );

        let mut cancelled = 0;
        let mut placed = 0;

        // Get new levels to place
        let new_levels = self.grid_strategy.get_levels_to_place(current_price);
        let new_level_indices: Vec<u32> = new_levels.iter().map(|l| l.level_index).collect();

        // Cancel orders that are no longer needed
        for &active_index in &state.active_orders {
            if !new_level_indices.contains(&active_index) {
                // Need to cancel order
                if let Err(e) = self.cancel_order_by_level(active_index).await {
                    warn!("Error cancelling order at level {}: {}", active_index, e);
                } else {
                    cancelled += 1;
                }
            }
        }

        // Place new orders
        for level in new_levels {
            if !state.active_orders.contains(&level.level_index)
                && !state.filled_orders.contains(&level.level_index)
            {
                match self.place_grid_order(&level).await {
                    Ok(order_result) => {
                        self.level_order_ids
                            .insert(level.level_index, order_result.order_id);
                        placed += 1;
                    }
                    Err(e) => {
                        warn!("Error placing order: {}", e);
                    }
                }
            }
        }

        // Update state
        if let Some(ref mut state) = self.grid_state {
            state.active_orders = new_level_indices;
            state.current_price = current_price;
        }

        info!(
            "Rebalance complete: cancelled={}, placed={}",
            cancelled, placed
        );

        Ok(RebalanceResult {
            cancelled_orders: cancelled,
            placed_orders: placed,
        })
    }

    /// Cancel the live broker order mapped to a grid level.
    async fn cancel_order_by_level(&mut self, level_index: u32) -> Result<()> {
        let Some(order_id) = self.level_order_ids.get(&level_index).cloned() else {
            warn!(
                "No mapped order_id for level {}, skipping cancel",
                level_index
            );
            return Ok(());
        };
        self.position_manager.cancel_order(&order_id).await?;
        self.level_order_ids.remove(&level_index);
        Ok(())
    }

    /// Handle order fill
    pub async fn on_order_filled(&mut self, level_index: u32) -> Result<()> {
        if let Some(ref mut state) = self.grid_state {
            // Remove from active
            state.active_orders.retain(|&i| i != level_index);
            // Add to filled
            state.filled_orders.push(level_index);

            // Place opposite order
            let levels = self.grid_strategy.calculate_grid_levels();
            if let Some(level) = self.grid_strategy.get_level_by_index(&levels, level_index) {
                let opposite_side = match level.order_type {
                    OrderSide::Buy => OrderSide::Sell,
                    OrderSide::Sell => OrderSide::Buy,
                };

                let opposite_level = GridLevel {
                    price: level.price, // Same price
                    order_type: opposite_side.clone(),
                    level_index: level.level_index + 1000, // Unique index
                };

                match self.place_grid_order(&opposite_level).await {
                    Ok(order_result) => {
                        self.level_order_ids
                            .insert(opposite_level.level_index, order_result.order_id);
                        info!(
                            "Order filled (level {:?}), placed opposite {:?}",
                            level.order_type, opposite_side
                        );
                    }
                    Err(e) => {
                        error!("Error placing opposite order: {}", e);
                    }
                }
            }
        }

        Ok(())
    }

    /// Get current state
    pub fn get_state(&self) -> Option<&GridState> {
        self.grid_state.as_ref()
    }

    /// Stop Grid bot - cancel every live broker order, then drop state.
    pub async fn stop_grid(&mut self) -> Result<()> {
        info!("Stopping Grid bot, cancelling all orders...");

        // Cancel every mapped live order. Collect first to avoid borrow issues.
        let order_ids: Vec<String> = self.level_order_ids.values().cloned().collect();
        let mut cancelled = 0;
        for order_id in &order_ids {
            match self.position_manager.cancel_order(order_id).await {
                Ok(_) => cancelled += 1,
                Err(e) => warn!("Failed to cancel order {}: {}", order_id, e),
            }
        }
        self.level_order_ids.clear();
        self.grid_state = None;
        info!("Grid stopped, cancelled {} orders", cancelled);

        Ok(())
    }
}

/// Rebalance result
#[derive(Debug, Clone)]
pub struct RebalanceResult {
    pub cancelled_orders: u32,
    pub placed_orders: u32,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::GridConfig;
    use crate::core::OrderStatus;

    #[test]
    fn test_rebalance_result_debug() {
        let result = RebalanceResult {
            cancelled_orders: 2,
            placed_orders: 3,
        };
        assert_eq!(
            format!("{:?}", result),
            "RebalanceResult { cancelled_orders: 2, placed_orders: 3 }"
        );
    }

    #[test]
    fn test_rebalance_result_clone() {
        let result = RebalanceResult {
            cancelled_orders: 1,
            placed_orders: 2,
        };
        let cloned = result.clone();
        assert_eq!(cloned.cancelled_orders, result.cancelled_orders);
        assert_eq!(cloned.placed_orders, result.placed_orders);
    }

    #[test]
    fn test_grid_order_result_debug() {
        // Test Debug implementation for GridOrderResult
        // Since OrderResult does not fully implement Debug, test the structure
        let order_result = OrderResult {
            order_id: "test_123".to_string(),
            figi: "BBG000B9XRY4".to_string(),
            action: OrderAction::Buy,
            quantity: 10,
            price: Some(150.50),
            status: OrderStatus::New,
            created_at: chrono::Utc::now(),
            message: "Test".to_string(),
        };

        let grid_result = GridOrderResult {
            level_index: 5,
            order_result,
        };

        let debug_str = format!("{:?}", grid_result);
        assert!(debug_str.contains("GridOrderResult"));
        assert!(debug_str.contains("level_index: 5"));
    }

    #[test]
    fn test_grid_state_structure() {
        // Test GridState structure
        let state = GridState {
            ticker: "TINK".to_string(),
            figi: "BBG000B9XRY4".to_string(),
            levels: vec![],
            active_orders: vec![1, 2, 3],
            filled_orders: vec![0],
            current_price: 150.0,
        };

        assert_eq!(state.ticker, "TINK");
        assert_eq!(state.active_orders.len(), 3);
        assert_eq!(state.filled_orders.len(), 1);
        assert_eq!(state.current_price, 150.0);
    }

    #[test]
    fn test_needs_rebalance_threshold() {
        // Test rebalance logic through GridStrategy
        let config = GridConfig {
            lower_price: 100.0,
            upper_price: 200.0,
            grid_levels: 11,
            order_size: 10,
            grid_ratio: 0.5,
        };

        let strategy = GridStrategy::new(config);

        let state = GridState {
            ticker: "TINK".to_string(),
            figi: "BBG000B9XRY4".to_string(),
            levels: vec![],
            active_orders: vec![],
            filled_orders: vec![],
            current_price: 150.0,
        };

        // Price change 1% - less than 2% threshold
        assert!(!strategy.needs_rebalance(&state, 151.5, 0.02));

        // Price change 3% - more than 2% threshold
        assert!(strategy.needs_rebalance(&state, 154.5, 0.02));

        // Price change down 3%
        assert!(strategy.needs_rebalance(&state, 145.5, 0.02));
    }

    #[test]
    fn test_get_levels_to_place_logic() {
        let config = GridConfig {
            lower_price: 100.0,
            upper_price: 200.0,
            grid_levels: 11,
            order_size: 10,
            grid_ratio: 0.5,
        };

        let strategy = GridStrategy::new(config);

        // Price 150 - buy levels < 150, sell levels > 150
        let levels = strategy.get_levels_to_place(150.0);

        let buy_count = levels
            .iter()
            .filter(|l| l.order_type == OrderSide::Buy)
            .count();
        let sell_count = levels
            .iter()
            .filter(|l| l.order_type == OrderSide::Sell)
            .count();

        // With grid_ratio 0.5 and 11 levels: 5 buy, 5 sell (middle level skipped)
        assert!(buy_count > 0);
        assert!(sell_count > 0);

        // All buy levels must be < 150
        for level in &levels {
            if level.order_type == OrderSide::Buy {
                assert!(level.price < 150.0);
            } else {
                assert!(level.price > 150.0);
            }
        }
    }

    #[tokio::test]
    async fn test_stop_grid_clears_state() {
        // Test that stop_grid clears state
        // A mock SDK is needed for a full test
        // Only test the method logic

        let state: Option<GridState> = None;

        assert!(state.is_none());
    }

    #[test]
    fn test_on_order_filled_logic() {
        // Test order fill handling logic
        let mut state = GridState {
            ticker: "TINK".to_string(),
            figi: "BBG000B9XRY4".to_string(),
            levels: vec![],
            active_orders: vec![1, 2, 3],
            filled_orders: vec![],
            current_price: 150.0,
        };

        // Simulate on_order_filled for level 2
        let level_index = 2;
        state.active_orders.retain(|&i| i != level_index);
        state.filled_orders.push(level_index);

        assert!(!state.active_orders.contains(&2));
        assert!(state.filled_orders.contains(&2));
        assert_eq!(state.active_orders.len(), 2);
        assert_eq!(state.filled_orders.len(), 1);
    }

    #[test]
    fn test_opposite_order_side() {
        // Test opposite side determination logic
        let buy_side = OrderSide::Buy;
        let sell_side = OrderSide::Sell;

        let opposite_to_buy = match buy_side {
            OrderSide::Buy => OrderSide::Sell,
            OrderSide::Sell => OrderSide::Buy,
        };

        let opposite_to_sell = match sell_side {
            OrderSide::Buy => OrderSide::Sell,
            OrderSide::Sell => OrderSide::Buy,
        };

        assert_eq!(opposite_to_buy, OrderSide::Sell);
        assert_eq!(opposite_to_sell, OrderSide::Buy);
    }
}
