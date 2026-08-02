use crate::agent::Action;
use crate::ml_inference::nlp::FinBertInference;
use crate::provider::prediction::{Prediction, PredictionContext};
use anyhow::Result;
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Clone)]
pub struct FinBertPredictor {
    inference: Arc<FinBertInference>,
}

impl std::fmt::Debug for FinBertPredictor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FinBertPredictor").finish()
    }
}

impl FinBertPredictor {
    pub fn new(inference: Arc<FinBertInference>) -> Self {
        Self { inference }
    }

    pub async fn predict(&self, ctx: &PredictionContext) -> Result<Prediction> {
        let mut features = HashMap::new();
        let mut rationale_parts = Vec::new();

        let news = match &ctx.news {
            Some(n) if n.articles_count > 0 => n,
            _ => {
                return Ok(Prediction {
                    action: Action::Hold,
                    confidence: 0.0,
                    conviction: 0.0,
                    features,
                    metadata: serde_json::json!({"method": "finbert", "reason": "no_news"}),
                    provider: "finbert".to_string(),
                    rationale: "no news data available".to_string(),
                });
            }
        };

        let texts: Vec<String> = news
            .articles
            .iter()
            .take(10)
            .map(|a| format!("{}: {}", a.source, a.title))
            .collect();

        if texts.is_empty() {
            return Ok(Prediction {
                action: Action::Hold,
                confidence: 0.0,
                conviction: 0.0,
                features,
                metadata: serde_json::json!({"method": "finbert", "reason": "empty_articles"}),
                provider: "finbert".to_string(),
                rationale: "no articles to analyze".to_string(),
            });
        }

        let inference = self.inference.clone();
        let texts_clone = texts.clone();
        let results = tokio::task::spawn_blocking(move || {
            texts_clone
                .iter()
                .map(|t| inference.predict(t))
                .collect::<Result<Vec<_>>>()
        })
        .await??;

        let mut total_pos = 0.0f64;
        let mut total_neg = 0.0f64;
        let mut total_neu = 0.0f64;
        let mut total_confidence = 0.0f64;
        let mut all_key_events = Vec::new();

        for (text, result) in texts.iter().zip(results.iter()) {
            let scores = &result.scores;
            total_neg += scores[0] as f64;
            total_neu += scores[1] as f64;
            total_pos += scores[2] as f64;
            total_confidence += result.confidence as f64;

            let key_events = crate::analysis::key_events::extract_key_events(text);
            all_key_events.extend(key_events);
        }

        let count = texts.len() as f64;
        let avg_pos = total_pos / count;
        let avg_neg = total_neg / count;
        let avg_neu = total_neu / count;
        let avg_confidence = total_confidence / count;

        let conviction = (avg_pos - avg_neg) as f64;
        let sentiment_score = conviction;

        features.insert("finbert_pos".into(), avg_pos);
        features.insert("finbert_neg".into(), avg_neg);
        features.insert("finbert_neu".into(), avg_neu);
        features.insert("finbert_confidence".into(), avg_confidence);
        features.insert("finbert_conviction".into(), conviction);

        let action = if conviction > 0.15 && avg_pos > avg_neg {
            Action::Buy
        } else if conviction < -0.15 && avg_neg > avg_pos {
            Action::Sell
        } else {
            Action::Hold
        };

        all_key_events.sort();
        all_key_events.dedup();

        rationale_parts.push(format!(
            "pos={:.2} neg={:.2} neu={:.2}",
            avg_pos, avg_neg, avg_neu
        ));
        if !all_key_events.is_empty() {
            rationale_parts.push(format!("events: {}", all_key_events.join(", ")));
        }

        Ok(Prediction {
            action,
            confidence: conviction.abs().clamp(0.05, 0.95),
            conviction,
            features,
            metadata: serde_json::json!({
                "method": "finbert_ensemble",
                "articles_count": texts.len(),
                "avg_confidence": avg_confidence,
            }),
            provider: "finbert".to_string(),
            rationale: rationale_parts.join("; "),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analysis::news::{NewsArticle, NewsSentiment, Sentiment};

    fn make_ctx_with_news(articles_count: usize) -> PredictionContext {
        let articles: Vec<NewsArticle> = (0..articles_count)
            .map(|i| NewsArticle {
                title: format!("Article {}", i),
                content: String::new(),
                source: "test".to_string(),
                url: String::new(),
                published_at: None,
                sentiment: None,
            })
            .collect();

        let mut ctx = PredictionContext::new("TEST");
        ctx.news = Some(NewsSentiment {
            ticker: "TEST".to_string(),
            overall_sentiment: Sentiment::Neutral,
            sentiment_score: 0.0,
            articles_count: articles.len(),
            articles,
            key_events: vec![],
        });
        ctx
    }

    #[ignore]
    #[tokio::test]
    async fn test_finbert_predictor_no_news() {
        let inference = Arc::new(FinBertInference::new("models/finbert").unwrap());
        let predictor = FinBertPredictor::new(inference);
        let ctx = PredictionContext::new("TEST");
        let pred = predictor.predict(&ctx).await.unwrap();
        assert_eq!(pred.action, Action::Hold);
        assert_eq!(pred.confidence, 0.0);
    }

    #[ignore]
    #[tokio::test]
    async fn test_finbert_predictor_empty_articles() {
        let inference = Arc::new(FinBertInference::new("models/finbert").unwrap());
        let predictor = FinBertPredictor::new(inference);
        let ctx = make_ctx_with_news(0);
        let pred = predictor.predict(&ctx).await.unwrap();
        assert_eq!(pred.action, Action::Hold);
    }
}
