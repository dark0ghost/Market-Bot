//! Grid Trading Bot Module
//!
//! Grid бот для торговли в боковом тренде.
//! Расставляет ордера на покупку и продажу по ценовой сетке.

use crate::client::MarketDataService;
use crate::config::GridConfig;
use crate::strategy::{GridExecutor, GridStrategy};
use anyhow::Result;
use log::{error, info, warn};
use t_invest_sdk::TInvestSdk;

/// Конфигурация Grid бота
pub struct GridBotConfig {
    pub account_id: String,
    pub figi: String,
    pub ticker: String,
    pub grid_config: GridConfig,
    pub check_interval_secs: u64,
}

/// Grid бот
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

    /// Запуск Grid бота
    pub async fn run(&mut self) -> Result<()> {
        info!(
            "Запуск Grid бота для {} ({})",
            self.config.ticker, self.config.figi
        );

        // Создание стратегии
        let grid_strategy = GridStrategy::new(self.config.grid_config.clone());

        // Создание исполнителя
        let mut executor = GridExecutor::new(
            self.market_data_service.sdk_clone(),
            self.config.account_id.clone(),
            grid_strategy,
            self.config.figi.clone(),
        );

        // Получение текущей цены
        let current_price = match self
            .market_data_service
            .get_last_price(&self.config.figi)
            .await
        {
            Ok(p) => p,
            Err(e) => {
                error!("Ошибка получения цены: {}", e);
                return Err(e);
            }
        };
        info!("Текущая цена: {:.2}", current_price);

        // Инициализация сетки
        match executor.initialize_grid(current_price).await {
            Ok(results) => {
                info!(
                    "Grid сетка инициализирована, размещено ордеров: {}",
                    results.len()
                );
            }
            Err(e) => {
                error!("Ошибка инициализации Grid сетки: {}", e);
                return Err(e);
            }
        }

        self.executor = Some(executor);

        // Основной цикл мониторинга
        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(
                self.config.check_interval_secs,
            ))
            .await;

            if let Some(ref mut executor) = self.executor {
                // Получение новой цены
                let new_price = match self
                    .market_data_service
                    .get_last_price(&self.config.figi)
                    .await
                {
                    Ok(p) => p,
                    Err(e) => {
                        warn!("Ошибка получения цены: {}", e);
                        continue;
                    }
                };

                // Перебалансировка сетки
                match executor.rebalance_grid(new_price).await {
                    Ok(result) => {
                        if result.cancelled_orders > 0 || result.placed_orders > 0 {
                            info!(
                                "Сетка перебалансирована: отменено={}, размещено={}",
                                result.cancelled_orders, result.placed_orders
                            );
                        }
                    }
                    Err(e) => {
                        warn!("Ошибка перебалансировки: {}", e);
                    }
                }
            }
        }
    }

    /// Остановка бота
    pub async fn stop(&mut self) -> Result<()> {
        info!("Остановка Grid бота...");

        if let Some(ref mut executor) = self.executor {
            executor.stop_grid().await?;
        }

        Ok(())
    }
}
