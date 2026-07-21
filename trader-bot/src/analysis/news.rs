use anyhow::Result;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Результат поиска новостей
#[derive(Debug, Clone)]
pub struct NewsArticle {
    pub title: String,
    pub content: String,
    pub source: String,
    pub url: String,
    pub published_at: Option<String>,
    pub sentiment: Option<Sentiment>,
}

/// Тональность новости
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Sentiment {
    Positive,
    Negative,
    Neutral,
}

impl Sentiment {
    /// Конвертация в числовое значение
    pub fn to_score(&self) -> f64 {
        match self {
            Sentiment::Positive => 1.0,
            Sentiment::Negative => -1.0,
            Sentiment::Neutral => 0.0,
        }
    }

    /// Конвертация из числового значения
    pub fn from_score(score: f64) -> Self {
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

/// Агрегированный новостной фон по инструменту
#[derive(Debug, Clone)]
pub struct NewsSentiment {
    pub ticker: String,
    pub overall_sentiment: Sentiment,
    pub sentiment_score: f64, // от -1.0 до 1.0
    pub articles_count: usize,
    pub articles: Vec<NewsArticle>,
    pub key_events: Vec<String>,
}

/// Сервис для анализа новостей
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

    /// Анализ новостей по инструменту
    pub async fn analyze(&self, ticker: &str, company_name: &str) -> Result<NewsSentiment> {
        let mut articles = Vec::new();

        // Поиск новостей по источникам
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

        // Анализ тональности
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

    /// Поиск новостей в источнике Tinkoff
    async fn search_tinkoff_news(&self, ticker: &str) -> Result<Vec<NewsArticle>> {
        // Используем DuckDuckGo для поиска новостей Tinkoff Invest
        let query = format!("{} акции новости site:tinkoff.ru", ticker);
        let articles = self.duckduckgo_search(&query).await?;
        Ok(articles)
    }

    /// Поиск новостей в Investing.com
    async fn search_investing_news(&self, ticker: &str) -> Result<Vec<NewsArticle>> {
        let query = format!("{} акции новости сайт:investing.com", ticker);
        let articles = self.duckduckgo_search(&query).await?;
        Ok(articles)
    }

    /// Поиск новостей в Bloomberg
    async fn search_bloomberg_news(&self, company_name: &str) -> Result<Vec<NewsArticle>> {
        let query = format!("{} stock news site:bloomberg.com", company_name);
        let articles = self.duckduckgo_search(&query).await?;
        Ok(articles)
    }

    /// Поиск через DuckDuckGo
    async fn duckduckgo_search(&self, query: &str) -> Result<Vec<NewsArticle>> {
        // Примечание: Это упрощенная реализация
        // В продакшене нужно использовать полноценный API или парсинг
        let url = format!("https://html.duckduckgo.com/html/?q={}", query);

        match self.client.get(&url).send().await {
            Ok(response) => {
                let text = response.text().await?;
                // Упрощенный парсинг - в реальности нужно использовать HTML парсер
                Ok(vec![]) // Заглушка - реальная реализация через task agent
            }
            Err(_) => Ok(vec![]),
        }
    }

    /// Расчет общего sentiment score
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

    /// Конвертация score в Sentiment
    fn score_to_sentiment(&self, score: f64) -> Sentiment {
        Sentiment::from_score(score)
    }

    /// Извлечение ключевых событий из новостей
    async fn extract_key_events(&self, articles: &[NewsArticle]) -> Result<Vec<String>> {
        // Здесь будет LLM анализ для извлечения ключевых событий
        // Например: "расконвертация", "дивиденды", "отчетность"
        let mut events = Vec::new();

        for article in articles {
            let title_lower = article.title.to_lowercase();

            if title_lower.contains("расконверт") {
                events.push("Расконвертация акций".to_string());
            }
            if title_lower.contains("дивиденд") {
                events.push("Дивидендные выплаты".to_string());
            }
            if title_lower.contains("отчет") || title_lower.contains("финансовый результат")
            {
                events.push("Финансовая отчетность".to_string());
            }
            if title_lower.contains("санкц") || title_lower.contains("ограничен") {
                events.push("Санкционное давление".to_string());
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
        // Проверяем, что from_score(to_score(x)) == x
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
