//! AI Trade Bot Library
//!
//! Multi-component AI-powered trading system

pub mod config;
pub mod utils;
pub mod strategy;
pub mod client;
pub mod mcp;
pub mod instrument;
pub mod analysis;
pub mod agent;
pub mod execution;
pub mod error;

// Re-export main types
pub use analysis::{Sentiment, NewsSentiment, NewsArticle};
pub use strategy::{GridStrategy, GridState, GridLevel, OrderSide};
pub use execution::{PositionManager, OrderAction, OrderResult, OrderStatus};
pub use error::{BotError, OrderError, StrategyError, NewsAnalysisError};

// Re-export analysis submodules for integration tests
pub use analysis::technical::{MacdValues, BollingerValues, VolumeAnalysis, TechnicalAnalysis, TechnicalAnalyzer, Trend, Recommendation};
pub use analysis::fundamental::{ValuationMetrics, ProfitabilityMetrics, FinancialHealthMetrics, GrowthMetrics, DividendMetrics, FundamentalAnalysis, CompanyRating, FundamentalAnalyzer};
