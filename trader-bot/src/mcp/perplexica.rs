use anyhow::{Context, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;

const DEFAULT_PERPLEXICA_URL: &str = "http://localhost:3000";

/// Provider for working with the Perplexica API
pub struct PerplexicaProvider {
    client: Client,
    base_url: String,
    chat_model: ModelConfig,
    embedding_model: ModelConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelConfig {
    pub provider_id: String,
    pub key: String,
}

/// Search optimization mode
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
pub enum OptimizationMode {
    /// Fastest results, lower quality
    Speed,
    /// Balance between speed and quality
    Balanced,
    /// Best quality, slower
    #[default]
    Quality,
}

/// Search source
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SearchSource {
    /// General web search
    Web,
    /// Academic sources (arxiv, scholar, pubmed)
    Academic,
    /// Forums and discussions (reddit, etc.)
    Discussions,
}

/// Focus mode (for /api/chat)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum FocusMode {
    WebSearch,
    AcademicSearch,
    YoutubeSearch,
    RedditSearch,
    /// Writing assistant (no search)
    WritingAssistant,
}

/// Request to the Perplexica API
#[derive(Debug, Clone, Serialize)]
pub struct SearchRequest {
    pub chat_model: ModelConfig,
    pub embedding_model: ModelConfig,
    pub sources: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub optimization_mode: Option<String>,
    pub query: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub history: Option<Vec<(String, String)>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_instructions: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
}

/// Source metadata
#[derive(Debug, Clone, Deserialize)]
pub struct SourceMetadata {
    pub title: Option<String>,
    pub url: Option<String>,
}

/// Source in the response
#[derive(Debug, Clone, Deserialize)]
pub struct Source {
    pub content: Option<String>,
    pub metadata: Option<SourceMetadata>,
}

/// Response from the Perplexica API
#[derive(Debug, Clone, Deserialize)]
pub struct SearchResponse {
    pub message: Option<String>,
    #[serde(default)]
    pub sources: Vec<Source>,
}

/// Search result for LLM consumption
#[derive(Debug, Clone)]
pub struct SearchResult {
    pub answer: String,
    pub sources: Vec<SourceInfo>,
}

/// Source information
#[derive(Debug, Clone)]
pub struct SourceInfo {
    pub title: String,
    pub url: String,
    pub snippet: String,
}

impl PerplexicaProvider {
    /// Create a new PerplexicaProvider with the default URL
    pub fn new(chat_model: ModelConfig, embedding_model: ModelConfig) -> Self {
        Self::with_url(DEFAULT_PERPLEXICA_URL, chat_model, embedding_model)
    }

    /// Create a PerplexicaProvider with a custom URL
    pub fn with_url(base_url: &str, chat_model: ModelConfig, embedding_model: ModelConfig) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .user_agent("AI-Trading-Bot/1.0")
            .build()
            .unwrap_or_default();

        PerplexicaProvider {
            client,
            base_url: base_url.trim_end_matches('/').to_string(),
            chat_model,
            embedding_model,
        }
    }

    /// Run a search query
    pub async fn search(&self, query: &str) -> Result<SearchResult> {
        self.search_with_options(query, None, None, None, Some(OptimizationMode::Balanced))
            .await
    }

    /// Run a search query with optional parameters
    pub async fn search_with_options(
        &self,
        query: &str,
        sources: Option<Vec<SearchSource>>,
        system_instructions: Option<&str>,
        history: Option<Vec<(String, String)>>,
        optimization_mode: Option<OptimizationMode>,
    ) -> Result<SearchResult> {
        let sources_list: Vec<String> = sources
            .unwrap_or(vec![SearchSource::Web])
            .iter()
            .map(|s| match s {
                SearchSource::Web => "web".to_string(),
                SearchSource::Academic => "academic".to_string(),
                SearchSource::Discussions => "discussions".to_string(),
            })
            .collect();

        let request = SearchRequest {
            chat_model: self.chat_model.clone(),
            embedding_model: self.embedding_model.clone(),
            optimization_mode: optimization_mode.map(|m| match m {
                OptimizationMode::Speed => "speed".to_string(),
                OptimizationMode::Balanced => "balanced".to_string(),
                OptimizationMode::Quality => "quality".to_string(),
            }),
            sources: sources_list,
            query: query.to_string(),
            history,
            system_instructions: system_instructions.map(String::from),
            stream: Some(false),
        };

        let url = format!("{}/api/search", self.base_url);

        let response = self
            .client
            .post(&url)
            .json(&request)
            .send()
            .await
            .context("Failed to send request to Perplexica API")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Perplexica API error ({}): {}", status, error_text);
        }

        let api_response: SearchResponse = response
            .json()
            .await
            .context("Failed to parse Perplexica API response")?;

        let answer = api_response.message.unwrap_or_default();
        let sources_info: Vec<SourceInfo> = api_response
            .sources
            .into_iter()
            .filter_map(|source| {
                let metadata = source.metadata?;
                let title = metadata.title.unwrap_or_else(|| "Unknown".to_string());
                let url = metadata.url.unwrap_or_else(|| "#".to_string());
                let snippet = source.content.unwrap_or_default();

                Some(SourceInfo {
                    title,
                    url,
                    snippet,
                })
            })
            .collect();

        Ok(SearchResult {
            answer,
            sources: sources_info,
        })
    }

    /// Search for company information
    pub async fn search_company(&self, ticker: &str, company_name: &str) -> Result<SearchResult> {
        let query = format!(
            "{} ({}) stocks news financial analysis",
            company_name, ticker
        );

        self.search_with_options(
            &query,
            Some(vec![SearchSource::Web, SearchSource::Academic]),
            Some("Provide detailed company information: financial metrics, latest news, analyst reports, forecasts."),
            None,
            Some(OptimizationMode::Quality),
        )
        .await
    }

    /// Search for news by ticker
    pub async fn search_news(&self, ticker: &str) -> Result<SearchResult> {
        let query = format!("{} stock latest news", ticker);

        self.search_with_options(
            &query,
            Some(vec![SearchSource::Web]),
            Some("Find the latest news about the company. Include publication date and source."),
            None,
            Some(OptimizationMode::Speed),
        )
        .await
    }

    /// Search for analyst ratings
    pub async fn search_analyst_ratings(
        &self,
        ticker: &str,
        company_name: &str,
    ) -> Result<SearchResult> {
        let query = format!(
            "{} {} analyst rating target price forecast",
            ticker, company_name
        );

        self.search_with_options(
            &query,
            Some(vec![SearchSource::Web, SearchSource::Academic]),
            Some("Find analyst reports, ratings, and target prices from investment banks and research agencies."),
            None,
            Some(OptimizationMode::Quality),
        )
        .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_model_config_serialization() {
        let config = ModelConfig {
            provider_id: "test-provider".to_string(),
            key: "gpt-4o-mini".to_string(),
        };

        let json = serde_json::to_string(&config).unwrap();
        assert!(json.contains("test-provider"));
        assert!(json.contains("gpt-4o-mini"));
    }

    #[test]
    fn test_search_request_serialization() {
        let request = SearchRequest {
            chat_model: ModelConfig {
                provider_id: "provider-1".to_string(),
                key: "gpt-4o".to_string(),
            },
            embedding_model: ModelConfig {
                provider_id: "provider-1".to_string(),
                key: "text-embedding-3-large".to_string(),
            },
            optimization_mode: Some("balanced".to_string()),
            sources: vec!["web".to_string()],
            query: "Test query".to_string(),
            history: None,
            system_instructions: None,
            stream: Some(false),
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("Test query"));
        assert!(json.contains("balanced"));
    }

    #[test]
    fn test_optimization_mode_default() {
        assert!(matches!(
            OptimizationMode::default(),
            OptimizationMode::Quality
        ));
    }

    #[test]
    fn test_optimization_mode_serialization() {
        assert_eq!(
            serde_json::to_string(&OptimizationMode::Speed).unwrap(),
            "\"speed\""
        );
        assert_eq!(
            serde_json::to_string(&OptimizationMode::Balanced).unwrap(),
            "\"balanced\""
        );
        assert_eq!(
            serde_json::to_string(&OptimizationMode::Quality).unwrap(),
            "\"quality\""
        );
    }

    #[test]
    fn test_focus_mode_serialization() {
        assert_eq!(
            serde_json::to_string(&FocusMode::WebSearch).unwrap(),
            "\"webSearch\""
        );
        assert_eq!(
            serde_json::to_string(&FocusMode::AcademicSearch).unwrap(),
            "\"academicSearch\""
        );
        assert_eq!(
            serde_json::to_string(&FocusMode::RedditSearch).unwrap(),
            "\"redditSearch\""
        );
    }

    #[test]
    fn test_search_source_serialization() {
        let web = serde_json::to_string(&SearchSource::Web).unwrap();
        assert_eq!(web, "\"web\"");

        let academic = serde_json::to_string(&SearchSource::Academic).unwrap();
        assert_eq!(academic, "\"academic\"");

        let discussions = serde_json::to_string(&SearchSource::Discussions).unwrap();
        assert_eq!(discussions, "\"discussions\"");
    }

    #[test]
    fn test_perplexica_searcher_creation() {
        let chat_model = ModelConfig {
            provider_id: "test".to_string(),
            key: "gpt-4o".to_string(),
        };
        let embedding_model = ModelConfig {
            provider_id: "test".to_string(),
            key: "text-embedding".to_string(),
        };

        let searcher = PerplexicaSearcher::new(chat_model.clone(), embedding_model.clone());
        assert_eq!(searcher.provider.chat_model.provider_id, "test");

        let searcher_custom =
            PerplexicaSearcher::with_url("http://custom:3000", chat_model, embedding_model);
        assert_eq!(searcher_custom.provider.base_url, "http://custom:3000");
    }
}

/// Search tool using Perplexica
/// Used with LLM for automatic information retrieval
pub struct PerplexicaSearcher {
    provider: PerplexicaProvider,
}

impl PerplexicaSearcher {
    /// Create a new search tool
    pub fn new(chat_model: ModelConfig, embedding_model: ModelConfig) -> Self {
        PerplexicaSearcher {
            provider: PerplexicaProvider::new(chat_model, embedding_model),
        }
    }

    /// Create with a custom URL
    pub fn with_url(base_url: &str, chat_model: ModelConfig, embedding_model: ModelConfig) -> Self {
        PerplexicaSearcher {
            provider: PerplexicaProvider::with_url(base_url, chat_model, embedding_model),
        }
    }

    /// Search for company information by ticker and name
    pub async fn search_company_info(
        &self,
        ticker: String,
        company_name: String,
    ) -> Result<String> {
        let result = self.provider.search_company(&ticker, &company_name).await?;

        let mut output = format!("=== Company Info: {} ({}) ===\n\n", company_name, ticker);
        output.push_str(&result.answer);
        output.push_str("\n\n=== Sources ===\n");

        for (i, source) in result.sources.iter().enumerate() {
            output.push_str(&format!("{}. {} - {}\n", i + 1, source.title, source.url));
        }

        Ok(output)
    }

    /// Search for latest news
    pub async fn search_latest_news(&self, ticker: String) -> Result<String> {
        let result = self.provider.search_news(&ticker).await?;

        let mut output = format!("=== Latest News: {} ===\n\n", ticker);
        output.push_str(&result.answer);
        output.push_str("\n\n=== Sources ===\n");

        for (i, source) in result.sources.iter().enumerate() {
            output.push_str(&format!("{}. {} - {}\n", i + 1, source.title, source.url));
        }

        Ok(output)
    }

    /// Search for analyst ratings
    pub async fn search_analyst_ratings(
        &self,
        ticker: String,
        company_name: String,
    ) -> Result<String> {
        let result = self
            .provider
            .search_analyst_ratings(&ticker, &company_name)
            .await?;

        let mut output = format!("=== Analyst Ratings: {} ({}) ===\n\n", company_name, ticker);
        output.push_str(&result.answer);
        output.push_str("\n\n=== Sources ===\n");

        for (i, source) in result.sources.iter().enumerate() {
            output.push_str(&format!("{}. {} - {}\n", i + 1, source.title, source.url));
        }

        Ok(output)
    }

    /// General search query
    pub async fn search(&self, query: String) -> Result<String> {
        let result = self.provider.search(&query).await?;

        let mut output = format!(
            "=== Search Results ===\n\n{}\n\n=== Sources ===\n",
            result.answer
        );

        for (i, source) in result.sources.iter().enumerate() {
            output.push_str(&format!("{}. {} - {}\n", i + 1, source.title, source.url));
        }

        Ok(output)
    }
}
