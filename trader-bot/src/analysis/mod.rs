pub mod fundamental;
pub mod fundamental_data;
pub mod news;
pub mod news_llm;
pub mod regime;
pub mod technical;

pub use fundamental::{CompanyRating, FundamentalAnalysis, FundamentalAnalyzer};
pub use fundamental_data::FundamentalDataService;
pub use news::{NewsAnalyzer, NewsArticle, NewsSentiment, Sentiment};
pub use news_llm::{BatchSentimentResult, NewsItem, NewsLLMService, NewsSentimentResult};
pub use regime::{MarketRegime, RegimeDetector};
pub use technical::{Recommendation, TechnicalAnalysis, TechnicalAnalyzer, Trend, VolumeAnalysis};

// Re-export Sentiment from news_llm for backward compatibility
pub use news::Sentiment as LlmSentiment;
