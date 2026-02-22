use anyhow::{Context, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Базовый URL Perplexica по умолчанию
const DEFAULT_PERPLEXICA_URL: &str = "http://localhost:3000";

/// Провайдер для работы с Perplexica API
pub struct PerplexicaProvider {
    client: Client,
    base_url: String,
    chat_model: ModelConfig,
    embedding_model: ModelConfig,
}

/// Конфигурация модели
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelConfig {
    pub provider_id: String,
    pub key: String,
}

/// Режим оптимизации поиска
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OptimizationMode {
    Speed,
    Balanced,
    Quality,
}

impl Default for OptimizationMode {
    fn default() -> Self {
        OptimizationMode::Balanced
    }
}

/// Источник поиска
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SearchSource {
    Web,
    Academic,
    Discussions,
}

/// Запрос к Perplexica API
#[derive(Debug, Clone, Serialize)]
pub struct SearchRequest {
    pub chat_model: ModelConfig,
    pub embedding_model: ModelConfig,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub optimization_mode: Option<String>,
    pub sources: Vec<String>,
    pub query: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub history: Option<Vec<(String, String)>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_instructions: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
}

/// Метаданные источника
#[derive(Debug, Clone, Deserialize)]
pub struct SourceMetadata {
    pub title: Option<String>,
    pub url: Option<String>,
}

/// Источник в ответе
#[derive(Debug, Clone, Deserialize)]
pub struct Source {
    pub content: Option<String>,
    pub metadata: Option<SourceMetadata>,
}

/// Ответ от Perplexica API
#[derive(Debug, Clone, Deserialize)]
pub struct SearchResponse {
    pub message: Option<String>,
    #[serde(default)]
    pub sources: Vec<Source>,
}

/// Результат поиска для LLM
#[derive(Debug, Clone)]
pub struct SearchResult {
    pub answer: String,
    pub sources: Vec<SourceInfo>,
}

/// Информация об источнике
#[derive(Debug, Clone)]
pub struct SourceInfo {
    pub title: String,
    pub url: String,
    pub snippet: String,
}

impl PerplexicaProvider {
    /// Создать новый PerplexicaProvider с URL по умолчанию
    pub fn new(chat_model: ModelConfig, embedding_model: ModelConfig) -> Self {
        Self::with_url(DEFAULT_PERPLEXICA_URL, chat_model, embedding_model)
    }

    /// Создать PerplexicaProvider с кастомным URL
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

    /// Выполнить поисковый запрос
    pub async fn search(&self, query: &str) -> Result<SearchResult> {
        self.search_with_options(
            query,
            None,
            None,
            None,
            Some(OptimizationMode::Balanced),
        )
        .await
    }

    /// Выполнить поисковый запрос с опциями
    pub async fn search_with_options(
        &self,
        query: &str,
        sources: Option<Vec<SearchSource>>,
        system_instructions: Option<&str>,
        history: Option<Vec<(String, String)>>,
        optimization_mode: Option<OptimizationMode>,
    ) -> Result<SearchResult> {
        let sources_list = sources
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
        
        let response = self.client
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

        // Преобразуем ответ в SearchResult
        let answer = api_response.message.unwrap_or_default();
        let sources_info = api_response
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

    /// Поиск информации о компании
    pub async fn search_company(&self, ticker: &str, company_name: &str) -> Result<SearchResult> {
        let query = format!(
            "{} ({}) акции новости финансовое состояние аналитика",
            company_name, ticker
        );
        
        self.search_with_options(
            &query,
            Some(vec![SearchSource::Web, SearchSource::Academic]),
            Some("Предоставь подробную информацию о компании: финансовые показатели, последние новости, аналитические отчеты, прогнозы."),
            None,
            Some(OptimizationMode::Quality),
        )
        .await
    }

    /// Поиск новостей по тикеру
    pub async fn search_news(&self, ticker: &str) -> Result<SearchResult> {
        let query = format!("{} акции последние новости", ticker);
        
        self.search_with_options(
            &query,
            Some(vec![SearchSource::Web]),
            Some("Найди последние новости о компании. Укажи дату публикации и источник."),
            None,
            Some(OptimizationMode::Speed),
        )
        .await
    }

    /// Поиск аналитики по компании
    pub async fn search_analyst_ratings(&self, ticker: &str, company_name: &str) -> Result<SearchResult> {
        let query = format!(
            "{} {} аналитика рейтинг целевая цена прогноз",
            ticker, company_name
        );
        
        self.search_with_options(
            &query,
            Some(vec![SearchSource::Web, SearchSource::Academic]),
            Some("Найди аналитические отчеты, рейтинги и целевые цены от инвестиционных банков и аналитических агентств."),
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
            OptimizationMode::Balanced
        ));
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

        let searcher_custom = PerplexicaSearcher::with_url(
            "http://custom:3000",
            chat_model,
            embedding_model,
        );
        assert_eq!(searcher_custom.provider.base_url, "http://custom:3000");
    }
}

/// Инструмент для поиска через Perplexica
/// Используется с LLM для автоматического поиска информации
pub struct PerplexicaSearcher {
    provider: PerplexicaProvider,
}

impl PerplexicaSearcher {
    /// Создать новый поисковой инструмент
    pub fn new(chat_model: ModelConfig, embedding_model: ModelConfig) -> Self {
        PerplexicaSearcher {
            provider: PerplexicaProvider::new(chat_model, embedding_model),
        }
    }

    /// Создать с кастомным URL
    pub fn with_url(base_url: &str, chat_model: ModelConfig, embedding_model: ModelConfig) -> Self {
        PerplexicaSearcher {
            provider: PerplexicaProvider::with_url(base_url, chat_model, embedding_model),
        }
    }

    /// Поиск информации о компании по тикеру и названию
    pub async fn search_company_info(&self, ticker: String, company_name: String) -> Result<String> {
        let result = self.provider.search_company(&ticker, &company_name).await?;
        
        let mut output = format!("=== Информация о компании {} ({}) ===\n\n", company_name, ticker);
        output.push_str(&result.answer);
        output.push_str("\n\n=== Источники ===\n");
        
        for (i, source) in result.sources.iter().enumerate() {
            output.push_str(&format!("{}. {} - {}\n", i + 1, source.title, source.url));
        }
        
        Ok(output)
    }

    /// Поиск последних новостей
    pub async fn search_latest_news(&self, ticker: String) -> Result<String> {
        let result = self.provider.search_news(&ticker).await?;
        
        let mut output = format!("=== Последние новости по {} ===\n\n", ticker);
        output.push_str(&result.answer);
        output.push_str("\n\n=== Источники ===\n");
        
        for (i, source) in result.sources.iter().enumerate() {
            output.push_str(&format!("{}. {} - {}\n", i + 1, source.title, source.url));
        }
        
        Ok(output)
    }

    /// Поиск аналитических рейтингов
    pub async fn search_analyst_ratings(&self, ticker: String, company_name: String) -> Result<String> {
        let result = self.provider.search_analyst_ratings(&ticker, &company_name).await?;
        
        let mut output = format!("=== Аналитические рейтинги {} ({}) ===\n\n", company_name, ticker);
        output.push_str(&result.answer);
        output.push_str("\n\n=== Источники ===\n");
        
        for (i, source) in result.sources.iter().enumerate() {
            output.push_str(&format!("{}. {} - {}\n", i + 1, source.title, source.url));
        }
        
        Ok(output)
    }

    /// Общий поисковой запрос
    pub async fn search(&self, query: String) -> Result<String> {
        let result = self.provider.search(&query).await?;
        
        let mut output = format!("=== Результаты поиска ===\n\n{}\n\n=== Источники ===\n", result.answer);
        
        for (i, source) in result.sources.iter().enumerate() {
            output.push_str(&format!("{}. {} - {}\n", i + 1, source.title, source.url));
        }
        
        Ok(output)
    }
}
