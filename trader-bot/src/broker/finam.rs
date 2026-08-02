use anyhow::{Result, anyhow};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use reqwest::Client;
use serde::{Deserialize, Serialize};

use crate::core::*;

// ─── API Response Types (prefixed to avoid conflict with core types) ────

#[derive(Serialize)]
struct FinamAuthRequest {
    secret: String,
}

#[derive(Deserialize)]
struct FinamAuthResponse {
    token: String,
}

#[derive(Deserialize)]
struct FinamAccountResponse {
    #[allow(dead_code)]
    account_id: Option<String>,
    #[allow(dead_code)]
    equity: Option<FinamDecimal>,
    #[allow(dead_code)]
    unrealized_profit: Option<FinamDecimal>,
    positions: Option<Vec<FinamPosition>>,
    cash: Option<Vec<FinamMoney>>,
    #[allow(dead_code)]
    portfolio_mc: Option<FinamPortfolioMc>,
}

#[derive(Deserialize)]
struct FinamDecimal {
    value: Option<String>,
}

#[derive(Deserialize)]
struct FinamPosition {
    symbol: Option<String>,
    quantity: Option<FinamDecimal>,
    average_price: Option<FinamDecimal>,
    current_price: Option<FinamDecimal>,
    #[allow(dead_code)]
    unrealized_pnl: Option<FinamDecimal>,
}

#[derive(Deserialize)]
struct FinamMoney {
    #[allow(dead_code)]
    currency_code: Option<String>,
    units: Option<i64>,
    nanos: Option<i32>,
}

#[derive(Deserialize)]
struct FinamPortfolioMc {
    available_cash: Option<FinamDecimal>,
    #[allow(dead_code)]
    initial_margin: Option<FinamDecimal>,
    #[allow(dead_code)]
    maintenance_margin: Option<FinamDecimal>,
}

#[derive(Serialize)]
struct FinamPlaceOrderRequest {
    symbol: String,
    quantity: serde_json::Value,
    side: String,
    #[serde(rename = "type")]
    order_type: String,
    time_in_force: String,
    limit_price: Option<serde_json::Value>,
    client_order_id: Option<String>,
}

#[derive(Deserialize)]
struct FinamPlaceOrderResponse {
    order_id: Option<String>,
    status: Option<String>,
}

#[derive(Deserialize)]
struct FinamOrdersListResponse {
    orders: Option<Vec<FinamOrderItem>>,
}

#[derive(Deserialize)]
struct FinamOrderItem {
    order_id: Option<String>,
    symbol: Option<String>,
    side: Option<String>,
    #[serde(rename = "type")]
    order_type: Option<String>,
    quantity: Option<FinamDecimal>,
    limit_price: Option<FinamDecimal>,
    status: Option<String>,
    #[allow(dead_code)]
    executed_quantity: Option<FinamDecimal>,
}

#[derive(Deserialize)]
struct FinamBarsResponse {
    bars: Option<Vec<FinamBar>>,
}

#[derive(Deserialize)]
struct FinamBar {
    timestamp: Option<FinamTimestamp>,
    open: Option<FinamDecimal>,
    high: Option<FinamDecimal>,
    low: Option<FinamDecimal>,
    close: Option<FinamDecimal>,
    volume: Option<FinamDecimal>,
}

#[derive(Deserialize)]
struct FinamTimestamp {
    seconds: Option<i64>,
    nanos: Option<i32>,
}

#[derive(Deserialize)]
struct FinamLastQuoteResponse {
    #[allow(dead_code)]
    symbol: Option<String>,
    bid: Option<serde_json::Value>,
    ask: Option<serde_json::Value>,
    #[allow(dead_code)]
    last_price: Option<FinamDecimal>,
    #[allow(dead_code)]
    timestamp: Option<FinamTimestamp>,
}

#[derive(Deserialize)]
struct FinamOrderBookResponse {
    #[allow(dead_code)]
    symbol: Option<String>,
    bids: Option<Vec<FinamOrderBookLevel>>,
    asks: Option<Vec<FinamOrderBookLevel>>,
}

#[derive(Deserialize)]
struct FinamOrderBookLevel {
    price: Option<FinamDecimal>,
    volume: Option<FinamDecimal>,
}

#[derive(Deserialize)]
struct FinamAssetsResponse {
    assets: Option<Vec<FinamAsset>>,
}

#[derive(Deserialize)]
struct FinamAsset {
    symbol: Option<String>,
    ticker: Option<String>,
    #[allow(dead_code)]
    mic: Option<String>,
    #[allow(dead_code)]
    name: Option<String>,
    #[allow(dead_code)]
    r#type: Option<String>,
}

// ─── Helpers ─────────────────────────────────────────────────────────

fn parse_decimal(d: &Option<FinamDecimal>) -> Option<f64> {
    let v = d.as_ref()?.value.as_ref()?;
    v.parse::<f64>().ok()
}

fn parse_money(m: &Option<Vec<FinamMoney>>) -> f64 {
    m.as_ref()
        .and_then(|arr| arr.first())
        .map(|c| {
            let units = c.units.unwrap_or(0) as f64;
            let nanos = c.nanos.unwrap_or(0) as f64 / 1_000_000_000.0;
            units + nanos
        })
        .unwrap_or(0.0)
}

fn parse_timestamp(ts: &Option<FinamTimestamp>) -> Option<DateTime<Utc>> {
    let t = ts.as_ref()?;
    DateTime::from_timestamp(t.seconds.unwrap_or(0), t.nanos.unwrap_or(0) as u32)
}

fn symbol_to_figi(symbol: &str) -> String {
    symbol.replace('@', "_")
}

fn figi_to_symbol(figi: &str) -> String {
    if figi.contains('@') {
        figi.to_string()
    } else {
        figi.replace('_', "@")
    }
}

fn map_order_status(s: Option<&str>) -> OrderStatus {
    match s {
        Some("ORDER_STATUS_NEW") => OrderStatus::New,
        Some("ORDER_STATUS_FILLED") => OrderStatus::Filled,
        Some("ORDER_STATUS_PARTIALLY_FILLED") => OrderStatus::PartiallyFilled,
        Some("ORDER_STATUS_REJECTED") => OrderStatus::Rejected,
        Some("ORDER_STATUS_CANCELED") => OrderStatus::Cancelled,
        _ => OrderStatus::Pending,
    }
}

// ─── Broker ──────────────────────────────────────────────────────────

pub struct FinamBroker {
    http: Client,
    base_url: String,
    account_id: String,
    token: String,
    name: String,
}

impl FinamBroker {
    pub async fn new(api_token: &str, account_id: String) -> Result<Self> {
        let http = Client::builder().user_agent("ai-trade-bot/0.2").build()?;

        let base_url = "https://api.finam.ru".to_string();

        let auth_resp: FinamAuthResponse = http
            .post(format!("{}/v1/sessions", base_url))
            .json(&FinamAuthRequest {
                secret: api_token.to_string(),
            })
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;

        let token = auth_resp.token;
        let name = format!("finam_{}", account_id);
        log::info!("Finam auth successful, token obtained");

        Ok(FinamBroker {
            http,
            base_url,
            account_id,
            token,
            name,
        })
    }

    fn headers(&self) -> reqwest::header::HeaderMap {
        let mut h = reqwest::header::HeaderMap::new();
        let auth_value = format!("Bearer {}", self.token);
        h.insert(
            reqwest::header::AUTHORIZATION,
            auth_value
                .parse()
                .unwrap_or_else(|e| {
                    log::error!("Failed to parse authorization header: {:?}, using fallback", e);
                    reqwest::header::HeaderValue::from_static("Bearer invalid")
                }),
        );
        h
    }
}

#[async_trait]
impl Broker for FinamBroker {
    fn name(&self) -> &str {
        &self.name
    }

    fn broker_kind(&self) -> BrokerKind {
        BrokerKind::Other("finam".to_string())
    }

    async fn candles(
        &self,
        instrument: &str,
        interval: CandleInterval,
        days: u32,
    ) -> Result<Vec<Candle>> {
        let symbol = figi_to_symbol(instrument);
        let timeframe = match interval {
            CandleInterval::Min1 => "TIME_FRAME_M1",
            CandleInterval::Min5 => "TIME_FRAME_M5",
            CandleInterval::Min15 => "TIME_FRAME_M15",
            CandleInterval::Hour1 => "TIME_FRAME_H1",
            CandleInterval::Hour4 => "TIME_FRAME_H4",
            CandleInterval::Day1 => "TIME_FRAME_D",
            _ => "TIME_FRAME_D",
        };

        let end = Utc::now();
        let start = end - chrono::Duration::days(days as i64);

        let url = format!(
            "{}/v1/instruments/{}/bars?timeframe={}&interval.start_time={}&interval.end_time={}",
            self.base_url,
            symbol,
            timeframe,
            start.format("%Y-%m-%dT%H:%M:%SZ"),
            end.format("%Y-%m-%dT%H:%M:%SZ"),
        );

        let resp: FinamBarsResponse = self
            .http
            .get(&url)
            .headers(self.headers())
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;

        let bars = resp.bars.unwrap_or_default();
        let mut candles = Vec::with_capacity(bars.len());

        for bar in bars {
            let time =
                parse_timestamp(&bar.timestamp).ok_or_else(|| anyhow!("Invalid bar timestamp"))?;
            candles.push(Candle {
                open: parse_decimal(&bar.open).unwrap_or(0.0),
                high: parse_decimal(&bar.high).unwrap_or(0.0),
                low: parse_decimal(&bar.low).unwrap_or(0.0),
                close: parse_decimal(&bar.close).unwrap_or(0.0),
                volume: parse_decimal(&bar.volume).unwrap_or(0.0),
                time,
                ticker: instrument.to_string(),
            });
        }

        Ok(candles)
    }

    async fn last_price(&self, instrument: &str) -> Result<f64> {
        let symbol = figi_to_symbol(instrument);
        let url = format!("{}/v1/instruments/{}/lastquote", self.base_url, symbol);

        let resp: FinamLastQuoteResponse = self
            .http
            .get(&url)
            .headers(self.headers())
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;

        if let Some(price) = parse_decimal(&resp.last_price) {
            return Ok(price);
        }

        let bid = resp
            .bid
            .as_ref()
            .and_then(|v| v.as_str())
            .and_then(|s| s.parse::<f64>().ok());
        let ask = resp
            .ask
            .as_ref()
            .and_then(|v| v.as_str())
            .and_then(|s| s.parse::<f64>().ok());

        match (bid, ask) {
            (Some(b), Some(a)) => Ok((b + a) / 2.0),
            (Some(b), None) => Ok(b),
            (None, Some(a)) => Ok(a),
            (None, None) => Err(anyhow!("No price data for {}", instrument)),
        }
    }

    async fn order_book(&self, instrument: &str, depth: u32) -> Result<OrderBook> {
        let symbol = figi_to_symbol(instrument);
        let url = format!(
            "{}/v1/instruments/{}/orderbook?depth={}",
            self.base_url, symbol, depth
        );

        let resp: FinamOrderBookResponse = self
            .http
            .get(&url)
            .headers(self.headers())
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;

        let bids = resp
            .bids
            .unwrap_or_default()
            .iter()
            .map(|b| OrderBookLevel {
                price: parse_decimal(&b.price).unwrap_or(0.0),
                volume: parse_decimal(&b.volume).unwrap_or(0.0),
            })
            .collect();

        let asks = resp
            .asks
            .unwrap_or_default()
            .iter()
            .map(|a| OrderBookLevel {
                price: parse_decimal(&a.price).unwrap_or(0.0),
                volume: parse_decimal(&a.volume).unwrap_or(0.0),
            })
            .collect();

        Ok(OrderBook {
            figi: instrument.to_string(),
            bids,
            asks,
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
        let symbol = figi_to_symbol(&request.instrument);
        let side = match request.action {
            OrderAction::Buy => "SIDE_BUY",
            OrderAction::Sell => "SIDE_SELL",
        };
        let order_type = match request.order_type {
            OrderType::Limit => "ORDER_TYPE_LIMIT",
            OrderType::Market => "ORDER_TYPE_MARKET",
        };

        let mut req = FinamPlaceOrderRequest {
            symbol: symbol.clone(),
            quantity: serde_json::json!({"value": request.quantity.to_string()}),
            side: side.to_string(),
            order_type: order_type.to_string(),
            time_in_force: "TIME_IN_FORCE_DAY".to_string(),
            limit_price: None,
            client_order_id: request.client_order_id.clone(),
        };

        if let Some(price) = request.price {
            req.limit_price = Some(serde_json::json!({"value": price.to_string()}));
        }

        let url = format!("{}/v1/accounts/{}/orders", self.base_url, self.account_id);

        let resp: FinamPlaceOrderResponse = self
            .http
            .post(&url)
            .headers(self.headers())
            .json(&req)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;

        let status = map_order_status(resp.status.as_deref());

        Ok(OrderResponse {
            order_id: resp.order_id.unwrap_or_default(),
            client_order_id: request.client_order_id,
            instrument: request.instrument,
            action: request.action,
            quantity: request.quantity,
            filled_quantity: 0,
            price: request.price,
            avg_fill_price: None,
            status,
            created_at: Utc::now(),
            message: "Placed via Finam".to_string(),
        })
    }

    async fn cancel_order(&self, order_id: &str) -> Result<()> {
        let url = format!(
            "{}/v1/accounts/{}/orders/{}",
            self.base_url, self.account_id, order_id
        );

        self.http
            .delete(&url)
            .headers(self.headers())
            .send()
            .await?
            .error_for_status()?;

        Ok(())
    }

    async fn get_orders(&self, _instrument: Option<&str>) -> Result<Vec<OrderResponse>> {
        let url = format!("{}/v1/accounts/{}/orders", self.base_url, self.account_id);

        let resp: FinamOrdersListResponse = self
            .http
            .get(&url)
            .headers(self.headers())
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;

        let orders = resp.orders.unwrap_or_default();
        Ok(orders
            .iter()
            .map(|o| {
                let action = match o.side.as_deref() {
                    Some("SIDE_BUY") => OrderAction::Buy,
                    _ => OrderAction::Sell,
                };
                let status = map_order_status(o.status.as_deref());

                OrderResponse {
                    order_id: o.order_id.clone().unwrap_or_default(),
                    client_order_id: None,
                    instrument: symbol_to_figi(o.symbol.as_deref().unwrap_or("")),
                    action,
                    quantity: parse_decimal(&o.quantity).unwrap_or(0.0) as i32,
                    filled_quantity: 0,
                    price: parse_decimal(&o.limit_price),
                    avg_fill_price: None,
                    status,
                    created_at: Utc::now(),
                    message: "Fetched from Finam".to_string(),
                }
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
        let url = format!("{}/v1/accounts/{}", self.base_url, self.account_id);

        let resp: FinamAccountResponse = self
            .http
            .get(&url)
            .headers(self.headers())
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;

        let cash = parse_money(&resp.cash);
        let positions = resp.positions.unwrap_or_default();
        let mut pos_views = Vec::new();

        for p in &positions {
            let qty = parse_decimal(&p.quantity).unwrap_or(0.0) as i32;
            let avg = parse_decimal(&p.average_price).unwrap_or(0.0);
            let cur = parse_decimal(&p.current_price).unwrap_or(0.0);

            pos_views.push(PositionView {
                instrument: symbol_to_figi(p.symbol.as_deref().unwrap_or("")),
                quantity: qty,
                average_price: avg,
                current_price: cur,
                pnl: (cur - avg) * qty as f64,
                pnl_pct: if avg > 0.0 {
                    (cur - avg) / avg * 100.0
                } else {
                    0.0
                },
                total_value: cur * qty as f64,
            });
        }

        let total_value: f64 = pos_views.iter().map(|p| p.total_value).sum();
        let total_pnl: f64 = pos_views.iter().map(|p| p.pnl).sum();

        let available = resp
            .portfolio_mc
            .as_ref()
            .and_then(|mc| parse_decimal(&mc.available_cash))
            .unwrap_or(cash);

        Ok(PortfolioView {
            account_id: self.account_id.clone(),
            total_balance: cash + total_value,
            available_balance: available,
            positions: pos_views,
            total_pnl,
            total_value,
        })
    }

    async fn balance(&self) -> Result<f64> {
        let portfolio = self.portfolio().await?;
        Ok(portfolio.available_balance)
    }

    async fn position(&self, instrument: &str) -> Result<Option<PositionView>> {
        let portfolio = self.portfolio().await?;
        Ok(portfolio
            .positions
            .into_iter()
            .find(|p| p.instrument == instrument))
    }

    fn account_id(&self) -> &str {
        &self.account_id
    }
}
