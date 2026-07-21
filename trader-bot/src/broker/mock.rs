use anyhow::Result;
use async_trait::async_trait;
use chrono::Utc;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::core::*;

/// In-memory mock broker for backtesting and testing.
pub struct MockBroker {
    name: String,
    account_id: String,
    state: Arc<Mutex<MockState>>,
}

struct MockState {
    balance: f64,
    positions: HashMap<String, PositionView>,
    orders: Vec<OrderResponse>,
    candles_map: HashMap<String, Vec<Candle>>,
    price_map: HashMap<String, f64>,
}

impl MockBroker {
    pub fn new(account_id: String, initial_balance: f64) -> Self {
        MockBroker {
            name: format!("mock_{}", account_id),
            account_id,
            state: Arc::new(Mutex::new(MockState {
                balance: initial_balance,
                positions: HashMap::new(),
                orders: Vec::new(),
                candles_map: HashMap::new(),
                price_map: HashMap::new(),
            })),
        }
    }

    pub fn set_candles(&self, instrument: &str, candles: Vec<Candle>) {
        let mut state = self.state.lock().unwrap();
        state.candles_map.insert(instrument.to_string(), candles);
    }

    pub fn set_price(&self, instrument: &str, price: f64) {
        let mut state = self.state.lock().unwrap();
        state.price_map.insert(instrument.to_string(), price);

        if let Some(pos) = state.positions.get_mut(instrument) {
            pos.current_price = price;
            pos.pnl = (price - pos.average_price) * pos.quantity as f64;
        }
    }

    pub fn set_position(&self, instrument: &str, quantity: i32, avg_price: f64) {
        let current_price = self.state.lock().unwrap()
            .price_map.get(instrument).copied().unwrap_or(avg_price);
        let mut state = self.state.lock().unwrap();
        state.positions.insert(instrument.to_string(), PositionView {
            instrument: instrument.to_string(),
            quantity,
            average_price: avg_price,
            current_price,
            pnl: (current_price - avg_price) * quantity as f64,
            pnl_pct: if avg_price > 0.0 {
                (current_price - avg_price) / avg_price * 100.0
            } else { 0.0 },
            total_value: current_price * quantity as f64,
        });
    }
}

#[async_trait]
impl Broker for MockBroker {
    fn name(&self) -> &str {
        &self.name
    }

    fn broker_kind(&self) -> BrokerKind {
        BrokerKind::Mock
    }

    async fn candles(&self, instrument: &str, _interval: CandleInterval, _count: u32) -> Result<Vec<Candle>> {
        let state = self.state.lock().unwrap();
        Ok(state.candles_map.get(instrument)
            .cloned()
            .unwrap_or_default())
    }

    async fn last_price(&self, instrument: &str) -> Result<f64> {
        let state = self.state.lock().unwrap();
        state.price_map.get(instrument)
            .copied()
            .ok_or_else(|| anyhow::anyhow!("No price set for {}", instrument))
    }

    async fn order_book(&self, instrument: &str, _depth: u32) -> Result<OrderBook> {
        let price = self.last_price(instrument).await?;
        Ok(OrderBook {
            figi: instrument.to_string(),
            bids: vec![OrderBookLevel { price: price * 0.999, volume: 1000.0 }],
            asks: vec![OrderBookLevel { price: price * 1.001, volume: 1000.0 }],
            timestamp: Utc::now(),
        })
    }

    async fn liquidity(&self, instrument: &str, _depth: u32) -> Result<LiquidityInfo> {
        Ok(LiquidityInfo {
            total_bid_volume: 10000.0,
            total_ask_volume: 10000.0,
            bid_ask_ratio: 1.0,
            spread: 0.002,
        })
    }

    async fn place_order(&self, request: OrderRequest) -> Result<OrderResponse> {
        let mut state = self.state.lock().unwrap();
        let price = request.price
            .or_else(|| state.price_map.get(&request.instrument).copied())
            .unwrap_or(100.0);

        let order_id = format!("mock_{}", Utc::now().timestamp_micros());
        let response = OrderResponse {
            order_id: order_id.clone(),
            client_order_id: request.client_order_id.clone(),
            instrument: request.instrument.clone(),
            action: request.action.clone(),
            quantity: request.quantity,
            filled_quantity: request.quantity,
            price: Some(price),
            avg_fill_price: Some(price),
            status: OrderStatus::Filled,
            created_at: Utc::now(),
            message: "Mock fill".to_string(),
        };

        // Update position and balance
        let cost = price * request.quantity as f64;
        match request.action {
            OrderAction::Buy => {
                if cost <= state.balance {
                    state.balance -= cost;
                    let avg = state.positions.get(&request.instrument)
                        .map(|p| p.average_price)
                        .unwrap_or(0.0);
                    let total_qty = state.positions.get(&request.instrument)
                        .map(|p| p.quantity)
                        .unwrap_or(0);
                    let new_qty = total_qty + request.quantity;
                    let new_avg = if new_qty > 0 {
                        (avg * total_qty as f64 + cost) / new_qty as f64
                    } else { price };
                    state.positions.insert(request.instrument.clone(), PositionView {
                        instrument: request.instrument.clone(),
                        quantity: new_qty,
                        average_price: new_avg,
                        current_price: price,
                        pnl: 0.0,
                        pnl_pct: 0.0,
                        total_value: price * new_qty as f64,
                    });
                }
            }
            OrderAction::Sell => {
                let exists = state.positions.contains_key(&request.instrument);
                if exists {
                    let pos = state.positions.get(&request.instrument).cloned();
                    if let Some(p) = pos {
                        if p.quantity >= request.quantity {
                            let remaining = p.quantity - request.quantity;
                            let pnl = (price - p.average_price) * request.quantity as f64;
                            state.balance += cost;
                            if remaining > 0 {
                                state.positions.insert(request.instrument.clone(), PositionView {
                                    instrument: request.instrument.clone(),
                                    quantity: remaining,
                                    average_price: p.average_price,
                                    current_price: price,
                                    pnl,
                                    pnl_pct: (price - p.average_price) / p.average_price * 100.0,
                                    total_value: price * remaining as f64,
                                });
                            } else {
                                state.positions.remove(&request.instrument);
                            }
                        }
                    }
                }
            }
        }

        state.orders.push(response.clone());
        Ok(response)
    }

    async fn cancel_order(&self, order_id: &str) -> Result<()> {
        let mut state = self.state.lock().unwrap();
        state.orders.retain(|o| o.order_id != order_id);
        Ok(())
    }

    async fn get_orders(&self, _instrument: Option<&str>) -> Result<Vec<OrderResponse>> {
        let state = self.state.lock().unwrap();
        Ok(state.orders.clone())
    }

    async fn get_order_status(&self, order_id: &str) -> Result<OrderStatus> {
        let state = self.state.lock().unwrap();
        Ok(state.orders.iter()
            .find(|o| o.order_id == order_id)
            .map(|o| o.status.clone())
            .unwrap_or(OrderStatus::Cancelled))
    }

    async fn portfolio(&self) -> Result<PortfolioView> {
        let state = self.state.lock().unwrap();
        let positions: Vec<PositionView> = state.positions.values().cloned().collect();
        let total_value = positions.iter().map(|p| p.total_value).sum::<f64>() + state.balance;
        let total_pnl = positions.iter().map(|p| p.pnl).sum();

        Ok(PortfolioView {
            account_id: self.account_id.clone(),
            total_balance: state.balance,
            available_balance: state.balance,
            positions,
            total_pnl,
            total_value,
        })
    }

    async fn balance(&self) -> Result<f64> {
        let state = self.state.lock().unwrap();
        Ok(state.balance)
    }

    async fn position(&self, instrument: &str) -> Result<Option<PositionView>> {
        let state = self.state.lock().unwrap();
        Ok(state.positions.get(instrument).cloned())
    }

    fn account_id(&self) -> &str {
        &self.account_id
    }
}
