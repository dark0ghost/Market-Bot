use anyhow::Result;
use mcp_client::ollama::OllamaProvider;
use mcp_client::llm_provider::LLMProvider;
use std::sync::Arc;
use super::news::Sentiment;

/// Сервис для LLM-анализа новостей
pub struct NewsLLMService {
    llm_provider: Arc<OllamaProvider>,
}

impl NewsLLMService {
    pub fn new(llm_provider: OllamaProvider) -> Self {
        NewsLLMService {
            llm_provider: Arc::new(llm_provider),
        }
    }

    /// Анализ тональности новости
    pub async fn analyze_sentiment(&self, title: &str, content: &str) -> Result<NewsSentimentResult> {
        let prompt = self.build_sentiment_prompt(title, content);
        
        let response = self.llm_provider.send_message(prompt).await?;
        let result = self.parse_sentiment_response(&response.message.content)?;
        
        Ok(result)
    }

    /// Анализ набора новостей для инструмента
    pub async fn analyze_news_batch(&self, ticker: &str, company_name: &str, news_items: &[NewsItem]) -> Result<BatchSentimentResult> {
        let prompt = self.build_batch_prompt(ticker, company_name, news_items);
        
        let response = self.llm_provider.send_message(prompt).await?;
        let result = self.parse_batch_response(&response.message.content)?;
        
        Ok(result)
    }

    /// Построение промпта для анализа одной новости
    fn build_sentiment_prompt(&self, title: &str, content: &str) -> String {
        format!(
            r#"Ты - финансовый аналитик. Проанализируй новость и определи её тональность для инвесторов.

Заголовок: {}
Содержание: {}

Оцени тональность по шкале от -1.0 до 1.0:
- -1.0 = крайне негативно для акций компании
- 0.0 = нейтрально
- 1.0 = крайне позитивно для акций компании

Также выдели ключевые события из новости.

Ответь в формате JSON:
{{
    "sentiment_score": число от -1.0 до 1.0,
    "sentiment": "Positive" | "Negative" | "Neutral",
    "key_events": ["событие 1", "событие 2"],
    "confidence": число от 0.0 до 1.0,
    "explanation": "краткое объяснение"
}}"#,
            title, content
        )
    }

    /// Построение промпта для анализа набора новостей
    fn build_batch_prompt(&self, ticker: &str, company_name: &str, news_items: &[NewsItem]) -> String {
        let mut news_text = String::new();
        for (i, item) in news_items.iter().enumerate() {
            news_text.push_str(&format!("{}. [{}] {}\n", i + 1, item.source, item.title));
        }

        format!(
            r#"Ты - профессиональный торговый аналитик. Проанализируй новостной фон по компании {} (тикер: {}).

Список последних новостей:
{}

Задачи:
1. Определи общий сентимент новостей для акций компании
2. Выдели ключевые события, которые могут повлиять на цену акций
3. Оцени степень влияния новостей на инвестиционную привлекательность

Ответь в формате JSON:
{{
    "overall_sentiment": "Positive" | "Negative" | "Neutral",
    "sentiment_score": число от -1.0 до 1.0,
    "key_events": ["событие 1", "событие 2"],
    "risks": ["риск 1", "риск 2"],
    "opportunities": ["возможность 1", "возможность 2"],
    "summary": "краткое резюме новостного фона",
    "confidence": число от 0.0 до 1.0
}}"#,
            company_name, ticker, news_text
        )
    }

    /// Парсинг ответа для одной новости
    fn parse_sentiment_response(&self, content: &str) -> Result<NewsSentimentResult> {
        // Поиск JSON в ответе
        let json_start = content.find('{').unwrap_or(0);
        let json_end = content.rfind('}').unwrap_or(content.len());
        let json_str = &content[json_start..=json_end];

        let parsed: serde_json::Value = serde_json::from_str(json_str)
            .unwrap_or_else(|_| serde_json::json!({}));

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
            .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
            .unwrap_or_default();

        Ok(NewsSentimentResult {
            sentiment,
            sentiment_score: score,
            confidence,
            key_events,
            explanation,
        })
    }

    /// Парсинг ответа для набора новостей
    fn parse_batch_response(&self, content: &str) -> Result<BatchSentimentResult> {
        // Поиск JSON в ответе
        let json_start = content.find('{').unwrap_or(0);
        let json_end = content.rfind('}').unwrap_or(content.len());
        let json_str = &content[json_start..=json_end];

        let parsed: serde_json::Value = serde_json::from_str(json_str)
            .unwrap_or_else(|_| serde_json::json!({}));

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
            .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
            .unwrap_or_default();

        let risks = parsed["risks"]
            .as_array()
            .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
            .unwrap_or_default();

        let opportunities = parsed["opportunities"]
            .as_array()
            .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
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

/// Результат анализа одной новости
#[derive(Debug, Clone)]
pub struct NewsSentimentResult {
    pub sentiment: Sentiment,
    pub sentiment_score: f64,
    pub confidence: f64,
    pub key_events: Vec<String>,
    pub explanation: String,
}

/// Результат анализа набора новостей
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

/// Элемент новости для анализа
#[derive(Debug, Clone)]
pub struct NewsItem {
    pub title: String,
    pub content: String,
    pub source: String,
    pub url: String,
}
