pub mod config;
pub mod utils;
pub mod provider;
pub mod strategy;
pub mod client;
pub mod mcp;
pub mod instrument;
pub mod analysis;
pub mod agent;
pub mod execution;
pub mod error;
pub mod scanner;
pub mod backtest;
pub mod stream;
pub mod storage;
pub mod telemetry;
pub mod scheduler;

// ─── New Architecture Modules ────────────────────────────────────────

/// Broker-agnostic core types and traits
pub mod core;

/// Broker implementations (Tinkoff, Mock, etc.)
pub mod broker;

/// Data source implementations (Tinkoff, Finam, etc.)
pub mod datasource;

/// Strategy parameter optimizer
pub mod optimizer;

/// Web dashboard API
pub mod api;

/// ONNX-based ML inference (FinBERT, etc.)
pub mod ml_inference;

// ─── Re-exports for backward compatibility ───────────────────────────

pub use analysis::{Sentiment, NewsSentiment, NewsArticle};
pub use strategy::{GridStrategy, GridState, GridLevel, OrderSide};
pub use execution::{PositionManager, TradingExecutor, OrderAction, OrderResult, OrderStatus, TradeJournal, TradeRecord, SignalRecord};
pub use error::{BotError, OrderError, StrategyError, NewsAnalysisError};
pub use backtest::{BacktestConfig, BacktestResult, run_backtest, backtest_grid};

pub use analysis::technical::{MacdValues, BollingerValues, VolumeAnalysis, TechnicalAnalysis, TechnicalAnalyzer, Trend, Recommendation};
pub use analysis::fundamental::{ValuationMetrics, ProfitabilityMetrics, FinancialHealthMetrics, GrowthMetrics, DividendMetrics, FundamentalAnalysis, CompanyRating, FundamentalAnalyzer};
