use anyhow::{Result, anyhow};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use t_invest_sdk::{
    TInvestSdk,
    api::{MoneyValue, OpenSandboxAccountRequest, SandboxPayInRequest},
};

use crate::client::{MarketDataService, OrderBookService, PortfolioService};
use crate::core::*;

/// Tinkoff implementation of the Broker trait.
/// Delegates to existing client wrappers.
pub struct TinkoffBroker {
    sdk: TInvestSdk,
    account_id: String,
    name: String,
}

impl TinkoffBroker {
    pub async fn new(
        token: &str,
        account_id: Option<String>,
        sandbox: bool,
        open_account: bool,
        pay_in_amount: f64,
    ) -> Result<Self> {
        use t_invest_sdk::Environment;
        let env = if sandbox {
            Environment::Sandbox
        } else {
            Environment::Production
        };
        let sdk = TInvestSdk::new(token, env).await?;

        let account_id =
            if sandbox && open_account && account_id.as_deref().is_none_or(|s| s.is_empty()) {
                let mut sc = sdk.sandbox();
                let resp = sc
                    .open_sandbox_account(OpenSandboxAccountRequest { name: None })
                    .await
                    .map_err(|e| anyhow!("Sandbox OpenSandboxAccount failed: {}", e))?;
                let id = resp.into_inner().account_id;
                log::info!("Sandbox: opened account {}", id);

                if pay_in_amount > 0.0 {
                    let units = pay_in_amount as i64;
                    let nano = (pay_in_amount.fract() * 1_000_000_000.0) as i32;
                    let pay_req = SandboxPayInRequest {
                        account_id: id.clone(),
                        amount: Some(MoneyValue {
                            currency: "RUB".to_string(),
                            units,
                            nano,
                        }),
                    };
                    sc.sandbox_pay_in(pay_req)
                        .await
                        .map_err(|e| anyhow!("Sandbox PayIn failed: {}", e))?;
                    log::info!(
                        "Sandbox: deposited {:.2} RUB into account {}",
                        pay_in_amount,
                        id
                    );
                }
                id
            } else {
                account_id.unwrap_or_default()
            };

        Ok(TinkoffBroker {
            sdk,
            account_id: account_id.clone(),
            name: format!("tinkoff_{}", account_id),
        })
    }

    pub fn from_sdk(sdk: TInvestSdk, account_id: String) -> Self {
        TinkoffBroker {
            sdk,
            account_id: account_id.clone(),
            name: format!("tinkoff_{}", account_id),
        }
    }

    pub fn sdk(&self) -> TInvestSdk {
        self.sdk.clone()
    }

    fn market_data(&self) -> MarketDataService {
        MarketDataService::new(self.sdk.clone())
    }

    fn portfolio_service(&self) -> PortfolioService {
        PortfolioService::new(self.sdk.clone(), self.account_id.clone())
    }

    fn order_book_service(&self) -> OrderBookService {
        OrderBookService::new(self.sdk.clone())
    }
}

#[async_trait]
impl Broker for TinkoffBroker {
    fn name(&self) -> &str {
        &self.name
    }

    fn broker_kind(&self) -> BrokerKind {
        BrokerKind::Tinkoff
    }

    async fn candles(
        &self,
        instrument: &str,
        interval: CandleInterval,
        _days: u32,
    ) -> Result<Vec<Candle>> {
        use t_invest_sdk::api::CandleInterval as TI;
        let ti = match interval {
            CandleInterval::Min1 => TI::CandleInterval1Min,
            CandleInterval::Min5 => TI::CandleInterval5Min,
            CandleInterval::Min15 => TI::CandleInterval15Min,
            CandleInterval::Hour1 => t_invest_sdk::api::CandleInterval::Hour,
            CandleInterval::Hour4 => TI::CandleInterval4Hour,
            CandleInterval::Day1 => t_invest_sdk::api::CandleInterval::Day,
            _ => t_invest_sdk::api::CandleInterval::Day,
        };

        let sdk = self.market_data();
        let hist = sdk
            .get_historical_candles(instrument, ti, _days.max(1))
            .await?;

        Ok(hist
            .iter()
            .filter_map(|c| {
                let time = c
                    .time
                    .as_ref()
                    .and_then(|t| DateTime::from_timestamp(t.seconds, t.nanos as u32))?;
                Some(Candle {
                    open: crate::client::market_data::extract_price(&c.open).ok()?,
                    high: crate::client::market_data::extract_price(&c.high).ok()?,
                    low: crate::client::market_data::extract_price(&c.low).ok()?,
                    close: crate::client::market_data::extract_price(&c.close).ok()?,
                    volume: c.volume as f64,
                    time,
                    ticker: instrument.to_string(),
                })
            })
            .collect())
    }

    async fn last_price(&self, instrument: &str) -> Result<f64> {
        self.market_data().get_last_price(instrument).await
    }

    async fn order_book(&self, instrument: &str, depth: u32) -> Result<OrderBook> {
        let ob = self.order_book_service();
        let book = ob.get_order_book(instrument, depth as i32).await?;

        Ok(OrderBook {
            figi: instrument.to_string(),
            bids: book
                .bids
                .iter()
                .map(|b| OrderBookLevel {
                    price: b.price,
                    volume: b.quantity as f64,
                })
                .collect(),
            asks: book
                .asks
                .iter()
                .map(|a| OrderBookLevel {
                    price: a.price,
                    volume: a.quantity as f64,
                })
                .collect(),
            timestamp: Utc::now(),
        })
    }

    async fn liquidity(&self, instrument: &str, depth: u32) -> Result<LiquidityInfo> {
        let ob = self.order_book(instrument, depth).await?;
        let total_bid: f64 = ob.bids.iter().map(|l| l.price * l.volume).sum();
        let total_ask: f64 = ob.asks.iter().map(|l| l.price * l.volume).sum();
        let spread = ob.asks.first().map(|a| a.price).unwrap_or(0.0)
            - ob.bids.first().map(|b| b.price).unwrap_or(0.0);
        Ok(LiquidityInfo {
            total_bid_volume: total_bid,
            total_ask_volume: total_ask,
            bid_ask_ratio: if total_ask > 0.0 {
                total_bid / total_ask
            } else {
                1.0
            },
            spread: spread.max(0.0),
        })
    }

    async fn place_order(&self, request: OrderRequest) -> Result<OrderResponse> {
        use t_invest_sdk::api::*;

        let direction = match request.action {
            OrderAction::Buy => OrderDirection::Buy,
            OrderAction::Sell => OrderDirection::Sell,
        };
        let tinkoff_order_type = match request.order_type {
            crate::core::OrderType::Limit => t_invest_sdk::api::OrderType::Limit,
            crate::core::OrderType::Market => t_invest_sdk::api::OrderType::Market,
        };

        let req = PostOrderRequest {
            figi: Some(request.instrument.clone()),
            quantity: request.quantity as i64,
            price: request.price.map(|p| Quotation {
                units: p as i64,
                nano: ((p.fract() * 1_000_000_000.0) as i32),
            }),
            direction: direction as i32,
            account_id: self.account_id.clone(),
            order_type: tinkoff_order_type as i32,
            order_id: request
                .client_order_id
                .unwrap_or_else(|| format!("order_{}", Utc::now().timestamp())),
            instrument_id: request.instrument.clone(),
            confirm_margin_trade: false,
            time_in_force: 0,
            price_type: 0,
        };

        let response = self.sdk.orders().post_order(req).await?;
        let r = response.into_inner();

        Ok(OrderResponse {
            order_id: r.order_id,
            client_order_id: None,
            instrument: request.instrument,
            action: request.action,
            quantity: request.quantity,
            filled_quantity: r.lots_executed as i32,
            price: request.price,
            avg_fill_price: None,
            status: OrderStatus::New,
            created_at: Utc::now(),
            message: "Placed via Tinkoff".to_string(),
        })
    }

    async fn cancel_order(&self, order_id: &str) -> Result<()> {
        use t_invest_sdk::api::*;
        let req = CancelOrderRequest {
            account_id: self.account_id.clone(),
            order_id: order_id.to_string(),
            order_id_type: Some(0),
        };
        self.sdk.orders().cancel_order(req).await?;
        Ok(())
    }

    async fn get_orders(&self, _instrument: Option<&str>) -> Result<Vec<OrderResponse>> {
        use t_invest_sdk::api::*;
        let req = GetOrdersRequest {
            account_id: self.account_id.clone(),
            advanced_filters: None,
        };
        let response = self.sdk.orders().get_orders(req).await?;
        let r = response.into_inner();

        Ok(r.orders
            .iter()
            .map(|o| OrderResponse {
                order_id: o.order_id.clone(),
                client_order_id: None,
                instrument: o.figi.clone(),
                action: OrderAction::Buy,
                quantity: o.lots_requested as i32,
                filled_quantity: o.lots_executed as i32,
                price: None,
                avg_fill_price: None,
                status: OrderStatus::New,
                created_at: Utc::now(),
                message: "Fetched from Tinkoff".to_string(),
            })
            .collect())
    }

    async fn get_order_status(&self, order_id: &str) -> Result<OrderStatus> {
        let orders = self.get_orders(None).await?;
        Ok(orders
            .iter()
            .find(|o| o.order_id == order_id)
            .map(|o| o.status.clone())
            .unwrap_or(OrderStatus::Cancelled))
    }

    async fn portfolio(&self) -> Result<PortfolioView> {
        let ps = self.portfolio_service();
        let info = ps.get_portfolio().await?;

        let mut positions = Vec::new();
        for pos in &info.positions {
            positions.push(PositionView {
                instrument: pos.uid.clone(),
                quantity: pos.quantity as i32,
                average_price: pos.average_position_price,
                current_price: pos.current_price,
                pnl: (pos.current_price - pos.average_position_price) * pos.quantity as f64,
                pnl_pct: if pos.average_position_price > 0.0 {
                    (pos.current_price - pos.average_position_price) / pos.average_position_price
                        * 100.0
                } else {
                    0.0
                },
                total_value: pos.current_price * pos.quantity as f64,
            });
        }

        let total_value = positions.iter().map(|p| p.total_value).sum::<f64>() + info.total_amount;
        let total_pnl = positions.iter().map(|p| p.pnl).sum();

        Ok(PortfolioView {
            account_id: self.account_id.clone(),
            total_balance: info.total_amount,
            available_balance: ps.get_available_balance().await.unwrap_or(0.0),
            positions,
            total_pnl,
            total_value,
        })
    }

    async fn balance(&self) -> Result<f64> {
        self.portfolio_service().get_available_balance().await
    }

    async fn position(&self, instrument: &str) -> Result<Option<PositionView>> {
        let ps = self.portfolio_service();
        match ps.get_position(instrument).await? {
            Some(p) => {
                let current_price = self.last_price(instrument).await.unwrap_or(0.0);
                Ok(Some(PositionView {
                    instrument: instrument.to_string(),
                    quantity: p.quantity,
                    average_price: p.average_price,
                    current_price,
                    pnl: (current_price - p.average_price) * p.quantity as f64,
                    pnl_pct: if p.average_price > 0.0 {
                        (current_price - p.average_price) / p.average_price * 100.0
                    } else {
                        0.0
                    },
                    total_value: current_price * p.quantity as f64,
                }))
            }
            None => Ok(None),
        }
    }

    fn account_id(&self) -> &str {
        &self.account_id
    }
}
