use crate::analysis::{
    CompanyRating, FundamentalAnalysis, MarketRegime, NewsSentiment, Recommendation, Sentiment,
    TechnicalAnalysis, Trend,
};
use crate::config::RiskManagementConfig;
use anyhow::Result;
use mcp_client::llm_provider::LLMProvider;
use mcp_client::ollama::OllamaProvider;
use serde::{Deserialize, Serialize};

/// Trading agent decision
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradingDecision {
    pub ticker: String,
    pub action: Action,
    pub confidence: f64,          // 0.0 - 1.0
    pub entry_price: Option<f64>, // Recommended entry price
    pub position_size_pct: f64,   // Portfolio share (0.0 - 1.0)
    pub stop_loss: Option<f64>,
    pub take_profit: Option<f64>,
    pub rationale: String,
    pub risks: Vec<String>,
    pub time_horizon: TimeHorizon,
    pub current_position: Option<i32>, // Current position in lots (for Sell)
    pub current_price: f64,            // Current instrument price
}

/// Action type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum Action {
    Buy,
    Sell,
    Hold,
}

/// Investment horizon
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TimeHorizon {
    Short,  // 1-7 days
    Medium, // 1-4 weeks
    Long,   // 1+ months
}

/// Decision context
#[derive(Debug, Clone)]
pub struct DecisionContext {
    pub ticker: String,
    pub company_name: String,
    pub current_price: f64,
    pub news_sentiment: Option<NewsSentiment>,
    pub technical_analysis: Option<TechnicalAnalysis>,
    pub fundamental_analysis: Option<FundamentalAnalysis>,
    pub available_balance: f64,
    pub current_position: Option<CurrentPosition>,
    pub risk_config: Option<RiskManagementConfig>,
    pub max_position_pct: f64,
    pub market_regime: MarketRegime,
    pub candles: Vec<crate::client::order_book::OrderBookLevel>,
}

/// Current position
#[derive(Debug, Clone)]
pub struct CurrentPosition {
    pub quantity: i32,
    pub average_price: f64,
    pub current_value: f64,
}

/// LLM-based trading agent
pub struct TradingAgent {
    llm_provider: OllamaProvider,
    model_name: String,
}

impl TradingAgent {
    pub fn new(llm_provider: OllamaProvider, model_name: String) -> Self {
        TradingAgent {
            llm_provider,
            model_name,
        }
    }

    /// Make trading decision
    pub async fn make_decision(&self, context: DecisionContext) -> Result<TradingDecision> {
        // Build prompt for LLM
        let prompt = self.build_decision_prompt(&context);

        // Request to LLM
        let llm_response = self.llm_provider.send_message(prompt).await?;

        // Parse LLM response
        let decision = self.parse_llm_response(&llm_response.message.content, &context)?;

        Ok(decision)
    }

    /// Fast decision without LLM (rule-based)
    pub fn make_rule_based_decision(&self, context: DecisionContext) -> Result<TradingDecision> {
        let mut action = Action::Hold;
        let mut confidence = 0.5;
        let mut rationale_parts = Vec::new();
        let mut risks = Vec::new();

        // News analysis
        if let Some(news) = &context.news_sentiment {
            match news.overall_sentiment {
                Sentiment::Positive => {
                    confidence += 0.1;
                    rationale_parts.push(format!(
                        "Positive news flow (score: {:.2}, articles: {})",
                        news.sentiment_score, news.articles_count
                    ));
                }
                Sentiment::Negative => {
                    confidence -= 0.1;
                    risks.push("Negative news flow".to_string());
                    rationale_parts.push(format!(
                        "Negative news flow (score: {:.2})",
                        news.sentiment_score
                    ));
                }
                Sentiment::Neutral => {
                    rationale_parts.push("Neutral news flow".to_string());
                }
            }

            // Key events
            if !news.key_events.is_empty() {
                rationale_parts.push(format!("Key events: {}", news.key_events.join(", ")));
            }
        }

        // Technical analysis
        if let Some(tech) = &context.technical_analysis {
            match tech.recommendation {
                Recommendation::StrongBuy => {
                    action = Action::Buy;
                    confidence += 0.2;
                    rationale_parts.push("Technical analysis: Strong Buy".to_string());
                }
                Recommendation::Buy => {
                    action = Action::Buy;
                    confidence += 0.15;
                    rationale_parts.push("Technical analysis: Buy".to_string());
                }
                Recommendation::Sell => {
                    action = Action::Sell;
                    confidence += 0.15;
                    rationale_parts.push("Technical analysis: Sell".to_string());
                }
                Recommendation::StrongSell => {
                    action = Action::Sell;
                    confidence += 0.2;
                    rationale_parts.push("Technical analysis: Strong Sell".to_string());
                }
                Recommendation::Hold => {
                    rationale_parts.push("Technical analysis: Hold".to_string());
                }
            }

            // Trend
            match tech.trend {
                Trend::Bullish => {
                    rationale_parts.push("Bullish trend".to_string());
                    if action == Action::Hold {
                        action = Action::Buy;
                        confidence += 0.1;
                    }
                }
                Trend::Bearish => {
                    rationale_parts.push("Bearish trend".to_string());
                    if action == Action::Hold {
                        action = Action::Sell;
                        confidence += 0.1;
                    }
                }
                Trend::Sideways => {
                    rationale_parts.push("Sideways movement".to_string());
                }
            }

            // Levels
            if !tech.support_levels.is_empty() {
                if let Some(&support) = tech.support_levels.first() {
                    if context.current_price <= support * 1.05 {
                        rationale_parts.push(format!("Price at support: {:.2}", support));
                        if action == Action::Hold {
                            action = Action::Buy;
                            confidence += 0.1;
                        }
                    }
                }
            }
        }

        // Fundamental analysis
        if let Some(fund) = &context.fundamental_analysis {
            match fund.rating {
                CompanyRating::Excellent => {
                    confidence += 0.15;
                    rationale_parts.push(format!(
                        "Excellent fundamentals (score: {:.1})",
                        fund.overall_score
                    ));
                    if action == Action::Hold {
                        action = Action::Buy;
                    }
                }
                CompanyRating::Good => {
                    confidence += 0.1;
                    rationale_parts.push(format!(
                        "Good fundamentals (score: {:.1})",
                        fund.overall_score
                    ));
                }
                CompanyRating::Poor | CompanyRating::VeryPoor => {
                    confidence -= 0.1;
                    risks.push("Weak fundamentals".to_string());
                }
                _ => {}
            }

            // Risks
            if !fund.key_risks.is_empty() {
                risks.extend(fund.key_risks.iter().take(3).cloned());
            }
        }

        // Calculate position size
        let position_size_pct = self.calculate_position_size(&context, &action, confidence);

        // Stop Loss and Take Profit
        let (stop_loss, take_profit) =
            self.calculate_levels(context.current_price, &action, &context.risk_config);

        // Time horizon
        let time_horizon = self.determine_time_horizon(&context);

        // Clamp confidence
        confidence = confidence.min(0.95).max(0.05);

        let rationale = rationale_parts.join("; ");

        // Get current position in lots
        let current_position = context.current_position.as_ref().map(|p| p.quantity);

        Ok(TradingDecision {
            ticker: context.ticker,
            action,
            confidence,
            entry_price: Some(context.current_price),
            position_size_pct,
            stop_loss,
            take_profit,
            rationale,
            risks,
            time_horizon,
            current_position,
            current_price: context.current_price,
        })
    }

    /// Calculate position size
    fn calculate_position_size(
        &self,
        context: &DecisionContext,
        action: &Action,
        confidence: f64,
    ) -> f64 {
        if *action != Action::Buy {
            return 0.0;
        }

        // Base size from confidence
        let base_size = confidence * context.max_position_pct;

        // Adjust for volatility and risk
        let adjusted_size = if let Some(tech) = &context.technical_analysis {
            // If price is near resistance - reduce position
            if !tech.resistance_levels.is_empty() {
                if let Some(&resistance) = tech.resistance_levels.first() {
                    let distance_to_resistance =
                        (resistance - context.current_price) / context.current_price;
                    if distance_to_resistance < 0.02 {
                        base_size * 0.5 // Reduce by 50%
                    } else {
                        base_size
                    }
                } else {
                    base_size
                }
            } else {
                base_size
            }
        } else {
            base_size
        };

        // Account for available balance
        let max_affordable = if context.current_price > 0.0 {
            (context.available_balance * (1.0 - 0.1)) / context.current_price // 10% reserve
        } else {
            0.0
        };

        // Return share, not exceeding limits
        adjusted_size.min(context.max_position_pct)
    }

    /// Calculate Stop Loss and Take Profit levels
    fn calculate_levels(
        &self,
        current_price: f64,
        action: &Action,
        risk_config: &Option<RiskManagementConfig>,
    ) -> (Option<f64>, Option<f64>) {
        let (sl_pct, tp_pct) = if let Some(config) = risk_config {
            (config.stop_loss_pct, config.take_profit_pct)
        } else {
            (0.03, 0.10) // Default values
        };

        match action {
            Action::Buy => {
                let sl = Some(current_price * (1.0 - sl_pct));
                let tp = Some(current_price * (1.0 + tp_pct));
                (sl, tp)
            }
            Action::Sell => {
                let sl = Some(current_price * (1.0 + sl_pct));
                let tp = Some(current_price * (1.0 - tp_pct));
                (sl, tp)
            }
            Action::Hold => (None, None),
        }
    }

    /// Determine time horizon
    fn determine_time_horizon(&self, context: &DecisionContext) -> TimeHorizon {
        // If fundamental analysis has high rating - long horizon
        if let Some(fund) = &context.fundamental_analysis {
            if fund.rating == CompanyRating::Excellent || fund.rating == CompanyRating::Good {
                return TimeHorizon::Long;
            }
        }

        // If strong technical signal - short horizon
        if let Some(tech) = &context.technical_analysis {
            if tech.recommendation == Recommendation::StrongBuy
                || tech.recommendation == Recommendation::StrongSell
            {
                return TimeHorizon::Short;
            }
        }

        TimeHorizon::Medium
    }

    /// Build prompt for LLM
    fn build_decision_prompt(&self, context: &DecisionContext) -> String {
        let mut prompt = format!(
            "You are a professional trading analyst. Analyze the data for stock {} ({}) and provide a recommendation.\n\n",
            context.ticker, context.company_name
        );

        prompt.push_str(&format!("Current price: {:.2}\n\n", context.current_price));

        // News
        if let Some(news) = &context.news_sentiment {
            prompt.push_str("NEWS:\n");
            prompt.push_str(&format!(
                "Overall sentiment: {:?} (score: {:.2})\n",
                news.overall_sentiment, news.sentiment_score
            ));
            prompt.push_str(&format!("Article count: {}\n", news.articles_count));
            if !news.key_events.is_empty() {
                prompt.push_str("Key events:\n");
                for event in &news.key_events {
                    prompt.push_str(&format!("  - {}\n", event));
                }
            }
            prompt.push('\n');
        }

        // Technical analysis
        if let Some(tech) = &context.technical_analysis {
            prompt.push_str("TECHNICAL ANALYSIS:\n");
            prompt.push_str(&format!("Trend: {:?}\n", tech.trend));
            prompt.push_str(&format!("Recommendation: {:?}\n", tech.recommendation));
            if let Some(rsi) = tech.rsi {
                prompt.push_str(&format!("RSI: {:.2}\n", rsi));
            }
            if let Some(ref macd) = tech.macd {
                prompt.push_str(&format!(
                    "MACD: {:.3} (signal: {:.3}, histogram: {:.3})\n",
                    macd.macd_line, macd.signal_line, macd.histogram
                ));
            }
            if let Some(ref bb) = tech.bollinger {
                prompt.push_str(&format!(
                    "Bollinger: upper={:.2}, middle={:.2}, lower={:.2}\n",
                    bb.upper, bb.middle, bb.lower
                ));
            }
            prompt.push_str(&format!(
                "Support: {:?}\n",
                tech.support_levels.iter().take(2).collect::<Vec<_>>()
            ));
            prompt.push_str(&format!(
                "Resistance: {:?}\n",
                tech.resistance_levels.iter().take(2).collect::<Vec<_>>()
            ));
            prompt.push('\n');
        }

        // Fundamental analysis
        if let Some(fund) = &context.fundamental_analysis {
            prompt.push_str("FUNDAMENTAL ANALYSIS:\n");
            prompt.push_str(&format!(
                "Рейтинг: {:?} (score: {:.1}/100)\n",
                fund.rating, fund.overall_score
            ));
            if let Some(pe) = fund.valuation.pe_ratio {
                prompt.push_str(&format!("P/E: {:.2}\n", pe));
            }
            if let Some(roe) = fund.profitability.roe {
                prompt.push_str(&format!("ROE: {:.2}%\n", roe));
            }
            if let Some(dte) = fund.financial_health.debt_to_equity {
                prompt.push_str(&format!("D/E: {:.2}\n", dte));
            }
            if let Some(growth) = fund.growth.revenue_growth_yoy {
                prompt.push_str(&format!("Revenue growth (YoY): {:.2}%\n", growth));
            }
            if !fund.key_risks.is_empty() {
                prompt.push_str("Risks:\n");
                for risk in &fund.key_risks {
                    prompt.push_str(&format!("  - {}\n", risk));
                }
            }
            if !fund.key_strengths.is_empty() {
                prompt.push_str("Strengths:\n");
                for strength in &fund.key_strengths {
                    prompt.push_str(&format!("  - {}\n", strength));
                }
            }
            prompt.push('\n');
        }

        // Portfolio context
        prompt.push_str("PORTFOLIO CONTEXT:\n");
        prompt.push_str(&format!(
            "Available balance: {:.2}\n",
            context.available_balance
        ));
        prompt.push_str(&format!(
            "Max position share: {:.1}%\n",
            context.max_position_pct * 100.0
        ));
        if let Some(pos) = &context.current_position {
            prompt.push_str(&format!(
                "Current position: {} lots at avg {:.2}\n",
                pos.quantity, pos.average_price
            ));
        }
        prompt.push('\n');

        // Request
        prompt.push_str("TASK:\n");
        prompt.push_str("Provide a recommendation in JSON format:\n");
        prompt.push_str("{\n");
        prompt.push_str("  \"action\": \"BUY\" | \"SELL\" | \"HOLD\",\n");
        prompt.push_str("  \"confidence\": 0.0-1.0,\n");
        prompt.push_str("  \"entry_price\": entry price,\n");
        prompt.push_str("  \"position_size_pct\": portfolio share 0.0-1.0,\n");
        prompt.push_str("  \"stop_loss\": stop-loss price,\n");
        prompt.push_str("  \"take_profit\": take-profit price,\n");
        prompt.push_str("  \"rationale\": \"rationale\",\n");
        prompt.push_str("  \"risks\": [\"risk1\", \"risk2\"],\n");
        prompt.push_str("  \"time_horizon\": \"SHORT\" | \"MEDIUM\" | \"LONG\"\n");
        prompt.push_str("}\n");

        prompt
    }

    /// Parse LLM response
    ///
    /// # Errors
    /// Returns an error if JSON is invalid or required fields are missing
    fn parse_llm_response(
        &self,
        content: &str,
        context: &DecisionContext,
    ) -> Result<TradingDecision> {
        // Find JSON in response
        let json_start = content
            .find('{')
            .ok_or_else(|| anyhow::anyhow!("No JSON found in LLM response"))?;
        let json_end = content
            .rfind('}')
            .ok_or_else(|| anyhow::anyhow!("No closing JSON symbol found"))?;

        if json_start > json_end {
            anyhow::bail!("Invalid JSON: opening brace after closing brace");
        }

        let json_str = &content[json_start..=json_end];

        // Parse JSON with error handling
        let parsed: serde_json::Value = serde_json::from_str(json_str)
            .map_err(|e| anyhow::anyhow!("JSON parse error: {}. JSON: {}", e, json_str))?;

        // Extract fields
        let action_str = parsed["action"].as_str().unwrap_or("HOLD");
        let action = match action_str.to_uppercase().as_str() {
            "BUY" => Action::Buy,
            "SELL" => Action::Sell,
            _ => Action::Hold,
        };

        let confidence = parsed["confidence"].as_f64().unwrap_or(0.5);
        let entry_price = parsed["entry_price"].as_f64();
        let position_size_pct = parsed["position_size_pct"].as_f64().unwrap_or(0.0);
        let stop_loss = parsed["stop_loss"].as_f64();
        let take_profit = parsed["take_profit"].as_f64();
        let rationale = parsed["rationale"]
            .as_str()
            .unwrap_or("No rationale provided")
            .to_string();

        let risks = parsed["risks"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();

        let time_horizon_str = parsed["time_horizon"].as_str().unwrap_or("MEDIUM");
        let time_horizon = match time_horizon_str.to_uppercase().as_str() {
            "SHORT" => TimeHorizon::Short,
            "LONG" => TimeHorizon::Long,
            _ => TimeHorizon::Medium,
        };

        // Get current position in lots
        let current_position = context.current_position.as_ref().map(|p| p.quantity);

        Ok(TradingDecision {
            ticker: context.ticker.clone(),
            action,
            confidence,
            entry_price,
            position_size_pct,
            stop_loss,
            take_profit,
            rationale,
            risks,
            time_horizon,
            current_position,
            current_price: context.current_price,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rule_based_decision_buy() {
        let context = DecisionContext {
            ticker: "TTECH".to_string(),
            company_name: "T-Technologies".to_string(),
            current_price: 100.0,
            news_sentiment: Some(NewsSentiment {
                ticker: "TTECH".to_string(),
                overall_sentiment: Sentiment::Positive,
                sentiment_score: 0.5,
                articles_count: 5,
                articles: vec![],
                key_events: vec!["Positive news".to_string()],
            }),
            technical_analysis: Some(TechnicalAnalysis {
                ticker: "TTECH".to_string(),
                timestamp: chrono::Utc::now(),
                current_price: 100.0,
                trend: Trend::Bullish,
                rsi: Some(45.0),
                macd: None,
                bollinger: None,
                volume_analysis: crate::analysis::VolumeAnalysis {
                    current_volume: 1000.0,
                    avg_volume: 500.0,
                    volume_ratio: 2.0,
                    is_unusual: true,
                },
                support_levels: vec![95.0],
                resistance_levels: vec![110.0],
                recommendation: Recommendation::Buy,
            }),
            fundamental_analysis: None,
            available_balance: 100000.0,
            current_position: None,
            risk_config: None,
            max_position_pct: 0.15,
            market_regime: MarketRegime::Quiet,
            candles: vec![],
        };

        // For the test, a real LLM provider is needed, so skip LLM test
        // let agent = TradingAgent::new(OllamaProvider::default(), "fin-expert".to_string());
        // let decision = agent.make_rule_based_decision(context).await.unwrap();

        // Check context structure
        assert_eq!(context.ticker, "TTECH");
        assert!(context.news_sentiment.is_some());
        assert!(context.technical_analysis.is_some());
    }
}
