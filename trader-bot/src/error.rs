//! Типы ошибок для торгового бота

use thiserror::Error;

/// Ошибки торгового бота
#[derive(Error, Debug)]
pub enum BotError {
    /// Ошибка при работе с позициями
    #[error("Ошибка позиции: {0}")]
    Position(String),

    /// Ошибка при исполнении ордера
    #[error("Ошибка исполнения ордера: {0}")]
    OrderExecution(String),

    /// Ошибка стратегии
    #[error("Ошибка стратегии: {0}")]
    Strategy(String),

    /// Ошибка анализа рынка
    #[error("Ошибка анализа рынка: {0}")]
    MarketAnalysis(String),

    /// Ошибка LLM
    #[error("Ошибка LLM: {0}")]
    LLM(String),

    /// Ошибка конфигурации
    #[error("Ошибка конфигурации: {0}")]
    Configuration(String),

    /// Ошибка сети
    #[error("Ошибка сети: {0}")]
    Network(#[from] reqwest::Error),

    /// Ошибка SDK
    #[error("Ошибка SDK: {0}")]
    SDK(String),

    /// Grid не инициализирован
    #[error("Grid стратегия не инициализирована")]
    GridNotInitialized,

    /// Недостаточно средств
    #[error("Недостаточно средств: требуется {required}, доступно {available}")]
    InsufficientFunds { required: f64, available: f64 },

    /// Некорректное количество
    #[error("Некорректное количество: {0}")]
    InvalidQuantity(String),

    /// Некорректная цена
    #[error("Некорректная цена: {0}")]
    InvalidPrice(String),
}

/// Ошибки исполнения ордера
#[derive(Error, Debug)]
pub enum OrderError {
    /// Ордер отклонён
    #[error("Ордер отклонён: {0}")]
    Rejected(String),

    /// Ордер не найден
    #[error("Ордер не найден: {order_id}")]
    NotFound { order_id: String },

    /// Недостаточно ликвидности
    #[error("Недостаточно ликвидности для исполнения ордера")]
    InsufficientLiquidity,

    /// Превышен лимит цены
    #[error("Цена вышла за допустимые пределы: {price}")]
    PriceLimitExceeded { price: f64 },

    /// Ошибка валидации ордера
    #[error("Валидация ордера не пройдена: {0}")]
    ValidationError(String),
}

/// Ошибки стратегии
#[derive(Error, Debug)]
pub enum StrategyError {
    /// Стратегия не инициализирована
    #[error("Стратегия не инициализирована")]
    NotInitialized,

    /// Некорректная конфигурация
    #[error("Некорректная конфигурация: {0}")]
    InvalidConfig(String),

    /// Ошибка расчёта
    #[error("Ошибка расчёта: {0}")]
    Calculation(String),
}

/// Ошибки анализа новостей
#[derive(Error, Debug)]
pub enum NewsAnalysisError {
    /// Не удалось получить новости
    #[error("Не удалось получить новости: {0}")]
    FetchError(String),

    /// Ошибка парсинга
    #[error("Ошибка парсинга новостей: {0}")]
    ParseError(String),

    /// Ошибка LLM анализа
    #[error("Ошибка LLM анализа: {0}")]
    LLMError(String),

    /// Нет данных для анализа
    #[error("Нет данных для анализа")]
    NoData,
}

/// Результат с ошибкой бота
pub type BotResult<T> = Result<T, BotError>;

/// Результат с ошибкой ордера
pub type OrderResult<T> = Result<T, OrderError>;

/// Результат с ошибкой стратегии
pub type StrategyResult<T> = Result<T, StrategyError>;

/// Результат с ошибкой анализа новостей
pub type NewsAnalysisResult<T> = Result<T, NewsAnalysisError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bot_error_display() {
        let err = BotError::Position("test error".to_string());
        assert!(err.to_string().contains("Ошибка позиции"));
        assert!(err.to_string().contains("test error"));

        let err = BotError::OrderExecution("order failed".to_string());
        assert!(err.to_string().contains("Ошибка исполнения ордера"));

        let err = BotError::Strategy("invalid config".to_string());
        assert!(err.to_string().contains("Ошибка стратегии"));

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
        assert!(err.to_string().contains("инициализирована"));

        let err = StrategyError::InvalidConfig("missing field".to_string());
        assert!(err.to_string().contains("missing field"));
    }

    #[test]
    fn test_news_analysis_error_display() {
        let err = NewsAnalysisError::FetchError("network timeout".to_string());
        assert!(err.to_string().contains("Не удалось получить новости"));

        let err = NewsAnalysisError::NoData;
        assert!(err.to_string().contains("Нет данных"));
    }
}
