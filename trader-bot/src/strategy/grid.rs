use crate::config::GridConfig;
use serde::{Deserialize, Serialize};

/// Уровень сетки
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GridLevel {
    /// Цена уровня
    pub price: f64,
    /// Тип ордера: Buy или Sell
    pub order_type: OrderSide,
    /// Индекс уровня (0 - самый нижний)
    pub level_index: u32,
}

/// Сторона ордера
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum OrderSide {
    Buy,
    Sell,
}

/// Состояние Grid стратегии
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GridState {
    /// Тикер инструмента
    pub ticker: String,
    /// FIGI инструмента
    pub figi: String,
    /// Все уровни сетки
    pub levels: Vec<GridLevel>,
    /// Активные ордера (индексы уровней)
    pub active_orders: Vec<u32>,
    /// Исполненные ордера (индексы уровней)
    pub filled_orders: Vec<u32>,
    /// Текущая цена
    pub current_price: f64,
}

/// Grid стратегия
pub struct GridStrategy {
    config: GridConfig,
}

impl GridStrategy {
    pub fn new(config: GridConfig) -> Self {
        GridStrategy { config }
    }

    /// Расчет уровней сетки
    pub fn calculate_grid_levels(&self) -> Vec<GridLevel> {
        let mut levels = Vec::new();
        let num_levels = self.config.grid_levels as usize;
        
        // Расчет шага сетки
        let price_range = self.config.upper_price - self.config.lower_price;
        let step = price_range / (num_levels as f64 - 1.0);
        
        // Разделение на buy и sell уровни
        let buy_levels = (num_levels as f64 * self.config.grid_ratio) as usize;
        let sell_levels = num_levels - buy_levels;
        
        // Создание уровней для покупки (нижняя часть)
        for i in 0..buy_levels {
            let price = self.config.lower_price + (step * i as f64);
            levels.push(GridLevel {
                price,
                order_type: OrderSide::Buy,
                level_index: i as u32,
            });
        }
        
        // Создание уровней для продажи (верхняя часть)
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

    /// Определение уровней для размещения на основе текущей цены
    pub fn get_levels_to_place(&self, current_price: f64) -> Vec<GridLevel> {
        let levels = self.calculate_grid_levels();
        let mut levels_to_place = Vec::new();
        
        for level in levels {
            match level.order_type {
                OrderSide::Buy => {
                    // Buy ордера размещаем ниже текущей цены
                    if level.price < current_price {
                        levels_to_place.push(level);
                    }
                }
                OrderSide::Sell => {
                    // Sell ордера размещаем выше текущей цены
                    if level.price > current_price {
                        levels_to_place.push(level);
                    }
                }
            }
        }
        
        levels_to_place
    }

    /// Проверка, нужно ли перевыставить ордера
    pub fn needs_rebalance(&self, state: &GridState, current_price: f64, price_threshold: f64) -> bool {
        // Если цена сильно изменилась с момента последнего обновления
        let price_diff = (state.current_price - current_price).abs() / state.current_price;
        price_diff > price_threshold
    }

    /// Получение уровня по индексу
    pub fn get_level_by_index<'a>(&'a self, levels: &'a [GridLevel], index: u32) -> Option<&'a GridLevel> {
        levels.iter().find(|l| l.level_index == index)
    }

    /// Конфигурация
    pub fn config(&self) -> &GridConfig {
        &self.config
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
        
        // Текущая цена 150 - должны быть buy < 150 и sell > 150
        let levels = strategy.get_levels_to_place(150.0);
        
        for level in &levels {
            match level.order_type {
                OrderSide::Buy => assert!(level.price < 150.0),
                OrderSide::Sell => assert!(level.price > 150.0),
            }
        }
    }
}
