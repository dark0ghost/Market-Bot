pub mod news;
pub mod technical;
pub mod fundamental;
pub mod fundamental_data;
pub mod news_llm;

pub use news::{NewsAnalyzer, NewsSentiment, Sentiment, NewsArticle};
pub use technical::{
    TechnicalAnalyzer, TechnicalAnalysis, Trend, Recommendation, VolumeAnalysis,
};
pub use fundamental::{
    FundamentalAnalyzer, FundamentalAnalysis, CompanyRating,
};
pub use fundamental_data::FundamentalDataService;
pub use news_llm::{NewsLLMService, NewsSentimentResult, BatchSentimentResult, NewsItem};

// Re-export Sentiment from news_llm for backward compatibility
pub use news::Sentiment as LlmSentiment;
