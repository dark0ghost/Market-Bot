use crate::config::GridConfig;
use serde::{Deserialize, Serialize};

/// Grid level
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GridLevel {
    /// Level price
    pub price: f64,
    /// Order type: Buy or Sell
    pub order_type: OrderSide,
    /// Level index (0 - lowest)
    pub level_index: u32,
}

/// Order side
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum OrderSide {
    Buy,
    Sell,
}

/// Grid strategy state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GridState {
    /// Instrument ticker
    pub ticker: String,
    /// Instrument FIGI
    pub figi: String,
    /// All grid levels
    pub levels: Vec<GridLevel>,
    /// Active orders (level indices)
    pub active_orders: Vec<u32>,
    /// Filled orders (level indices)
    pub filled_orders: Vec<u32>,
    /// Current price
    pub current_price: f64,
}

/// Grid strategy
pub struct GridStrategy {
    config: GridConfig,
}

impl GridStrategy {
    pub const fn new(config: GridConfig) -> Self {
        GridStrategy { config }
    }

    /// Calculate grid levels
    pub fn calculate_grid_levels(&self) -> Vec<GridLevel> {
        let mut levels = Vec::new();
        let num_levels = self.config.grid_levels as usize;

        // Calculate grid step
        let price_range = self.config.upper_price - self.config.lower_price;
        let step = price_range / (num_levels as f64 - 1.0);

        // Split into buy and sell levels
        let buy_levels = (num_levels as f64 * self.config.grid_ratio) as usize;
        let sell_levels = num_levels - buy_levels;

        // Create buy levels (lower part)
        for i in 0..buy_levels {
            let price = self.config.lower_price + (step * i as f64);
            levels.push(GridLevel {
                price,
                order_type: OrderSide::Buy,
                level_index: i as u32,
            });
        }

        // Create sell levels (upper part)
        for i in 0..sell_levels {
            let level_idx = buy_levels + i;
            let price = self.config.lower_price + (step * level_idx as f64);
            levels.push(GridLevel {
                price,
                order_type: OrderSide::Sell,
                level_index: level_idx as u32,
            });
        }

        levels
    }

    /// Determine levels to place based on current price
    pub fn get_levels_to_place(&self, current_price: f64) -> Vec<GridLevel> {
        let levels = self.calculate_grid_levels();
        let mut levels_to_place = Vec::new();

        for level in levels {
            match level.order_type {
                OrderSide::Buy => {
                    // Buy orders are placed below current price
                    if level.price < current_price {
                        levels_to_place.push(level);
                    }
                }
                OrderSide::Sell => {
                    // Sell orders are placed above current price
                    if level.price > current_price {
                        levels_to_place.push(level);
                    }
                }
            }
        }

        levels_to_place
    }

    /// Check if orders need to be re-placed
    pub const fn needs_rebalance(
        &self,
        state: &GridState,
        current_price: f64,
        price_threshold: f64,
    ) -> bool {
        // If price changed significantly since last update
        let price_diff = (state.current_price - current_price).abs() / state.current_price;
        price_diff > price_threshold
    }

    /// Get level by index
    pub fn get_level_by_index<'a>(
        &'a self,
        levels: &'a [GridLevel],
        index: u32,
    ) -> Option<&'a GridLevel> {
        levels.iter().find(|l| l.level_index == index)
    }

    /// Configuration
    pub const fn config(&self) -> &GridConfig {
        &self.config
    }
}

// ─── Strategy trait implementation ────────────────────────────────
use crate::core::*;
use anyhow::Result;
use async_trait::async_trait;

#[async_trait]
impl Strategy for GridStrategy {
    fn name(&self) -> &str {
        "grid"
    }

    fn kind(&self) -> StrategyKind {
        StrategyKind::Grid
    }

    async fn on_start(&mut self, _broker: &dyn Broker) -> Result<()> {
        log::info!(
            "GridStrategy: starting with config: levels={}, range={}-{}",
            self.config.grid_levels,
            self.config.lower_price,
            self.config.upper_price
        );
        Ok(())
    }

    async fn analyze(&self, broker: &dyn Broker, instrument: &str) -> Result<Vec<Signal>> {
        let price = broker.last_price(instrument).await?;
        let levels = self.get_levels_to_place(price);
        let mut signals = Vec::new();

        for level in levels {
            let action = match level.order_type {
                OrderSide::Buy => crate::core::OrderAction::Buy,
                OrderSide::Sell => crate::core::OrderAction::Sell,
            };
            signals.push(Signal {
                ticker: instrument.to_string(),
                timestamp: chrono::Utc::now(),
                action,
                confidence: 0.7,
                price: level.price,
                source: "grid".to_string(),
                metadata: std::collections::HashMap::new(),
            });
        }

        Ok(signals)
    }

    async fn on_tick(&mut self, _broker: &dyn Broker) -> Result<()> {
        Ok(())
    }

    fn validate(&self) -> Result<()> {
        if self.config.grid_levels < 2 {
            anyhow::bail!("Grid must have at least 2 levels");
        }
        if self.config.upper_price <= self.config.lower_price {
            anyhow::bail!("upper_price must be greater than lower_price");
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_grid_calculation() {
        let config = GridConfig {
            lower_price: 100.0,
            upper_price: 200.0,
            grid_levels: 11,
            order_size: 10,
            grid_ratio: 0.5,
        };

        let strategy = GridStrategy::new(config);
        let levels = strategy.calculate_grid_levels();

        assert_eq!(levels.len(), 11);
        assert_eq!(levels[0].price, 100.0);
        assert_eq!(levels[10].price, 200.0);
        assert_eq!(levels[0].order_type, OrderSide::Buy);
        assert_eq!(levels[10].order_type, OrderSide::Sell);
    }

    #[test]
    fn test_levels_to_place() {
        let config = GridConfig {
            lower_price: 100.0,
            upper_price: 200.0,
            grid_levels: 11,
            order_size: 10,
            grid_ratio: 0.5,
        };

        let strategy = GridStrategy::new(config);

        // Current price 150 - buy should be < 150 and sell > 150
        let levels = strategy.get_levels_to_place(150.0);

        for level in &levels {
            match level.order_type {
                OrderSide::Buy => assert!(level.price < 150.0),
                OrderSide::Sell => assert!(level.price > 150.0),
            }
        }
    }
}
