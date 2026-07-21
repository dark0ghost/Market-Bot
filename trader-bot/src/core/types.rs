use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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
    pub fn as_str(&self) -> &'static str {
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
    pub price: Option<f64>,
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
    pub price: Option<f64>,
    pub avg_fill_price: Option<f64>,
    pub status: OrderStatus,
    pub created_at: DateTime<Utc>,
    pub message: String,
}

// ─── Portfolio Types ─────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PositionView {
    pub instrument: String,
    pub quantity: i32,
    pub average_price: f64,
    pub current_price: f64,
    pub pnl: f64,
    pub pnl_pct: f64,
    pub total_value: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortfolioView {
    pub account_id: String,
    pub total_balance: f64,
    pub available_balance: f64,
    pub positions: Vec<PositionView>,
    pub total_pnl: f64,
    pub total_value: f64,
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
    pub fn as_str(&self) -> &'static str {
        match self {
            BrokerKind::Tinkoff => "tinkoff",
            BrokerKind::Mock => "mock",
            BrokerKind::Alor => "alor",
            BrokerKind::Binance => "binance",
            BrokerKind::ByBit => "bybit",
            BrokerKind::InteractiveBrokers => "ib",
            BrokerKind::Other(s) => Box::leak(s.clone().into_boxed_str()),
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
    PairsTrading,
    StatisticalArbitrage,
    Custom(String),
}

impl StrategyKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            StrategyKind::Grid => "grid",
            StrategyKind::Interval => "interval",
            StrategyKind::Momentum => "momentum",
            StrategyKind::MeanReversion => "mean_reversion",
            StrategyKind::PairsTrading => "pairs",
            StrategyKind::StatisticalArbitrage => "stat_arb",
            StrategyKind::Custom(s) => Box::leak(s.clone().into_boxed_str()),
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

pub fn to_f64(units: i64, nano: i32) -> f64 {
    units as f64 + nano as f64 / 1_000_000_000.0
}

pub fn to_quotation(value: f64) -> (i64, i32) {
    let units = value as i64;
    let nano = ((value.fract() * 1_000_000_000.0).round() as i32).abs();
    (units, nano)
}
