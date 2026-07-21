use anyhow::Result;
use std::collections::HashMap;
use crate::agent::Action;
use crate::provider::prediction::{Prediction, PredictionContext};
use mcp_client::ollama::OllamaProvider;
use mcp_client::llm_provider::LLMProvider;

#[derive(Clone)]
pub struct LLMPredictor {
    llm_provider: OllamaProvider,
    model_name: String,
}

impl std::fmt::Debug for LLMPredictor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LLMPredictor")
            .field("model_name", &self.model_name)
            .finish()
    }
}

impl LLMPredictor {
    pub fn new(llm_provider: OllamaProvider, model_name: &str) -> Self {
        LLMPredictor {
            llm_provider,
            model_name: model_name.to_string(),
        }
    }

    pub async fn predict(&self, ctx: &PredictionContext) -> Result<Prediction> {
        let prompt = self.build_prompt(ctx);
        let response = self.llm_provider.send_message(prompt).await?;
        let content = &response.message.content;
        self.parse_response(content, ctx)
    }

    fn build_prompt(&self, ctx: &PredictionContext) -> String {
        let mut prompt = String::from(
            "You are a professional financial analyst. Analyze the data and respond in JSON.\n\n"
        );

        prompt.push_str(&format!("Ticker: {} ({})\n", ctx.ticker, ctx.company_name));
        prompt.push_str(&format!("Current price: {:.2}\n", ctx.current_price));
        prompt.push_str(&format!("Market regime: {:?}\n", ctx.regime));

        if let Some(ref news) = ctx.news {
            prompt.push_str(&format!(
                "News sentiment: {:?} (score: {:.2})\n",
                news.overall_sentiment, news.sentiment_score
            ));
            if !news.key_events.is_empty() {
                prompt.push_str(&format!("Key events: {}\n", news.key_events.join(", ")));
            }
        }

        if let Some(ref tech) = ctx.technical {
            prompt.push_str(&format!("RSI: {:?}\n", tech.rsi));
            if let Some(ref macd) = tech.macd {
                prompt.push_str(&format!("MACD hist: {:.4}\n", macd.histogram));
            }
            prompt.push_str(&format!("Trend: {:?}\n", tech.trend));
            prompt.push_str(&format!("Recommendation: {:?}\n", tech.recommendation));
        }

        if let Some(ref fund) = ctx.fundamental {
            prompt.push_str(&format!("Fundamental rating: {:?}\n", fund.rating));
        }

        prompt.push_str("\nRespond ONLY with valid JSON:\n");
        prompt.push_str(r#"{"action":"BUY/SELL/HOLD","confidence":0.0-1.0,"conviction":-1.0-1.0,"rationale":"..."}"#);
        prompt
    }

    fn parse_response(&self, content: &str, _ctx: &PredictionContext) -> Result<Prediction> {
        let json_start = content.find('{').unwrap_or(0);
        let json_end = content.rfind('}').unwrap_or(content.len());
        if json_start >= json_end {
            anyhow::bail!("No valid JSON in LLM response");
        }
        let json_str = &content[json_start..=json_end];

        let parsed: serde_json::Value = serde_json::from_str(json_str)
            .map_err(|e| anyhow::anyhow!("LLM JSON parse error: {}", e))?;

        let action_str = parsed["action"].as_str().unwrap_or("HOLD");
        let action = match action_str.to_uppercase().as_str() {
            "BUY" => Action::Buy,
            "SELL" => Action::Sell,
            _ => Action::Hold,
        };
        let confidence = parsed["confidence"].as_f64().unwrap_or(0.5).clamp(0.0, 1.0);
        let conviction = parsed["conviction"].as_f64().unwrap_or(0.0).clamp(-1.0, 1.0);
        let rationale = parsed["rationale"].as_str().unwrap_or("").to_string();

        Ok(Prediction {
            action,
            confidence,
            conviction,
            features: HashMap::new(),
            metadata: serde_json::json!({"model": self.model_name}),
            provider: "llm".to_string(),
            rationale,
        })
    }
}
