use anyhow::Result;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// News search result
#[derive(Debug, Clone)]
pub struct NewsArticle {
    pub title: String,
    pub content: String,
    pub source: String,
    pub url: String,
    pub published_at: Option<String>,
    pub sentiment: Option<Sentiment>,
}

/// News sentiment
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Sentiment {
    Positive,
    Negative,
    Neutral,
}

impl Sentiment {
    /// Convert to numeric value
    pub const fn to_score(&self) -> f64 {
        match self {
            Sentiment::Positive => 1.0,
            Sentiment::Negative => -1.0,
            Sentiment::Neutral => 0.0,
        }
    }

    /// Convert from numeric value
    pub const fn from_score(score: f64) -> Self {
        if score > 0.2 {
            Sentiment::Positive
        } else if score < -0.2 {
            Sentiment::Negative
        } else {
            Sentiment::Neutral
        }
    }
}

impl std::fmt::Display for Sentiment {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Sentiment::Positive => write!(f, "Positive"),
            Sentiment::Negative => write!(f, "Negative"),
            Sentiment::Neutral => write!(f, "Neutral"),
        }
    }
}

/// Aggregated news sentiment by instrument
#[derive(Debug, Clone)]
pub struct NewsSentiment {
    pub ticker: String,
    pub overall_sentiment: Sentiment,
    pub sentiment_score: f64, // from -1.0 to 1.0
    pub articles_count: usize,
    pub articles: Vec<NewsArticle>,
    pub key_events: Vec<String>,
}

/// News analysis service
pub struct NewsAnalyzer {
    client: Client,
    sources: Vec<String>,
}

impl NewsAnalyzer {
    pub fn new(sources: Vec<String>) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(10))
            .user_agent("AI-Trading-Bot/1.0")
            .build()
            .unwrap_or_default();

        NewsAnalyzer { client, sources }
    }

    /// Analyze news by instrument
    pub async fn analyze(&self, ticker: &str, company_name: &str) -> Result<NewsSentiment> {
        let mut articles = Vec::new();

        // Search news by sources
        for source in &self.sources {
            match source.as_str() {
                "tinkoff" => {
                    let tinkoff_news = self.search_tinkoff_news(ticker).await?;
                    articles.extend(tinkoff_news);
                }
                "investing" => {
                    let investing_news = self.search_investing_news(ticker).await?;
                    articles.extend(investing_news);
                }
                "bloomberg" => {
                    let bloomberg_news = self.search_bloomberg_news(company_name).await?;
                    articles.extend(bloomberg_news);
                }
                _ => {
                    log::warn!("Unknown news source: {}", source);
                }
            }
        }

        // Sentiment analysis
        let sentiment_score = self.calculate_sentiment_score(&articles).await?;
        let overall_sentiment = self.score_to_sentiment(sentiment_score);
        let key_events = self.extract_key_events(&articles).await?;

        Ok(NewsSentiment {
            ticker: ticker.to_string(),
            overall_sentiment,
            sentiment_score,
            articles_count: articles.len(),
            articles,
            key_events,
        })
    }

    /// Search news in Tinkoff source
    async fn search_tinkoff_news(&self, ticker: &str) -> Result<Vec<NewsArticle>> {
        // Use DuckDuckGo to search Tinkoff Invest news
        let query = format!("{} stock news site:tinkoff.ru", ticker);
        let articles = self.duckduckgo_search(&query).await?;
        Ok(articles)
    }

    /// Search news on Investing.com
    async fn search_investing_news(&self, ticker: &str) -> Result<Vec<NewsArticle>> {
        let query = format!("{} stock news site:investing.com", ticker);
        let articles = self.duckduckgo_search(&query).await?;
        Ok(articles)
    }

    /// Search news on Bloomberg
    async fn search_bloomberg_news(&self, company_name: &str) -> Result<Vec<NewsArticle>> {
        let query = format!("{} stock news site:bloomberg.com", company_name);
        let articles = self.duckduckgo_search(&query).await?;
        Ok(articles)
    }

    /// Search via DuckDuckGo
    async fn duckduckgo_search(&self, query: &str) -> Result<Vec<NewsArticle>> {
        // Note: This is a simplified implementation
        // In production, use a full API or parsing
        let url = format!("https://html.duckduckgo.com/html/?q={}", query);

        match self.client.get(&url).send().await {
            Ok(response) => {
                let text = response.text().await?;
                // Simplified parsing - in reality, use an HTML parser
                Ok(vec![]) // Placeholder - real implementation via task agent
            }
            Err(_) => Ok(vec![]),
        }
    }

    /// Calculate overall sentiment score
    async fn calculate_sentiment_score(&self, articles: &[NewsArticle]) -> Result<f64> {
        if articles.is_empty() {
            return Ok(0.0);
        }

        let total_score: f64 = articles
            .iter()
            .map(|article| article.sentiment.as_ref().map_or(0.0, Sentiment::to_score))
            .sum();

        Ok(total_score / articles.len() as f64)
    }

    /// Convert score to Sentiment
    fn score_to_sentiment(&self, score: f64) -> Sentiment {
        Sentiment::from_score(score)
    }

    /// Extract key events from news
    async fn extract_key_events(&self, articles: &[NewsArticle]) -> Result<Vec<String>> {
        // LLM analysis will be here for extracting key events
        // For example: "reconversion", "dividends", "financial reports"
        let mut events = Vec::new();

        for article in articles {
            let title_lower = article.title.to_lowercase();

            if title_lower.contains("reconversion") || title_lower.contains("conversion") {
                events.push("Stock reconversion".to_string());
            }
            if title_lower.contains("dividend") {
                events.push("Dividend payments".to_string());
            }
            if title_lower.contains("report") || title_lower.contains("financial result") {
                events.push("Financial reporting".to_string());
            }
            if title_lower.contains("sanction") || title_lower.contains("restrict") {
                events.push("Sanctions pressure".to_string());
            }
        }

        events.sort();
        events.dedup();
        Ok(events)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_sentiment_conversion() {
        let analyzer = NewsAnalyzer::new(vec![]);

        assert_eq!(analyzer.score_to_sentiment(0.5), Sentiment::Positive);
        assert_eq!(analyzer.score_to_sentiment(-0.5), Sentiment::Negative);
        assert_eq!(analyzer.score_to_sentiment(0.0), Sentiment::Neutral);
    }

    #[test]
    fn test_sentiment_to_score() {
        assert_eq!(Sentiment::Positive.to_score(), 1.0);
        assert_eq!(Sentiment::Negative.to_score(), -1.0);
        assert_eq!(Sentiment::Neutral.to_score(), 0.0);
    }

    #[test]
    fn test_sentiment_from_score() {
        assert_eq!(Sentiment::from_score(0.5), Sentiment::Positive);
        assert_eq!(Sentiment::from_score(-0.5), Sentiment::Negative);
        assert_eq!(Sentiment::from_score(0.0), Sentiment::Neutral);
        assert_eq!(Sentiment::from_score(0.2), Sentiment::Neutral);
        assert_eq!(Sentiment::from_score(-0.2), Sentiment::Neutral);
        assert_eq!(Sentiment::from_score(0.21), Sentiment::Positive);
        assert_eq!(Sentiment::from_score(-0.21), Sentiment::Negative);
    }

    #[test]
    fn test_sentiment_roundtrip() {
        // Check that from_score(to_score(x)) == x
        assert_eq!(
            Sentiment::from_score(Sentiment::Positive.to_score()),
            Sentiment::Positive
        );
        assert_eq!(
            Sentiment::from_score(Sentiment::Negative.to_score()),
            Sentiment::Negative
        );
        assert_eq!(
            Sentiment::from_score(Sentiment::Neutral.to_score()),
            Sentiment::Neutral
        );
    }

    #[test]
    fn test_sentiment_display() {
        assert_eq!(format!("{}", Sentiment::Positive), "Positive");
        assert_eq!(format!("{}", Sentiment::Negative), "Negative");
        assert_eq!(format!("{}", Sentiment::Neutral), "Neutral");
    }
}
