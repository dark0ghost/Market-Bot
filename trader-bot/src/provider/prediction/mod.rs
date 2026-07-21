pub mod technical;
pub mod llm;
pub mod stat_arb;
pub mod fundamental;

use anyhow::Result;
use std::collections::HashMap;
use crate::agent::Action;
use crate::agent::calibration::PredictionTracker;
use crate::analysis::regime::MarketRegime;

#[derive(Debug, Clone)]
pub struct Prediction {
    pub action: Action,
    pub confidence: f64,
    pub conviction: f64,
    pub features: HashMap<String, f64>,
    pub metadata: serde_json::Value,
    pub provider: String,
    pub rationale: String,
}

#[derive(Debug, Clone)]
pub struct PredictionContext {
    pub ticker: String,
    pub company_name: String,
    pub current_price: f64,
    pub available_balance: f64,
    pub max_position_pct: f64,
    pub regime: MarketRegime,
    pub candles: Vec<crate::client::order_book::OrderBookLevel>,
    pub technical: Option<crate::analysis::TechnicalAnalysis>,
    pub news: Option<crate::analysis::NewsSentiment>,
    pub fundamental: Option<crate::analysis::FundamentalAnalysis>,
}

impl PredictionContext {
    pub fn new(ticker: &str) -> Self {
        PredictionContext {
            ticker: ticker.to_string(),
            company_name: String::new(),
            current_price: 0.0,
            available_balance: 0.0,
            max_position_pct: 0.0,
            regime: MarketRegime::Quiet,
            candles: vec![],
            technical: None,
            news: None,
            fundamental: None,
        }
    }
}

#[derive(Debug, Clone)]
pub enum PredictionProviderKind {
    Technical(technical::TechnicalPredictor),
    LLM(llm::LLMPredictor),
    StatArb(stat_arb::StatArbPredictor),
    Fundamental(fundamental::FundamentalPredictor),
}

impl PredictionProviderKind {
    pub fn provider_id(&self) -> &str {
        match self {
            PredictionProviderKind::Technical(_) => "technical",
            PredictionProviderKind::LLM(_) => "llm",
            PredictionProviderKind::StatArb(_) => "stat_arb",
            PredictionProviderKind::Fundamental(_) => "fundamental",
        }
    }

    pub fn ensemble_weight(&self) -> f64 {
        match self {
            PredictionProviderKind::Technical(_) => 0.30,
            PredictionProviderKind::LLM(_) => 0.35,
            PredictionProviderKind::StatArb(_) => 0.15,
            PredictionProviderKind::Fundamental(_) => 0.20,
        }
    }

    pub async fn predict(&self, ctx: &PredictionContext) -> Result<Prediction> {
        match self {
            PredictionProviderKind::Technical(p) => p.predict(ctx).await,
            PredictionProviderKind::LLM(p) => p.predict(ctx).await,
            PredictionProviderKind::StatArb(p) => p.predict(ctx).await,
            PredictionProviderKind::Fundamental(p) => p.predict(ctx).await,
        }
    }
}

pub struct EnsemblePredictor {
    providers: Vec<PredictionProviderKind>,
    tracker: PredictionTracker,
    regime_weights: HashMap<MarketRegime, Vec<f64>>,
}

impl EnsemblePredictor {
    pub fn new(providers: Vec<PredictionProviderKind>, tracker: PredictionTracker) -> Self {
        let mut ep = EnsemblePredictor {
            providers,
            tracker,
            regime_weights: HashMap::new(),
        };
        ep.init_regime_weights();
        ep
    }

    fn init_regime_weights(&mut self) {
        use MarketRegime::*;
        for regime in [Trending, Ranging, Volatile, Quiet] {
            let n = self.providers.len();
            let w: Vec<f64> = match regime {
                Trending => vec![0.35, 0.30, 0.10, 0.25],
                Ranging => vec![0.20, 0.25, 0.35, 0.20],
                Volatile => vec![0.15, 0.45, 0.25, 0.15],
                Quiet => vec![0.30, 0.25, 0.20, 0.25],
            };
            let w = if w.len() == n { w } else { vec![1.0 / n as f64; n] };
            self.regime_weights.insert(regime, w);
        }
    }

    fn weights_for_regime(&self, regime: &MarketRegime) -> Vec<f64> {
        self.regime_weights
            .get(regime)
            .cloned()
            .unwrap_or_else(|| vec![1.0 / self.providers.len() as f64; self.providers.len()])
    }

    pub async fn predict(&self, ctx: &PredictionContext) -> Result<Prediction> {
        let regime = &ctx.regime;
        let base_weights = self.weights_for_regime(regime);
        let mut weighted_predictions: Vec<(f64, f64, String, String)> = Vec::new();

        for (i, provider) in self.providers.iter().enumerate() {
            let pred = provider.predict(ctx).await?;
            let accuracy = self.tracker.provider_accuracy(provider.provider_id());
            let weight = base_weights[i] * (0.5 + 0.5 * accuracy);
            weighted_predictions.push((
                weight,
                pred.confidence * pred.conviction.signum(),
                pred.provider,
                pred.rationale,
            ));
        }

        let total_weight: f64 = weighted_predictions.iter().map(|(w, _, _, _)| w).sum();
        let total_weight = if total_weight == 0.0 { 1.0 } else { total_weight };

        let mut buy_score = 0.0;
        let mut sell_score = 0.0;
        let mut combined_rationale = Vec::new();
        let mut combined_features = HashMap::new();

        for (weight, signal, provider, rationale) in &weighted_predictions {
            if *signal > 0.0 {
                buy_score += weight * signal;
            } else {
                sell_score += weight * signal.abs();
            }
            let norm_weight = weight / total_weight;
            combined_features.insert(provider.clone(), *signal);
            combined_rationale.push(format!("[{}] {:.1}%: {}", provider, norm_weight * 100.0, rationale));
        }

        let action = if buy_score > sell_score && buy_score > 0.05 {
            Action::Buy
        } else if sell_score > buy_score && sell_score > 0.05 {
            Action::Sell
        } else {
            Action::Hold
        };

        let confidence = match action {
            Action::Buy => buy_score.min(0.95).max(0.05),
            Action::Sell => sell_score.min(0.95).max(0.05),
            Action::Hold => 0.5,
        };

        Ok(Prediction {
            action,
            confidence,
            conviction: (buy_score - sell_score).abs(),
            features: combined_features,
            metadata: serde_json::json!({
                "buy_score": buy_score,
                "sell_score": sell_score,
                "total_weight": total_weight,
                "regime": format!("{:?}", regime),
            }),
            provider: "ensemble".to_string(),
            rationale: combined_rationale.join("; "),
        })
    }

    pub fn tracker(&self) -> &PredictionTracker {
        &self.tracker
    }

    pub fn tracker_mut(&mut self) -> &mut PredictionTracker {
        &mut self.tracker
    }
}
