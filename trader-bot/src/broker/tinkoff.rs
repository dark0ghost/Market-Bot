use anyhow::{Result, anyhow};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use std::sync::atomic::{AtomicU64, Ordering};
use t_invest_sdk::{
    TInvestSdk,
    api::{MoneyValue, OpenSandboxAccountRequest, SandboxPayInRequest},
};

use crate::client::{MarketDataService, OrderBookService, PortfolioService};
use crate::core::*;

/// Map a Tinkoff `OrderDirection` enum value into our `OrderAction`.
fn map_tinkoff_direction(direction: i32) -> OrderAction {
    use t_invest_sdk::api::OrderDirection;
    match OrderDirection::try_from(direction).unwrap_or(OrderDirection::Unspecified) {
        OrderDirection::Buy => OrderAction::Buy,
        OrderDirection::Sell => OrderAction::Sell,
        OrderDirection::Unspecified => OrderAction::Buy, // unknown → conservative default
    }
}

/// Map a Tinkoff `OrderExecutionReportStatus` into our `OrderStatus`.
fn map_tinkoff_status(status: i32) -> OrderStatus {
    use t_invest_sdk::api::OrderExecutionReportStatus;
    match OrderExecutionReportStatus::try_from(status)
        .unwrap_or(OrderExecutionReportStatus::ExecutionReportStatusUnspecified)
    {
        OrderExecutionReportStatus::ExecutionReportStatusFill => OrderStatus::Filled,
        OrderExecutionReportStatus::ExecutionReportStatusRejected => OrderStatus::Rejected,
        OrderExecutionReportStatus::ExecutionReportStatusCancelled => OrderStatus::Cancelled,
        OrderExecutionReportStatus::ExecutionReportStatusNew => OrderStatus::New,
        OrderExecutionReportStatus::ExecutionReportStatusPartiallyfill => {
            OrderStatus::PartiallyFilled
        }
        OrderExecutionReportStatus::ExecutionReportStatusUnspecified => OrderStatus::Pending,
    }
}

/// Token-bucket rate limiter for Tinkoff API calls.
///
/// Tinkoff limits:
/// - Market data: ~100 requests/sec
/// - Trading: ~50 requests/sec per account
/// - Portfolio: ~20 requests/sec
struct RateLimiter {
    /// Max tokens (burst capacity)
    max_tokens: u64,
    /// Tokens added per second
    refill_rate: f64,
    /// Current tokens available
    tokens: AtomicU64,
    /// Last refill time
    last_refill: std::sync::atomic::AtomicU64,
}

impl RateLimiter {
    fn new(max_tokens: u64, tokens_per_second: f64) -> Self {
        Self {
            max_tokens,
            refill_rate: tokens_per_second,
            tokens: AtomicU64::new(max_tokens),
            last_refill: std::sync::atomic::AtomicU64::new(
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_micros() as u64,
            ),
        }
    }

    fn try_acquire(&self) -> bool {
        loop {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_micros() as u64;
            let last = self.last_refill.load(Ordering::Acquire);
            let tokens = self.tokens.load(Ordering::Acquire);

            let elapsed = if now > last { now - last } else { 0 };
            // Refill is measured from `last` since the last time we consumed a token.
            let refilled = (elapsed as f64 * self.refill_rate / 1_000_000.0) as u64;
            let updated_tokens = tokens.saturating_add(refilled).min(self.max_tokens);

            if updated_tokens == 0 {
                // Bucket empty - wait for a refill tick.
                return false;
            }

            // Claim one token and advance last_refill to "now" so the next caller
            // only accrues refilled tokens for the time since this claim.
            match self.tokens.compare_exchange(
                tokens,
                updated_tokens - 1,
                Ordering::AcqRel,
                Ordering::Acquire,
            ) {
                Ok(_) => {
                    self.last_refill.store(now, Ordering::Release);
                    return true;
                }
                Err(_) => continue, // CAS failed, retry with fresh state
            }
        }
    }

    /// Wait until a token is available, with a timeout.
    async fn acquire_with_backoff(&self, max_retries: u32, base_delay_ms: u64) -> Result<()> {
        for i in 0..max_retries {
            if self.try_acquire() {
                return Ok(());
            }
            let delay = base_delay_ms * 2u64.pow(i);
            log::warn!(
                "Rate limit hit, retry {}/{} in {}ms",
                i + 1,
                max_retries,
                delay
            );
            tokio::time::sleep(std::time::Duration::from_millis(delay)).await;
        }
        Err(anyhow!("rate limit exceeded after {} retries", max_retries))
    }
}

/// Tinkoff implementation of the Broker trait.
/// Delegates to existing client wrappers.
pub struct TinkoffBroker {
    sdk: TInvestSdk,
    account_id: String,
    name: String,
    rate_limiter: std::sync::Arc<RateLimiter>,
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
            rate_limiter: std::sync::Arc::new(RateLimiter::new(50, 50.0)),
        })
    }

    pub fn from_sdk(sdk: TInvestSdk, account_id: String) -> Self {
        TinkoffBroker {
            sdk,
            account_id: account_id.clone(),
            name: format!("tinkoff_{}", account_id),
            rate_limiter: std::sync::Arc::new(RateLimiter::new(50, 50.0)),
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
        self.rate_limiter.acquire_with_backoff(3, 100).await?;

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
        self.rate_limiter.acquire_with_backoff(3, 200).await?;

        use t_invest_sdk::api::*;

        let direction = match request.action {
            OrderAction::Buy => OrderDirection::Buy,
            OrderAction::Sell => OrderDirection::Sell,
        };
        let tinkoff_order_type = match request.order_type {
            crate::core::OrderType::Limit => t_invest_sdk::api::OrderType::Limit,
            crate::core::OrderType::Market => t_invest_sdk::api::OrderType::Market,
        };

        #[allow(deprecated)]
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
        self.rate_limiter.acquire_with_backoff(3, 100).await?;

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
                // Map the real direction instead of hardcoding Buy.
                action: map_tinkoff_direction(o.direction),
                quantity: o.lots_requested as i32,
                filled_quantity: o.lots_executed as i32,
                price: None,
                avg_fill_price: None,
                // Map the real execution status instead of hardcoding New.
                status: map_tinkoff_status(o.execution_report_status),
                created_at: Utc::now(),
                message: "Fetched from Tinkoff".to_string(),
            })
            .collect())
    }

    async fn place_stop_order(&self, request: StopOrderRequest) -> Result<OrderResponse> {
        self.rate_limiter.acquire_with_backoff(3, 200).await?;

        use t_invest_sdk::api::*;

        // A stop-loss/take-profit closes a long position by selling, or a short by buying.
        let direction = match request.action {
            OrderAction::Buy => StopOrderDirection::Buy,
            OrderAction::Sell => StopOrderDirection::Sell,
        };
        let stop_order_type = match request.kind {
            crate::core::StopOrderKind::StopLoss => StopOrderType::StopLoss,
            crate::core::StopOrderKind::TakeProfit => StopOrderType::TakeProfit,
        };

        #[allow(deprecated)]
        let req = PostStopOrderRequest {
            figi: Some(request.instrument.clone()),
            quantity: request.quantity as i64,
            // Limit price for the child order; None ⇒ market child order.
            price: request.price.map(|p| Quotation {
                units: p as i64,
                nano: ((p.fract() * 1_000_000_000.0) as i32),
            }),
            stop_price: Some(Quotation {
                units: request.stop_price as i64,
                nano: ((request.stop_price.fract() * 1_000_000_000.0) as i32),
            }),
            direction: direction as i32,
            account_id: self.account_id.clone(),
            expiration_type: StopOrderExpirationType::GoodTillCancel as i32,
            stop_order_type: stop_order_type as i32,
            expire_date: None,
            instrument_id: request.instrument.clone(),
            exchange_order_type: 0,
            take_profit_type: 0,
            trailing_data: None,
            price_type: 0,
            order_id: request
                .client_order_id
                .unwrap_or_else(|| format!("stop_{}_{}", Utc::now().timestamp_nanos_opt().unwrap_or(0), std::process::id())),
            confirm_margin_trade: false,
            instant_execution: None,
        };

        let response = self.sdk.stop_orders().post_stop_order(req).await?;
        let r = response.into_inner();

        Ok(OrderResponse {
            order_id: r.stop_order_id,
            client_order_id: Some(r.order_request_id),
            instrument: request.instrument,
            action: request.action,
            quantity: request.quantity,
            filled_quantity: 0,
            price: request.price,
            avg_fill_price: None,
            status: OrderStatus::New,
            created_at: Utc::now(),
            message: format!("Stop {:?} placed via Tinkoff", request.kind),
        })
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
        self.rate_limiter.acquire_with_backoff(3, 100).await?;

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
