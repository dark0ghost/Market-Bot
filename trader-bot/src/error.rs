//! Error types for the trading bot

use thiserror::Error;

/// Trading bot errors
#[derive(Error, Debug)]
pub enum BotError {
    /// Position error
    #[error("Position error: {0}")]
    Position(String),

    /// Order execution error
    #[error("Order execution error: {0}")]
    OrderExecution(String),

    /// Strategy error
    #[error("Strategy error: {0}")]
    Strategy(String),

    /// Market analysis error
    #[error("Market analysis error: {0}")]
    MarketAnalysis(String),

    /// Llm error
    #[error("LLM error: {0}")]
    Llm(String),

    /// Configuration error
    #[error("Configuration error: {0}")]
    Configuration(String),

    /// Network error
    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),

    /// SDK error
    #[error("SDK error: {0}")]
    SDK(String),

    /// Grid not initialized
    #[error("Grid strategy not initialized")]
    GridNotInitialized,

    /// Insufficient funds
    #[error("Insufficient funds: required {required}, available {available}")]
    InsufficientFunds { required: f64, available: f64 },

    /// Invalid quantity
    #[error("Invalid quantity: {0}")]
    InvalidQuantity(String),

    /// Invalid price
    #[error("Invalid price: {0}")]
    InvalidPrice(String),
}

/// Order execution errors
#[derive(Error, Debug)]
pub enum OrderError {
    /// Order rejected
    #[error("Order rejected: {0}")]
    Rejected(String),

    /// Order not found
    #[error("Order not found: {order_id}")]
    NotFound { order_id: String },

    /// Insufficient liquidity
    #[error("Insufficient liquidity to execute order")]
    InsufficientLiquidity,

    /// Price limit exceeded
    #[error("Price exceeded allowed limits: {price}")]
    PriceLimitExceeded { price: f64 },

    /// Order validation error
    #[error("Order validation failed: {0}")]
    ValidationError(String),
}

/// Strategy errors
#[derive(Error, Debug)]
pub enum StrategyError {
    /// Strategy not initialized
    #[error("Strategy not initialized")]
    NotInitialized,

    /// Invalid configuration
    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),

    /// Calculation error
    #[error("Calculation error: {0}")]
    Calculation(String),
}

/// News analysis errors
#[derive(Error, Debug)]
pub enum NewsAnalysisError {
    /// Failed to fetch news
    #[error("Failed to fetch news: {0}")]
    FetchError(String),

    /// Parse error
    #[error("News parse error: {0}")]
    ParseError(String),

    /// LLM analysis error
    #[error("LLM analysis error: {0}")]
    LLMError(String),

    /// No data for analysis
    #[error("No data for analysis")]
    NoData,
}

/// Result with bot error
pub type BotResult<T> = Result<T, BotError>;

/// Result with order error
pub type OrderResult<T> = Result<T, OrderError>;

/// Result with strategy error
pub type StrategyResult<T> = Result<T, StrategyError>;

/// Result with news analysis error
pub type NewsAnalysisResult<T> = Result<T, NewsAnalysisError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bot_error_display() {
        let err = BotError::Position("test error".to_string());
        assert!(err.to_string().contains("Position error"));
        assert!(err.to_string().contains("test error"));

        let err = BotError::OrderExecution("order failed".to_string());
        assert!(err.to_string().contains("Order execution error"));

        let err = BotError::Strategy("invalid config".to_string());
        assert!(err.to_string().contains("Strategy error"));

        let err = BotError::InsufficientFunds {
            required: 100.0,
            available: 50.0,
        };
        assert!(err.to_string().contains("100"));
        assert!(err.to_string().contains("50"));
    }

    #[test]
    fn test_order_error_display() {
        let err = OrderError::Rejected("margin call".to_string());
        assert!(err.to_string().contains("margin call"));

        let err = OrderError::NotFound {
            order_id: "123".to_string(),
        };
        assert!(err.to_string().contains("123"));

        let err = OrderError::PriceLimitExceeded { price: 150.0 };
        assert!(err.to_string().contains("150"));
    }

    #[test]
    fn test_strategy_error_display() {
        let err = StrategyError::NotInitialized;
        assert!(err.to_string().contains("not initialized"));

        let err = StrategyError::InvalidConfig("missing field".to_string());
        assert!(err.to_string().contains("missing field"));
    }

    #[test]
    fn test_news_analysis_error_display() {
        let err = NewsAnalysisError::FetchError("network timeout".to_string());
        assert!(err.to_string().contains("Failed to fetch news"));

        let err = NewsAnalysisError::NoData;
        assert!(err.to_string().contains("No data"));
    }
}
