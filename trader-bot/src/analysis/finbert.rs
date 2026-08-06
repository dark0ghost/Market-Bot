use super::news::Sentiment;
use super::news_llm::{BatchSentimentResult, NewsItem, NewsSentimentAnalyzer};
use crate::ml_inference::nlp::{FinBertInference, NlpResult};
use anyhow::Result;
use async_trait::async_trait;
use std::sync::Arc;

pub struct FinBertSentimentService {
    inference: Arc<FinBertInference>,
}

impl FinBertSentimentService {
    pub fn new(model_dir: &str) -> Result<Self> {
        let inference = Arc::new(FinBertInference::new(model_dir)?);
        inference.clone().enable_hot_reload(model_dir);
        Ok(Self { inference })
    }

    pub async fn analyze(&self, text: &str) -> Result<FinBertSentiment> {
        let result = tokio::task::spawn_blocking({
            let inference = self.inference.clone();
            let text = text.to_string();
            move || inference.predict(&text)
        })
        .await??;

        Ok(FinBertSentiment::from_nlp_result(text, result))
    }

    pub async fn analyze_batch(&self, texts: &[String]) -> Result<Vec<FinBertSentiment>> {
        // Run each inference on the blocking pool concurrently so the async runtime
        // stays responsive. ONNX inference itself serializes on the session Mutex,
        // but this avoids holding a tokio worker thread for the whole batch.
        let futs: Vec<_> = texts
            .iter()
            .map(|t| {
                let inference = self.inference.clone();
                let text = t.clone();
                async move {
                    let text_for_events = text.clone();
                    let result =
                        tokio::task::spawn_blocking(move || inference.predict(&text)).await??;
                    Ok::<_, anyhow::Error>(FinBertSentiment::from_nlp_result(
                        &text_for_events,
                        result,
                    ))
                }
            })
            .collect();
        let results: Vec<FinBertSentiment> = futures::future::try_join_all(futs).await?;
        Ok(results)
    }
}

#[derive(Debug, Clone)]
pub struct FinBertSentiment {
    pub label: String,
    pub confidence: f32,
    pub sentiment_score: f32,
    pub scores: [f32; 3],
    pub key_events: Vec<String>,
}

impl FinBertSentiment {
    fn from_nlp_result(text: &str, result: NlpResult) -> Self {
        let key_events = crate::analysis::key_events::extract_key_events(text);
        let sentiment_score = result.sentiment_score();
        Self {
            label: result.label,
            confidence: result.confidence,
            sentiment_score,
            scores: result.scores,
            key_events,
        }
    }

    pub fn to_sentiment(&self) -> crate::analysis::news::Sentiment {
        crate::analysis::news::Sentiment::from_score(self.sentiment_score as f64)
    }
}

#[async_trait]
impl NewsSentimentAnalyzer for FinBertSentimentService {
    async fn analyze_news_batch(
        &self,
        _ticker: &str,
        _company_name: &str,
        news_items: &[NewsItem],
    ) -> anyhow::Result<BatchSentimentResult> {
        if news_items.is_empty() {
            return Ok(BatchSentimentResult {
                overall_sentiment: Sentiment::Neutral,
                sentiment_score: 0.0,
                confidence: 0.0,
                key_events: vec![],
                risks: vec![],
                opportunities: vec![],
                summary: String::new(),
            });
        }

        let mut total_score = 0.0f64;
        let mut total_confidence = 0.0f64;
        let mut all_key_events = Vec::new();
        let mut risks = Vec::new();

        let texts: Vec<String> = news_items
            .iter()
            .map(|item| format!("{}: {}", item.source, item.title))
            .collect();

        let results = self.analyze_batch(&texts).await?;

        for (item, sentiment) in news_items.iter().zip(results.iter()) {
            total_score += sentiment.sentiment_score as f64;
            total_confidence += sentiment.confidence as f64;
            all_key_events.extend(sentiment.key_events.clone());
            if sentiment.sentiment_score < -0.3 {
                risks.push(format!("Negative news: {}", item.title));
            }
        }

        let count = news_items.len() as f64;
        let avg_score = total_score / count;
        let avg_confidence = total_confidence / count;

        let overall_sentiment = if avg_score > 0.2 {
            Sentiment::Positive
        } else if avg_score < -0.2 {
            Sentiment::Negative
        } else {
            Sentiment::Neutral
        };

        all_key_events.sort();
        all_key_events.dedup();

        Ok(BatchSentimentResult {
            overall_sentiment,
            sentiment_score: avg_score,
            confidence: avg_confidence,
            key_events: all_key_events,
            risks,
            opportunities: vec![],
            summary: format!(
                "FinBERT analysis: {} articles, avg score {:.3}",
                news_items.len(),
                avg_score
            ),
        })
    }
}
