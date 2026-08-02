//! Grid Trading Bot Module
//!
//! Grid bot for trading in a sideways trend.
//! Places buy and sell orders on a price grid.

use crate::client::MarketDataService;
use crate::config::GridConfig;
use crate::strategy::{GridExecutor, GridStrategy, TradingCalendar};
use anyhow::Result;
use log::{error, info, warn};
use t_invest_sdk::TInvestSdk;

/// Grid bot configuration
pub struct GridBotConfig {
    pub account_id: String,
    pub figi: String,
    pub ticker: String,
    pub grid_config: GridConfig,
    pub check_interval_secs: u64,
}

/// Grid bot
pub struct GridBot {
    config: GridBotConfig,
    executor: Option<GridExecutor>,
    market_data_service: MarketDataService,
}

impl GridBot {
    pub fn new(sdk: TInvestSdk, config: GridBotConfig) -> Self {
        let market_data_service = MarketDataService::new(sdk.clone());

        GridBot {
            config,
            executor: None,
            market_data_service,
        }
    }

    /// Run Grid bot
    pub async fn run(&mut self) -> Result<()> {
        info!(
            "Starting Grid bot for {} ({})",
            self.config.ticker, self.config.figi
        );

        // Create strategy
        let grid_strategy = GridStrategy::new(self.config.grid_config.clone());

        // Create executor
        let mut executor = GridExecutor::new(
            self.market_data_service.sdk_clone(),
            self.config.account_id.clone(),
            grid_strategy,
            self.config.figi.clone(),
        );

        // Get current price
        let current_price = match self
            .market_data_service
            .get_last_price(&self.config.figi)
            .await
        {
            Ok(p) => p,
            Err(e) => {
                error!("Price fetch error: {}", e);
                return Err(e);
            }
        };
        info!("Current price: {:.2}", current_price);

        // Initialize grid
        match executor.initialize_grid(current_price).await {
            Ok(results) => {
                info!("Grid initialized, orders placed: {}", results.len());
            }
            Err(e) => {
                error!("Grid initialization error: {}", e);
                return Err(e);
            }
        }

        self.executor = Some(executor);

        // Main monitoring loop
        let calendar = TradingCalendar::default();
        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(
                self.config.check_interval_secs,
            ))
            .await;

            // Don't place/rebalance orders outside the MOEX session -
            // exchange would reject them anyway and we'd waste API calls.
            if !calendar.is_open_now() {
                info!("MOEX closed, skipping grid rebalance this tick");
                continue;
            }

            if let Some(ref mut executor) = self.executor {
                // Get new price
                let new_price = match self
                    .market_data_service
                    .get_last_price(&self.config.figi)
                    .await
                {
                    Ok(p) => p,
                    Err(e) => {
                        warn!("Price fetch error: {}", e);
                        continue;
                    }
                };

                // Rebalance grid
                match executor.rebalance_grid(new_price).await {
                    Ok(result) => {
                        if result.cancelled_orders > 0 || result.placed_orders > 0 {
                            info!(
                                "Grid rebalanced: cancelled={}, placed={}",
                                result.cancelled_orders, result.placed_orders
                            );
                        }
                    }
                    Err(e) => {
                        warn!("Grid rebalance error: {}", e);
                    }
                }
            }
        }
    }

    /// Stop bot
    pub async fn stop(&mut self) -> Result<()> {
        info!("Stopping Grid bot...");

        if let Some(ref mut executor) = self.executor {
            executor.stop_grid().await?;
        }

        Ok(())
    }
}
