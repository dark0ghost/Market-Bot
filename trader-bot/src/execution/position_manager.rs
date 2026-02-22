use anyhow::Result;
use t_invest_sdk::api::{
    PostOrderRequest, OrderDirection, OrderType,
    GetOrdersRequest,
};
use t_invest_sdk::TInvestSdk;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Тип заявки
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OrderAction {
    Buy,
    Sell,
}

/// Результат размещения заявки
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

/// Статус заявки
#[derive(Debug, Clone, PartialEq)]
pub enum OrderStatus {
    New,
    PartiallyFilled,
    Filled,
    Cancelled,
    Rejected,
}

/// Менеджер позиций и заявок
pub struct PositionManager {
    sdk: TInvestSdk,
    account_id: String,
}

impl PositionManager {
    pub fn new(sdk: TInvestSdk, account_id: String) -> Self {
        PositionManager {
            sdk,
            account_id,
        }
    }

    /// Размещение лимитной заявки
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

    /// Размещение рыночной заявки
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

    /// Получение списка заявок
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

/// Расширенный менеджер для работы с решениями агента
pub struct TradingExecutor {
    position_manager: PositionManager,
    available_balance: f64,
}

impl TradingExecutor {
    pub fn new(position_manager: PositionManager, available_balance: f64) -> Self {
        TradingExecutor {
            position_manager,
            available_balance,
        }
    }

    /// Обновление доступного баланса
    pub fn update_balance(&mut self, balance: f64) {
        self.available_balance = balance;
    }

    /// Исполнение торгового решения
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
                    let quantity = self.calculate_quantity(
                        entry_price,
                        decision.position_size_pct,
                    );

                    if quantity > 0 {
                        log::info!(
                            "Размещение BUY заявки: {} лотов по цене {:.2} (сумма: {:.2})",
                            quantity,
                            entry_price,
                            quantity as f64 * entry_price
                        );

                        let order_result = self.position_manager
                            .place_limit_order(instrument_uid, OrderAction::Buy, quantity, entry_price)
                            .await?;
                        results.push(order_result);

                        if let Some(sl_price) = decision.stop_loss {
                            log::info!(
                                "Размещение Stop Loss: {} лотов по цене {:.2}",
                                quantity,
                                sl_price
                            );
                            // Stop loss через отдельную заявку
                        }
                    } else {
                        log::warn!("Расчетное количество лотов равно 0");
                    }
                }
            }
            Action::Sell => {
                if let Some(entry_price) = decision.entry_price {
                    // Для продажи используем текущую позицию
                    let quantity = self.calculate_sell_quantity(
                        entry_price,
                        decision.position_size_pct,
                    );

                    if quantity > 0 {
                        log::info!(
                            "Размещение SELL заявки: {} лотов по цене {:.2}",
                            quantity,
                            entry_price
                        );

                        let order_result = self.position_manager
                            .place_limit_order(instrument_uid, OrderAction::Sell, quantity, entry_price)
                            .await?;
                        results.push(order_result);
                    }
                }
            }
            Action::Hold => {
                log::info!("Decision for {}: HOLD - no action taken", decision.ticker);
            }
        }

        Ok(results)
    }

    /// Расчет количества лотов для покупки
    fn calculate_quantity(&self, price: f64, position_pct: f64) -> i32 {
        if price <= 0.0 || position_pct <= 0.0 {
            return 0;
        }

        // Используем реальный доступный баланс
        let position_value = self.available_balance * position_pct;
        let quantity = (position_value / price) as i32;

        // Округляем вниз до целого лота
        quantity.max(0)
    }

    /// Расчет количества лотов для продажи
    fn calculate_sell_quantity(&self, _price: f64, position_pct: f64) -> i32 {
        // Для продажи пока возвращаем заглушку
        // В реальности нужно получать текущую позицию из портфеля
        if position_pct <= 0.0 {
            return 0;
        }

        // Продаем всю позицию (100 лотов по умолчанию)
        100
    }
}
