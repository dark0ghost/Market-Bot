pub mod detectors;
pub mod finbert;
pub mod fundamental;
pub mod fundamental_data;
pub mod news;
pub mod news_llm;
pub mod regime;
pub mod technical;

pub use finbert::FinBertSentimentService;
pub use fundamental::{
    CompanyRating, DividendMetrics, FinancialHealthMetrics, FundamentalAnalysis, GrowthMetrics,
    ProfitabilityMetrics, ValuationMetrics,
};
pub use fundamental_data::FundamentalDataService;
pub use news::{NewsAnalyzer, NewsArticle, NewsSentiment, Sentiment};
pub use news_llm::{NewsItem, NewsLlmService, NewsSentimentAnalyzer};
pub use regime::{MarketRegime, RegimeDetector};
pub use technical::{
    BollingerValues, MacdValues, Recommendation, TechnicalAnalysis, TechnicalAnalyzer, Trend,
    VolumeAnalysis,
};

// Re-export for backward compatibility
