use super::news::Sentiment;
use anyhow::Result;
use mcp_client::llm_provider::LLMProvider;
use mcp_client::ollama::OllamaProvider;
use std::sync::Arc;

/// Service for LLM news analysis
pub struct NewsLLMService {
    llm_provider: Arc<OllamaProvider>,
}

impl NewsLLMService {
    pub fn new(llm_provider: OllamaProvider) -> Self {
        NewsLLMService {
            llm_provider: Arc::new(llm_provider),
        }
    }

    /// Analyze news sentiment
    pub async fn analyze_sentiment(
        &self,
        title: &str,
        content: &str,
    ) -> Result<NewsSentimentResult> {
        let prompt = self.build_sentiment_prompt(title, content);

        let response = self.llm_provider.send_message(prompt).await?;
        let result = self.parse_sentiment_response(&response.message.content)?;

        Ok(result)
    }

    /// Analyze a batch of news for an instrument
    pub async fn analyze_news_batch(
        &self,
        ticker: &str,
        company_name: &str,
        news_items: &[NewsItem],
    ) -> Result<BatchSentimentResult> {
        let prompt = self.build_batch_prompt(ticker, company_name, news_items);

        let response = self.llm_provider.send_message(prompt).await?;
        let result = self.parse_batch_response(&response.message.content)?;

        Ok(result)
    }

    /// Build prompt for analyzing a single news item
    fn build_sentiment_prompt(&self, title: &str, content: &str) -> String {
        format!(
            r#"You are a financial analyst. Analyze the news and determine its sentiment for investors.

Title: {}
Content: {}

Rate the sentiment on a scale from -1.0 to 1.0:
- -1.0 = extremely negative for the company's stock
- 0.0 = neutral
- 1.0 = extremely positive for the company's stock

Also extract key events from the news.

Respond in JSON format:
{{
    "sentiment_score": number from -1.0 to 1.0,
    "sentiment": "Positive" | "Negative" | "Neutral",
    "key_events": ["event 1", "event 2"],
    "confidence": number from 0.0 to 1.0,
    "explanation": "brief explanation"
}}"#,
            title, content
        )
    }

    /// Build prompt for analyzing a batch of news items
    fn build_batch_prompt(
        &self,
        ticker: &str,
        company_name: &str,
        news_items: &[NewsItem],
    ) -> String {
        let mut news_text = String::new();
        for (i, item) in news_items.iter().enumerate() {
            news_text.push_str(&format!("{}. [{}] {}\n", i + 1, item.source, item.title));
        }

        format!(
            r#"You are a professional trading analyst. Analyze the news background for company {} (ticker: {}).

Latest news:
{}

Tasks:
1. Determine the overall news sentiment for the company's stock
2. Identify key events that may affect the stock price
3. Assess the degree of news impact on investment attractiveness

Respond in JSON format:
{{
    "overall_sentiment": "Positive" | "Negative" | "Neutral",
    "sentiment_score": number from -1.0 to 1.0,
    "key_events": ["event 1", "event 2"],
    "risks": ["risk 1", "risk 2"],
    "opportunities": ["opportunity 1", "opportunity 2"],
    "summary": "brief summary of the news background",
    "confidence": number from 0.0 to 1.0
}}"#,
            company_name, ticker, news_text
        )
    }

    /// Parse response for a single news item
    fn parse_sentiment_response(&self, content: &str) -> Result<NewsSentimentResult> {
        // Find JSON in response
        let json_start = content.find('{').unwrap_or(0);
        let json_end = content.rfind('}').unwrap_or(content.len());
        let json_str = &content[json_start..=json_end];

        let parsed: serde_json::Value =
            serde_json::from_str(json_str).unwrap_or_else(|_| serde_json::json!({}));

        let sentiment_str = parsed["sentiment"].as_str().unwrap_or("Neutral");
        let sentiment = match sentiment_str {
            "Positive" => Sentiment::Positive,
            "Negative" => Sentiment::Negative,
            _ => Sentiment::Neutral,
        };

        let score = parsed["sentiment_score"].as_f64().unwrap_or(0.0);
        let confidence = parsed["confidence"].as_f64().unwrap_or(0.5);
        let explanation = parsed["explanation"].as_str().unwrap_or("").to_string();

        let key_events = parsed["key_events"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();

        Ok(NewsSentimentResult {
            sentiment,
            sentiment_score: score,
            confidence,
            key_events,
            explanation,
        })
    }

    /// Parse response for a batch of news items
    fn parse_batch_response(&self, content: &str) -> Result<BatchSentimentResult> {
        // Find JSON in response
        let json_start = content.find('{').unwrap_or(0);
        let json_end = content.rfind('}').unwrap_or(content.len());
        let json_str = &content[json_start..=json_end];

        let parsed: serde_json::Value =
            serde_json::from_str(json_str).unwrap_or_else(|_| serde_json::json!({}));

        let sentiment_str = parsed["overall_sentiment"].as_str().unwrap_or("Neutral");
        let overall_sentiment = match sentiment_str {
            "Positive" => Sentiment::Positive,
            "Negative" => Sentiment::Negative,
            _ => Sentiment::Neutral,
        };

        let score = parsed["sentiment_score"].as_f64().unwrap_or(0.0);
        let confidence = parsed["confidence"].as_f64().unwrap_or(0.5);
        let summary = parsed["summary"].as_str().unwrap_or("").to_string();

        let key_events = parsed["key_events"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();

        let risks = parsed["risks"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();

        let opportunities = parsed["opportunities"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();

        Ok(BatchSentimentResult {
            overall_sentiment,
            sentiment_score: score,
            confidence,
            key_events,
            risks,
            opportunities,
            summary,
        })
    }
}

/// Result of analyzing a single news item
#[derive(Debug, Clone)]
pub struct NewsSentimentResult {
    pub sentiment: Sentiment,
    pub sentiment_score: f64,
    pub confidence: f64,
    pub key_events: Vec<String>,
    pub explanation: String,
}

/// Result of analyzing a batch of news items
#[derive(Debug, Clone)]
pub struct BatchSentimentResult {
    pub overall_sentiment: Sentiment,
    pub sentiment_score: f64,
    pub confidence: f64,
    pub key_events: Vec<String>,
    pub risks: Vec<String>,
    pub opportunities: Vec<String>,
    pub summary: String,
}

/// News item for analysis
#[derive(Debug, Clone)]
pub struct NewsItem {
    pub title: String,
    pub content: String,
    pub source: String,
    pub url: String,
}
