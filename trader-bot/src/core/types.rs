use chrono::{DateTime, Utc};
pub use rust_decimal::Decimal;
use rust_decimal::prelude::{FromPrimitive, ToPrimitive};
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::collections::HashMap;

// ─── Money helpers ───────────────────────────────────────────────────
//
// Broker-facing money fields (order prices, balances, position values) use
// `Decimal` to avoid floating-point drift on money. Market-data prices (OHLCV,
// order book) stay as `f64` since they feed indicator math. These helpers do
// the boundary conversions.

/// Convert an `f64` into `Decimal` for broker-facing money fields.
/// Non-finite values (NaN/inf) fall back to zero.
pub fn f64_to_decimal(v: f64) -> Decimal {
    Decimal::from_f64(v).unwrap_or_default()
}

/// Convert a `Decimal` money value back to `f64` for internal analysis math.
pub fn decimal_to_f64(d: Decimal) -> f64 {
    d.to_f64().unwrap_or_default()
}

/// Parse a `Decimal` from a broker API money string, falling back to zero.
pub fn decimal_from_str(s: &str) -> Decimal {
    s.trim().parse().unwrap_or_default()
}

// ─── Market Data Types ───────────────────────────────────────────────

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum CandleInterval {
    Min1,
    Min5,
    Min15,
    Hour1,
    Hour4,
    Day1,
    Week1,
    Month1,
}

impl CandleInterval {
    pub const fn as_str(&self) -> &'static str {
        match self {
            CandleInterval::Min1 => "1m",
            CandleInterval::Min5 => "5m",
            CandleInterval::Min15 => "15m",
            CandleInterval::Hour1 => "1h",
            CandleInterval::Hour4 => "4h",
            CandleInterval::Day1 => "1d",
            CandleInterval::Week1 => "1w",
            CandleInterval::Month1 => "1M",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Candle {
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: f64,
    pub time: DateTime<Utc>,
    pub ticker: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderBookLevel {
    pub price: f64,
    pub volume: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderBook {
    pub figi: String,
    pub bids: Vec<OrderBookLevel>,
    pub asks: Vec<OrderBookLevel>,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiquidityInfo {
    pub total_bid_volume: f64,
    pub total_ask_volume: f64,
    pub bid_ask_ratio: f64,
    pub spread: f64,
}

// ─── Order Types ─────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum OrderAction {
    Buy,
    Sell,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum OrderType {
    Limit,
    Market,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum OrderStatus {
    New,
    PartiallyFilled,
    Filled,
    Cancelled,
    Rejected,
    Pending,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderRequest {
    pub instrument: String,
    pub action: OrderAction,
    pub order_type: OrderType,
    pub quantity: i32,
    pub price: Option<Decimal>,
    pub account_id: String,
    pub client_order_id: Option<String>,
}

/// Stop order kind: StopLoss or TakeProfit (broker-side).
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum StopOrderKind {
    StopLoss,
    TakeProfit,
}

/// Broker-side stop order request. For a long position: StopLoss direction = Sell,
/// stop_price below current; TakeProfit direction = Sell, stop_price above current.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StopOrderRequest {
    pub instrument: String,
    /// Direction of the child order triggered when stop_price is hit.
    pub action: OrderAction,
    pub kind: StopOrderKind,
    pub quantity: i32,
    /// Activation price (stop price).
    pub stop_price: Decimal,
    /// Optional limit price for stop-limit orders; if None a market order is used.
    pub price: Option<Decimal>,
    pub account_id: String,
    pub client_order_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderResponse {
    pub order_id: String,
    pub client_order_id: Option<String>,
    pub instrument: String,
    pub action: OrderAction,
    pub quantity: i32,
    pub filled_quantity: i32,
    pub price: Option<Decimal>,
    pub avg_fill_price: Option<Decimal>,
    pub status: OrderStatus,
    pub created_at: DateTime<Utc>,
    pub message: String,
}

// ─── Portfolio Types ─────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PositionView {
    pub instrument: String,
    pub quantity: i32,
    pub average_price: Decimal,
    pub current_price: Decimal,
    pub pnl: Decimal,
    pub pnl_pct: Decimal,
    pub total_value: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortfolioView {
    pub account_id: String,
    pub total_balance: Decimal,
    pub available_balance: Decimal,
    pub positions: Vec<PositionView>,
    pub total_pnl: Decimal,
    pub total_value: Decimal,
}

// ─── Instrument Types ────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstrumentInfo {
    pub ticker: String,
    pub figi: String,
    pub name: String,
    pub currency: String,
    pub instrument_type: InstrumentKind,
    pub lot_size: i32,
    pub min_price_step: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum InstrumentKind {
    Share,
    Bond,
    Etf,
    Currency,
    Future,
    Option,
    Crypto,
}

// ─── Broker Types ────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum BrokerKind {
    Tinkoff,
    Mock,
    Alor,
    Binance,
    ByBit,
    InteractiveBrokers,
    Other(String),
}

impl BrokerKind {
    pub fn as_str(&self) -> Cow<'_, str> {
        match self {
            BrokerKind::Tinkoff => Cow::Borrowed("tinkoff"),
            BrokerKind::Mock => Cow::Borrowed("mock"),
            BrokerKind::Alor => Cow::Borrowed("alor"),
            BrokerKind::Binance => Cow::Borrowed("binance"),
            BrokerKind::ByBit => Cow::Borrowed("bybit"),
            BrokerKind::InteractiveBrokers => Cow::Borrowed("ib"),
            BrokerKind::Other(s) => Cow::Borrowed(s.as_str()),
        }
    }
}

// ─── Data Source Types ───────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DataSourceKind {
    Tinkoff,
    Finam,
    MoexIss,
    Yahoo,
    Polygon,
    Other(String),
}

// ─── Strategy Types ──────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum StrategyKind {
    Grid,
    Interval,
    Momentum,
    MeanReversion,
    Ai,
    PairsTrading,
    StatisticalArbitrage,
    Custom(String),
}

impl StrategyKind {
    pub fn as_str(&self) -> Cow<'_, str> {
        match self {
            StrategyKind::Grid => Cow::Borrowed("grid"),
            StrategyKind::Interval => Cow::Borrowed("interval"),
            StrategyKind::Momentum => Cow::Borrowed("momentum"),
            StrategyKind::MeanReversion => Cow::Borrowed("mean_reversion"),
            StrategyKind::Ai => Cow::Borrowed("ai"),
            StrategyKind::PairsTrading => Cow::Borrowed("pairs"),
            StrategyKind::StatisticalArbitrage => Cow::Borrowed("stat_arb"),
            StrategyKind::Custom(s) => Cow::Borrowed(s.as_str()),
        }
    }
}

// ─── Time Series / Analytics Types ───────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Signal {
    pub ticker: String,
    pub timestamp: DateTime<Utc>,
    pub action: OrderAction,
    pub confidence: f64,
    pub price: f64,
    pub source: String,
    pub metadata: HashMap<String, String>,
}

// ─── Conversion helpers ──────────────────────────────────────────────

pub const fn to_f64(units: i64, nano: i32) -> f64 {
    units as f64 + nano as f64 / 1_000_000_000.0
}

pub const fn to_quotation(value: f64) -> (i64, i32) {
    let units = value as i64;
    let nano = ((value.fract() * 1_000_000_000.0).round() as i32).abs();
    (units, nano)
}
