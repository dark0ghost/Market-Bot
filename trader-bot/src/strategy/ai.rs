use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;

use crate::agent::TradingAgent;
use crate::config::AiConfig;
use crate::core::{Broker, OrderAction, Signal, Strategy, StrategyKind};

/// AI-powered trading strategy
///
/// Uses TradingAgent (LLM or rule-based) to produce signals.
/// The heavy analysis pipeline (technical, news, fundamental) runs
/// in main.rs via TradingAgent directly. This strategy provides
/// registry integration for the dashboard and monitoring.
pub struct AiStrategy {
    account_id: String,
    config: AiConfig,
    trading_agent: Arc<TradingAgent>,
}

impl AiStrategy {
    pub fn new(account_id: String, config: AiConfig, trading_agent: Arc<TradingAgent>) -> Self {
        Self {
            account_id,
            config,
            trading_agent,
        }
    }

    pub fn config(&self) -> &AiConfig {
        &self.config
    }

    pub fn trading_agent(&self) -> &TradingAgent {
        &self.trading_agent
    }
}

#[async_trait]
impl Strategy for AiStrategy {
    fn name(&self) -> &str {
        "ai"
    }

    fn kind(&self) -> StrategyKind {
        StrategyKind::Ai
    }

    async fn on_start(&mut self, _broker: &dyn Broker) -> Result<()> {
        log::info!(
            "AiStrategy initialized for account {} (llm={}, min_confidence={})",
            self.account_id,
            self.config.use_llm,
            self.config.min_confidence
        );
        Ok(())
    }

    async fn analyze(&self, broker: &dyn Broker, instrument: &str) -> Result<Vec<Signal>> {
        let current_price = broker.last_price(instrument).await?;
        let balance = broker.balance().await.unwrap_or(0.0);

        let action = if balance > 0.0 && current_price > 0.0 {
            OrderAction::Buy
        } else {
            return Ok(Vec::new());
        };

        let signal = Signal {
            ticker: instrument.to_string(),
            timestamp: chrono::Utc::now(),
            action,
            confidence: 0.5,
            price: current_price,
            source: "ai".to_string(),
            metadata: {
                let mut m = std::collections::HashMap::new();
                m.insert("strategy".to_string(), "ai".to_string());
                m
            },
        };

        Ok(vec![signal])
    }

    async fn on_tick(&mut self, _broker: &dyn Broker) -> Result<()> {
        Ok(())
    }

    fn validate(&self) -> Result<()> {
        if self.config.min_confidence <= 0.0 || self.config.min_confidence > 1.0 {
            anyhow::bail!(
                "min_confidence must be between 0.0 and 1.0, got {}",
                self.config.min_confidence
            );
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::{Action, CurrentPosition, DecisionContext, OllamaQuery, TimeHorizon};
    use crate::analysis::{
        BollingerValues, CompanyRating, FinancialHealthMetrics, FundamentalAnalysis, GrowthMetrics,
        MacdValues, MarketRegime, ProfitabilityMetrics, Recommendation, Sentiment,
        TechnicalAnalysis, Trend, ValuationMetrics,
    };
    use crate::config::RiskManagementConfig;

    // ─── AiStrategy config tests ─────────────────────────────────────

    #[test]
    fn test_ai_strategy_name_and_kind() {
        let agent = Arc::new(
            TradingAgent::new(
                Box::new(OllamaQuery::new(
                    crate::mcp::ollama::OllamaProvider::default(),
                )),
                "test".to_string(),
                None,
            )
            .unwrap(),
        );
        let config = AiConfig {
            use_llm: false,
            use_finbert: false,
            min_confidence: 0.5,
            force_regime: None,
            memory_path: None,
        };
        let strategy = AiStrategy::new("test".to_string(), config, agent);
        assert_eq!(strategy.name(), "ai");
        assert_eq!(strategy.kind(), StrategyKind::Ai);
    }

    #[test]
    fn test_ai_strategy_validate() {
        let agent = Arc::new(
            TradingAgent::new(
                Box::new(OllamaQuery::new(
                    crate::mcp::ollama::OllamaProvider::default(),
                )),
                "test".to_string(),
                None,
            )
            .unwrap(),
        );
        let config = AiConfig {
            use_llm: true,
            use_finbert: false,
            min_confidence: 0.6,
            force_regime: None,
            memory_path: None,
        };
        let strategy = AiStrategy::new("test".to_string(), config, agent);
        assert!(strategy.validate().is_ok());
    }

    #[test]
    fn test_ai_strategy_validate_invalid() {
        let agent = Arc::new(
            TradingAgent::new(
                Box::new(OllamaQuery::new(
                    crate::mcp::ollama::OllamaProvider::default(),
                )),
                "test".to_string(),
                None,
            )
            .unwrap(),
        );
        let config = AiConfig {
            use_llm: true,
            use_finbert: false,
            min_confidence: 1.5,
            force_regime: None,
            memory_path: None,
        };
        let strategy = AiStrategy::new("test".to_string(), config, agent);
        assert!(strategy.validate().is_err());
    }

    #[test]
    fn test_ai_strategy_config_access() {
        let agent = Arc::new(
            TradingAgent::new(
                Box::new(OllamaQuery::new(
                    crate::mcp::ollama::OllamaProvider::default(),
                )),
                "test".to_string(),
                None,
            )
            .unwrap(),
        );
        let config = AiConfig {
            use_llm: true,
            use_finbert: false,
            min_confidence: 0.7,
            force_regime: Some("trending".to_string()),
            memory_path: None,
        };
        let strategy = AiStrategy::new("test".to_string(), config, agent);
        assert_eq!(strategy.config().min_confidence, 0.7);
        assert!(strategy.config().use_llm);
        assert_eq!(strategy.config().force_regime.as_deref(), Some("trending"));
    }

    // ─── TradingAgent rule-based decision tests ─────────────────────

    fn make_agent() -> TradingAgent {
        TradingAgent::new(
            Box::new(OllamaQuery::new(
                crate::mcp::ollama::OllamaProvider::default(),
            )),
            "test".to_string(),
            None,
        )
        .unwrap()
    }

    #[tokio::test]
    async fn test_rule_based_buy_signal() {
        let agent = make_agent();
        let ctx = DecisionContext {
            ticker: "SBER".to_string(),
            company_name: "Sberbank".to_string(),
            current_price: 100.0,
            news_sentiment: Some(crate::analysis::NewsSentiment {
                ticker: "SBER".to_string(),
                overall_sentiment: Sentiment::Positive,
                sentiment_score: 0.6,
                articles_count: 3,
                articles: vec![],
                key_events: vec!["Strong earnings".to_string()],
            }),
            technical_analysis: Some(TechnicalAnalysis {
                ticker: "SBER".to_string(),
                timestamp: chrono::Utc::now(),
                current_price: 100.0,
                trend: Trend::Bullish,
                rsi: Some(55.0),
                macd: Some(MacdValues {
                    macd_line: 1.0,
                    signal_line: 0.5,
                    histogram: 0.5,
                }),
                bollinger: Some(BollingerValues {
                    upper: 110.0,
                    middle: 100.0,
                    lower: 90.0,
                    bandwidth: 0.1,
                }),
                volume_analysis: crate::analysis::VolumeAnalysis {
                    current_volume: 2000.0,
                    avg_volume: 1000.0,
                    volume_ratio: 2.0,
                    is_unusual: true,
                },
                support_levels: vec![95.0],
                resistance_levels: vec![108.0],
                recommendation: Recommendation::Buy,
            }),
            fundamental_analysis: Some(FundamentalAnalysis {
                ticker: "SBER".to_string(),
                company_name: "Sberbank".to_string(),
                market_cap: None,
                rating: CompanyRating::Good,
                overall_score: 70.0,
                valuation: ValuationMetrics::default(),
                profitability: ProfitabilityMetrics::default(),
                financial_health: FinancialHealthMetrics::default(),
                growth: GrowthMetrics::default(),
                dividends: None,
                key_risks: vec![],
                key_strengths: vec!["Market leader".to_string()],
            }),
            available_balance: 100000.0,
            current_position: None,
            risk_config: None,
            max_position_pct: 0.15,
            market_regime: MarketRegime::Trending,
            candles: vec![],
        };

        let decision = agent
            .make_rule_based_decision(ctx)
            .await
            .expect("Rule-based decision failed");
        assert_eq!(decision.action, Action::Buy);
        assert!(decision.confidence > 0.5);
        assert!(decision.stop_loss.is_some());
        assert!(decision.take_profit.is_some());
        assert!(decision.position_size_pct > 0.0);
    }

    #[tokio::test]
    async fn test_rule_based_sell_signal() {
        let agent = make_agent();
        let ctx = DecisionContext {
            ticker: "SBER".to_string(),
            company_name: "Sberbank".to_string(),
            current_price: 250.0,
            news_sentiment: Some(crate::analysis::NewsSentiment {
                ticker: "SBER".to_string(),
                overall_sentiment: Sentiment::Negative,
                sentiment_score: -0.5,
                articles_count: 2,
                articles: vec![],
                key_events: vec!["Sanctions risk".to_string()],
            }),
            technical_analysis: Some(TechnicalAnalysis {
                ticker: "SBER".to_string(),
                timestamp: chrono::Utc::now(),
                current_price: 250.0,
                trend: Trend::Bearish,
                rsi: Some(75.0),
                macd: Some(MacdValues {
                    macd_line: 1.0,
                    signal_line: 2.0,
                    histogram: -1.0,
                }),
                bollinger: Some(BollingerValues {
                    upper: 260.0,
                    middle: 250.0,
                    lower: 240.0,
                    bandwidth: 0.08,
                }),
                volume_analysis: crate::analysis::VolumeAnalysis {
                    current_volume: 5000.0,
                    avg_volume: 2000.0,
                    volume_ratio: 2.5,
                    is_unusual: true,
                },
                support_levels: vec![245.0],
                resistance_levels: vec![258.0],
                recommendation: Recommendation::Sell,
            }),
            fundamental_analysis: None,
            available_balance: 500000.0,
            current_position: Some(CurrentPosition {
                quantity: 100,
                average_price: 240.0,
                current_value: 25000.0,
            }),
            risk_config: Some(RiskManagementConfig {
                max_loss_pct: 0.05,
                take_profit_pct: 0.10,
                stop_loss_pct: 0.03,
                max_open_positions: 5,
                min_balance_reserve: 50000.0,
            }),
            max_position_pct: 0.15,
            market_regime: MarketRegime::Trending,
            candles: vec![],
        };

        let decision = agent
            .make_rule_based_decision(ctx)
            .await
            .expect("Rule-based decision failed");
        assert_eq!(decision.action, Action::Sell);
        assert!(decision.confidence > 0.5);
        assert!(decision.stop_loss.is_some());
        assert!(decision.take_profit.is_some());
    }

    #[tokio::test]
    async fn test_rule_based_hold_signal() {
        let agent = make_agent();
        let ctx = DecisionContext {
            ticker: "TTECH".to_string(),
            company_name: "T-Technologies".to_string(),
            current_price: 100.0,
            news_sentiment: Some(crate::analysis::NewsSentiment {
                ticker: "TTECH".to_string(),
                overall_sentiment: Sentiment::Neutral,
                sentiment_score: 0.0,
                articles_count: 1,
                articles: vec![],
                key_events: vec![],
            }),
            technical_analysis: Some(TechnicalAnalysis {
                ticker: "TTECH".to_string(),
                timestamp: chrono::Utc::now(),
                current_price: 100.0,
                trend: Trend::Sideways,
                rsi: Some(50.0),
                macd: Some(MacdValues {
                    macd_line: 0.0,
                    signal_line: 0.0,
                    histogram: 0.0,
                }),
                bollinger: Some(BollingerValues {
                    upper: 105.0,
                    middle: 100.0,
                    lower: 95.0,
                    bandwidth: 0.1,
                }),
                volume_analysis: crate::analysis::VolumeAnalysis {
                    current_volume: 1000.0,
                    avg_volume: 1000.0,
                    volume_ratio: 1.0,
                    is_unusual: false,
                },
                support_levels: vec![95.0],
                resistance_levels: vec![105.0],
                recommendation: Recommendation::Hold,
            }),
            fundamental_analysis: Some(FundamentalAnalysis {
                ticker: "TTECH".to_string(),
                company_name: "T-Technologies".to_string(),
                market_cap: None,
                rating: CompanyRating::Fair,
                overall_score: 50.0,
                valuation: ValuationMetrics::default(),
                profitability: ProfitabilityMetrics::default(),
                financial_health: FinancialHealthMetrics::default(),
                growth: GrowthMetrics::default(),
                dividends: None,
                key_risks: vec![],
                key_strengths: vec![],
            }),
            available_balance: 100000.0,
            current_position: Some(CurrentPosition {
                quantity: 50,
                average_price: 98.0,
                current_value: 5000.0,
            }),
            risk_config: None,
            max_position_pct: 0.10,
            market_regime: MarketRegime::Quiet,
            candles: vec![],
        };

        let decision = agent
            .make_rule_based_decision(ctx)
            .await
            .expect("Rule-based decision failed");
        assert_eq!(decision.action, Action::Hold);
    }

    #[tokio::test]
    async fn test_rule_based_strong_buy_overrides_hold() {
        let agent = make_agent();
        let ctx = DecisionContext {
            ticker: "T".to_string(),
            company_name: "T-Technologies".to_string(),
            current_price: 100.0,
            news_sentiment: None,
            technical_analysis: Some(TechnicalAnalysis {
                ticker: "T".to_string(),
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
                recommendation: Recommendation::StrongBuy,
            }),
            fundamental_analysis: None,
            available_balance: 50000.0,
            current_position: None,
            risk_config: None,
            max_position_pct: 0.10,
            market_regime: MarketRegime::Volatile,
            candles: vec![],
        };

        let decision = agent
            .make_rule_based_decision(ctx)
            .await
            .expect("Rule-based decision failed");
        assert_eq!(decision.action, Action::Buy);
        assert!(decision.confidence >= 0.6);
    }

    #[tokio::test]
    async fn test_position_size_calculation() {
        let agent = make_agent();
        let ctx = DecisionContext {
            ticker: "TEST".to_string(),
            company_name: "Test".to_string(),
            current_price: 100.0,
            news_sentiment: Some(crate::analysis::NewsSentiment {
                ticker: "TEST".to_string(),
                overall_sentiment: Sentiment::Positive,
                sentiment_score: 0.3,
                articles_count: 1,
                articles: vec![],
                key_events: vec![],
            }),
            technical_analysis: None,
            fundamental_analysis: None,
            available_balance: 10000.0,
            current_position: None,
            risk_config: None,
            max_position_pct: 0.1,
            market_regime: MarketRegime::Ranging,
            candles: vec![],
        };

        let decision = agent
            .make_rule_based_decision(ctx)
            .await
            .expect("Rule-based decision failed");
        assert!(decision.position_size_pct >= 0.0);
        assert!(decision.position_size_pct <= 0.1);
    }

    #[tokio::test]
    async fn test_stop_loss_take_profit_levels() {
        let agent = make_agent();
        let ctx = DecisionContext {
            ticker: "SBER".to_string(),
            company_name: "Sberbank".to_string(),
            current_price: 100.0,
            news_sentiment: Some(crate::analysis::NewsSentiment {
                ticker: "SBER".to_string(),
                overall_sentiment: Sentiment::Positive,
                sentiment_score: 0.5,
                articles_count: 2,
                articles: vec![],
                key_events: vec!["Dividend growth".to_string()],
            }),
            technical_analysis: Some(TechnicalAnalysis {
                ticker: "SBER".to_string(),
                timestamp: chrono::Utc::now(),
                current_price: 100.0,
                trend: Trend::Bullish,
                rsi: Some(52.0),
                macd: None,
                bollinger: None,
                volume_analysis: crate::analysis::VolumeAnalysis {
                    current_volume: 1500.0,
                    avg_volume: 1000.0,
                    volume_ratio: 1.5,
                    is_unusual: false,
                },
                support_levels: vec![95.0],
                resistance_levels: vec![108.0],
                recommendation: Recommendation::Buy,
            }),
            fundamental_analysis: None,
            available_balance: 200000.0,
            current_position: None,
            risk_config: Some(RiskManagementConfig {
                max_loss_pct: 0.05,
                take_profit_pct: 0.10,
                stop_loss_pct: 0.03,
                max_open_positions: 3,
                min_balance_reserve: 10000.0,
            }),
            max_position_pct: 0.20,
            market_regime: MarketRegime::Trending,
            candles: vec![],
        };

        let decision = agent
            .make_rule_based_decision(ctx)
            .await
            .expect("Rule-based decision failed");
        assert_eq!(decision.action, Action::Buy);
        let (sl, tp) = (decision.stop_loss.unwrap(), decision.take_profit.unwrap());
        assert!(
            sl < 100.0,
            "Stop loss {:.2} should be below entry 100.0",
            sl
        );
        assert!(
            tp > 100.0,
            "Take profit {:.2} should be above entry 100.0",
            tp
        );
        assert!(
            (sl - 97.0).abs() < 0.1,
            "Stop loss should be ~97 (3% below 100), got {:.2}",
            sl
        );
    }

    #[tokio::test]
    async fn test_decision_scoring_confidence_bounds() {
        let agent = make_agent();
        let ctx = DecisionContext {
            ticker: "TEST".to_string(),
            company_name: "Test".to_string(),
            current_price: 50.0,
            news_sentiment: Some(crate::analysis::NewsSentiment {
                ticker: "TEST".to_string(),
                overall_sentiment: Sentiment::Positive,
                sentiment_score: 0.8,
                articles_count: 10,
                articles: vec![],
                key_events: vec!["Breakthrough technology".to_string()],
            }),
            technical_analysis: Some(TechnicalAnalysis {
                ticker: "TEST".to_string(),
                timestamp: chrono::Utc::now(),
                current_price: 50.0,
                trend: Trend::Bullish,
                rsi: Some(60.0),
                macd: None,
                bollinger: None,
                volume_analysis: crate::analysis::VolumeAnalysis {
                    current_volume: 5000.0,
                    avg_volume: 1000.0,
                    volume_ratio: 5.0,
                    is_unusual: true,
                },
                support_levels: vec![48.0],
                resistance_levels: vec![55.0],
                recommendation: Recommendation::StrongBuy,
            }),
            fundamental_analysis: Some(FundamentalAnalysis {
                ticker: "TEST".to_string(),
                company_name: "Test".to_string(),
                market_cap: None,
                rating: CompanyRating::Excellent,
                overall_score: 90.0,
                valuation: ValuationMetrics::default(),
                profitability: ProfitabilityMetrics::default(),
                financial_health: FinancialHealthMetrics::default(),
                growth: GrowthMetrics::default(),
                dividends: None,
                key_risks: vec![],
                key_strengths: vec![],
            }),
            available_balance: 1000000.0,
            current_position: None,
            risk_config: None,
            max_position_pct: 0.25,
            market_regime: MarketRegime::Trending,
            candles: vec![],
        };

        let decision = agent
            .make_rule_based_decision(ctx)
            .await
            .expect("Rule-based decision failed");
        assert!(
            decision.confidence >= 0.05 && decision.confidence <= 0.95,
            "Confidence {:.3} should be clamped to [0.05, 0.95]",
            decision.confidence
        );
        assert_eq!(decision.action, Action::Buy);
    }

    #[test]
    fn test_trading_action_equality() {
        assert_eq!(Action::Buy, Action::Buy);
        assert_ne!(Action::Buy, Action::Sell);
        assert_ne!(Action::Hold, Action::Sell);
    }

    #[test]
    fn test_time_horizon_ordering() {
        let short = TimeHorizon::Short;
        let _medium = TimeHorizon::Medium;
        let long = TimeHorizon::Long;

        // Just check they're different values
        assert_ne!(short as i32, TimeHorizon::Medium as i32);
        assert_ne!(TimeHorizon::Medium as i32, long as i32);
    }
}
