use crate::agent::DecisionMemory;
use crate::agent::PredictionTracker;
use crate::agent::{Action, DecisionContext};
use crate::analysis::regime::MarketRegime;
use crate::provider::prediction::EnsemblePredictor;
use crate::provider::prediction::fundamental::FundamentalPredictor;
use crate::provider::prediction::llm::LLMPredictor;
use crate::provider::prediction::stat_arb::StatArbPredictor;
use crate::provider::prediction::technical::TechnicalPredictor;
use crate::provider::prediction::{Prediction, PredictionContext};
use anyhow::Result;

#[derive(Debug, Clone)]
pub struct AnalystProposal {
    pub action: Action,
    pub confidence: f64,
    pub conviction: f64,
    pub rationale: String,
    pub provider: String,
    pub prediction: Prediction,
}

#[derive(Debug, Clone)]
pub struct RiskAssessment {
    pub allowed: bool,
    pub max_position_pct: f64,
    pub stop_loss_pct: f64,
    pub take_profit_pct: f64,
    pub risk_score: f64,
    pub rationale: String,
}

#[derive(Debug, Clone)]
pub struct SupervisorDecision {
    pub action: Action,
    pub confidence: f64,
    pub conviction: f64,
    pub proposal: AnalystProposal,
    pub risk: RiskAssessment,
    pub final_rationale: String,
}

pub struct AnalystAgent {
    ensemble: EnsemblePredictor,
    memory: DecisionMemory,
}

impl AnalystAgent {
    pub fn new(llm_provider: mcp_client::ollama::OllamaProvider, model_name: &str) -> Self {
        let tracker = PredictionTracker::new();
        let providers = vec![
            crate::provider::prediction::PredictionProviderKind::Technical(
                TechnicalPredictor::new(),
            ),
            crate::provider::prediction::PredictionProviderKind::LLM(LLMPredictor::new(
                llm_provider,
                model_name,
            )),
            crate::provider::prediction::PredictionProviderKind::StatArb(StatArbPredictor::new(
                2.0, 0.5, 20,
            )),
            crate::provider::prediction::PredictionProviderKind::Fundamental(
                FundamentalPredictor::new(),
            ),
        ];
        AnalystAgent {
            ensemble: EnsemblePredictor::new(providers, tracker),
            memory: DecisionMemory::new(500),
        }
    }

    pub async fn analyze(&self, ctx: &DecisionContext) -> Result<AnalystProposal> {
        let mut pred_ctx = PredictionContext::new(&ctx.ticker);
        pred_ctx.company_name = ctx.company_name.clone();
        pred_ctx.current_price = ctx.current_price;
        pred_ctx.available_balance = ctx.available_balance;
        pred_ctx.max_position_pct = ctx.max_position_pct;
        pred_ctx.regime = ctx.market_regime;
        pred_ctx.candles = ctx.candles.clone();
        pred_ctx.technical = ctx.technical_analysis.clone();
        pred_ctx.news = ctx.news_sentiment.clone();
        pred_ctx.fundamental = ctx.fundamental_analysis.clone();

        let prediction = self.ensemble.predict(&pred_ctx).await?;

        Ok(AnalystProposal {
            action: prediction.action.clone(),
            confidence: prediction.confidence,
            conviction: prediction.conviction,
            rationale: prediction.rationale.clone(),
            provider: "ensemble".to_string(),
            prediction,
        })
    }

    pub fn memory(&self) -> &DecisionMemory {
        &self.memory
    }

    pub fn memory_mut(&mut self) -> &mut DecisionMemory {
        &mut self.memory
    }
}

pub struct RiskAgent {
    max_position_pct: f64,
    max_drawdown_pct: f64,
    var_confidence: f64,
}

impl RiskAgent {
    pub fn new(max_position_pct: f64, max_drawdown_pct: f64) -> Self {
        RiskAgent {
            max_position_pct,
            max_drawdown_pct,
            var_confidence: 0.95,
        }
    }

    pub fn assess(&self, proposal: &AnalystProposal, ctx: &DecisionContext) -> RiskAssessment {
        let mut risk_score = 0.0;
        let mut reasons = Vec::new();

        if proposal.confidence < 0.3 {
            risk_score += 0.3;
            reasons.push("low confidence");
        }

        match ctx.market_regime {
            MarketRegime::Volatile => {
                risk_score += 0.2;
                reasons.push("volatile regime");
            }
            MarketRegime::Trending => {
                if proposal.conviction.abs() > 0.5 {
                    risk_score -= 0.1;
                    reasons.push("strong trend conviction");
                }
            }
            _ => {}
        }

        let price_volatility = ctx
            .technical_analysis
            .as_ref()
            .and_then(|t| t.rsi)
            .map(|rsi| if rsi > 70.0 || rsi < 30.0 { 0.15 } else { 0.0 })
            .unwrap_or(0.0);
        risk_score += price_volatility;

        let win_rate = proposal
            .prediction
            .metadata
            .get("total_weight")
            .and_then(|v: &serde_json::Value| v.as_f64())
            .map(|w: f64| if w < 0.3 { 0.1 } else { 0.0 })
            .unwrap_or(0.0);
        risk_score += win_rate;

        let allowed = risk_score < 0.6;
        let max_pos_pct = if allowed {
            self.max_position_pct * (1.0 - risk_score)
        } else {
            0.0
        };
        let stop_loss_pct = match proposal.action {
            Action::Buy | Action::Sell => 2.0 + risk_score * 3.0,
            Action::Hold => 0.0,
        };

        RiskAssessment {
            allowed,
            max_position_pct: max_pos_pct,
            stop_loss_pct,
            take_profit_pct: stop_loss_pct * 1.5,
            risk_score,
            rationale: reasons.join(", "),
        }
    }
}

pub struct SupervisorAgent {
    analyst: AnalystAgent,
    risk: RiskAgent,
}

impl SupervisorAgent {
    pub fn new(
        llm_provider: mcp_client::ollama::OllamaProvider,
        model_name: &str,
        max_position_pct: f64,
        max_drawdown_pct: f64,
    ) -> Self {
        SupervisorAgent {
            analyst: AnalystAgent::new(llm_provider, model_name),
            risk: RiskAgent::new(max_position_pct, max_drawdown_pct),
        }
    }

    pub async fn decide(&self, ctx: &DecisionContext) -> Result<SupervisorDecision> {
        let proposal = self.analyst.analyze(ctx).await?;
        let risk_assessment = self.risk.assess(&proposal, ctx);

        let (final_action, final_rationale) = if !risk_assessment.allowed {
            (
                Action::Hold,
                format!("Risk blocked: {}", risk_assessment.rationale),
            )
        } else if proposal.action == Action::Hold {
            (Action::Hold, "Analyst recommends hold".to_string())
        } else {
            (proposal.action.clone(), proposal.rationale.clone())
        };

        Ok(SupervisorDecision {
            action: final_action,
            confidence: if risk_assessment.allowed {
                proposal.confidence
            } else {
                0.0
            },
            conviction: if risk_assessment.allowed {
                proposal.conviction
            } else {
                0.0
            },
            proposal,
            risk: risk_assessment,
            final_rationale,
        })
    }

    pub fn analyst(&self) -> &AnalystAgent {
        &self.analyst
    }

    pub fn analyst_mut(&mut self) -> &mut AnalystAgent {
        &mut self.analyst
    }

    pub fn risk_agent(&self) -> &RiskAgent {
        &self.risk
    }
}
