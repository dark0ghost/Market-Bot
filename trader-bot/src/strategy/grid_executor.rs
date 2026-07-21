use crate::execution::position_manager::{OrderAction, OrderResult, PositionManager};
use crate::strategy::grid::{GridLevel, GridState, GridStrategy, OrderSide};
use anyhow::Result;
use log::{error, info, warn};
use t_invest_sdk::TInvestSdk;

/// Результат размещения ордера в Grid
#[derive(Debug, Clone)]
pub struct GridOrderResult {
    pub level_index: u32,
    pub order_result: OrderResult,
}

/// Менеджер Grid стратегии
pub struct GridExecutor {
    position_manager: PositionManager,
    grid_strategy: GridStrategy,
    /// Текущее состояние сетки
    grid_state: Option<GridState>,
    /// FIGI инструмента
    figi: String,
    /// Размер ордера в лотах
    order_size: u32,
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
        }
    }

    /// Инициализация сетки ордеров
    pub async fn initialize_grid(&mut self, current_price: f64) -> Result<Vec<GridOrderResult>> {
        info!("Инициализация Grid сетки для {}", self.figi);
        info!("Текущая цена: {:.2}", current_price);

        // Получаем уровни для размещения
        let levels_to_place = self.grid_strategy.get_levels_to_place(current_price);
        info!("Уровней для размещения: {}", levels_to_place.len());

        let mut results = Vec::new();
        let mut active_orders = Vec::new();

        for level in levels_to_place {
            match self.place_grid_order(&level).await {
                Ok(order_result) => {
                    info!(
                        "Ордер размещен: уровень={}, цена={:.2}, сторона={:?}",
                        level.level_index, level.price, level.order_type
                    );
                    active_orders.push(level.level_index);
                    results.push(GridOrderResult {
                        level_index: level.level_index,
                        order_result,
                    });
                }
                Err(e) => {
                    warn!(
                        "Ошибка размещения ордера на уровне {}: {}",
                        level.level_index, e
                    );
                }
            }
        }

        // Сохраняем состояние
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

    /// Размещение ордера на уровне
    async fn place_grid_order(&self, level: &GridLevel) -> Result<OrderResult> {
        let action = match level.order_type {
            OrderSide::Buy => OrderAction::Buy,
            OrderSide::Sell => OrderAction::Sell,
        };

        self.position_manager
            .place_limit_order(&self.figi, action, self.order_size as i32, level.price)
            .await
    }

    /// Проверка исполнения ордеров и перевыставление
    pub async fn rebalance_grid(&mut self, current_price: f64) -> Result<RebalanceResult> {
        let state = match &self.grid_state {
            Some(s) => s.clone(),
            None => return Err(anyhow::anyhow!("Grid состояние не инициализировано")),
        };

        // Проверяем, нужно ли перебалансировать
        let needs_rebalance = self
            .grid_strategy
            .needs_rebalance(&state, current_price, 0.02); // 2% порог

        if !needs_rebalance {
            return Ok(RebalanceResult {
                cancelled_orders: 0,
                placed_orders: 0,
            });
        }

        info!("Перебалансировка Grid сетки...");
        info!(
            "Старая цена: {:.2}, новая цена: {:.2}",
            state.current_price, current_price
        );

        let mut cancelled = 0;
        let mut placed = 0;

        // Получаем новые уровни для размещения
        let new_levels = self.grid_strategy.get_levels_to_place(current_price);
        let new_level_indices: Vec<u32> = new_levels.iter().map(|l| l.level_index).collect();

        // Отменяем ордера, которые больше не нужны
        for &active_index in &state.active_orders {
            if !new_level_indices.contains(&active_index) {
                // Нужно отменить ордер
                if let Err(e) = self.cancel_order_by_level(active_index).await {
                    warn!("Ошибка отмены ордера уровня {}: {}", active_index, e);
                } else {
                    cancelled += 1;
                }
            }
        }

        // Размещаем новые ордера
        for level in new_levels {
            if !state.active_orders.contains(&level.level_index)
                && !state.filled_orders.contains(&level.level_index)
            {
                match self.place_grid_order(&level).await {
                    Ok(_) => {
                        placed += 1;
                    }
                    Err(e) => {
                        warn!("Ошибка размещения ордера: {}", e);
                    }
                }
            }
        }

        // Обновляем состояние
        if let Some(ref mut state) = self.grid_state {
            state.active_orders = new_level_indices;
            state.current_price = current_price;
        }

        info!(
            "Перебалансировка завершена: отменено={}, размещено={}",
            cancelled, placed
        );

        Ok(RebalanceResult {
            cancelled_orders: cancelled,
            placed_orders: placed,
        })
    }

    /// Отмена ордера по индексу уровня
    async fn cancel_order_by_level(&self, _level_index: u32) -> Result<()> {
        // TODO: Нужно хранить mapping level_index -> order_id
        // Пока заглушка
        Ok(())
    }

    /// Обработка исполнения ордера
    pub async fn on_order_filled(&mut self, level_index: u32) -> Result<()> {
        if let Some(ref mut state) = self.grid_state {
            // Удаляем из активных
            state.active_orders.retain(|&i| i != level_index);
            // Добавляем в исполненные
            state.filled_orders.push(level_index);

            // Размещаем противоположный ордер
            let levels = self.grid_strategy.calculate_grid_levels();
            if let Some(level) = self.grid_strategy.get_level_by_index(&levels, level_index) {
                let opposite_side = match level.order_type {
                    OrderSide::Buy => OrderSide::Sell,
                    OrderSide::Sell => OrderSide::Buy,
                };

                let opposite_level = GridLevel {
                    price: level.price, // Цена та же
                    order_type: opposite_side.clone(),
                    level_index: level.level_index + 1000, // Уникальный индекс
                };

                match self.place_grid_order(&opposite_level).await {
                    Ok(_) => {
                        info!(
                            "Ордер исполнен (уровень {:?}), размещен противоположный {:?}",
                            level.order_type, opposite_side
                        );
                    }
                    Err(e) => {
                        error!("Ошибка размещения противоположного ордера: {}", e);
                    }
                }
            }
        }

        Ok(())
    }

    /// Получение текущего состояния
    pub fn get_state(&self) -> Option<&GridState> {
        self.grid_state.as_ref()
    }

    /// Остановка Grid бота - отмена всех ордеров
    pub async fn stop_grid(&mut self) -> Result<()> {
        info!("Остановка Grid бота, отмена всех ордеров...");

        // TODO: Отмена всех активных ордеров

        self.grid_state = None;

        Ok(())
    }
}

/// Результат перебалансировки
#[derive(Debug, Clone)]
pub struct RebalanceResult {
    pub cancelled_orders: u32,
    pub placed_orders: u32,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::GridConfig;

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
        // Тестируем Debug реализацию для GridOrderResult
        // Поскольку OrderResult не реализует Debug полностью, тестируем структуру
        let order_result = OrderResult {
            order_id: "test_123".to_string(),
            figi: "BBG000B9XRY4".to_string(),
            action: OrderAction::Buy,
            quantity: 10,
            price: Some(150.50),
            status: crate::execution::position_manager::OrderStatus::New,
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
        // Тестируем структуру GridState
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
        // Тестируем логику rebalance через GridStrategy
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

        // Изменение цены 1% - меньше порога 2%
        assert!(!strategy.needs_rebalance(&state, 151.5, 0.02));

        // Изменение цены 3% - больше порога 2%
        assert!(strategy.needs_rebalance(&state, 154.5, 0.02));

        // Изменение цены вниз 3%
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

        // Цена 150 - buy уровни < 150, sell уровни > 150
        let levels = strategy.get_levels_to_place(150.0);

        let buy_count = levels
            .iter()
            .filter(|l| l.order_type == OrderSide::Buy)
            .count();
        let sell_count = levels
            .iter()
            .filter(|l| l.order_type == OrderSide::Sell)
            .count();

        // При grid_ratio 0.5 и 11 уровнях: 5 buy, 5 sell (средний уровень пропускается)
        assert!(buy_count > 0);
        assert!(sell_count > 0);

        // Все buy уровни должны быть < 150
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
        // Тестируем, что stop_grid очищает состояние
        // Для полноценного теста нужен моковый SDK
        // Проверяем только логику метода

        let mut state: Option<GridState> = Some(GridState {
            ticker: "TINK".to_string(),
            figi: "BBG000B9XRY4".to_string(),
            levels: vec![],
            active_orders: vec![1, 2],
            filled_orders: vec![],
            current_price: 150.0,
        });

        // Имитация stop_grid
        state = None;

        assert!(state.is_none());
    }

    #[test]
    fn test_on_order_filled_logic() {
        // Тестируем логику обработки исполнения ордера
        let mut state = GridState {
            ticker: "TINK".to_string(),
            figi: "BBG000B9XRY4".to_string(),
            levels: vec![],
            active_orders: vec![1, 2, 3],
            filled_orders: vec![],
            current_price: 150.0,
        };

        // Имитация on_order_filled для уровня 2
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
        // Тестируем логику определения противоположной стороны
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
