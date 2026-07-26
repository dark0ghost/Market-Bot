use crate::agent::Action;
use crate::analysis::CompanyRating;
use crate::provider::prediction::{Prediction, PredictionContext};
use anyhow::Result;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct FundamentalPredictor {
    rating_weight: f64,
    pe_weight: f64,
    roe_weight: f64,
    growth_weight: f64,
}

impl Default for FundamentalPredictor {
    fn default() -> Self {
        Self::new()
    }
}

impl FundamentalPredictor {
    pub fn new() -> Self {
        FundamentalPredictor {
            rating_weight: 0.35,
            pe_weight: 0.20,
            roe_weight: 0.25,
            growth_weight: 0.20,
        }
    }

    pub async fn predict(&self, ctx: &PredictionContext) -> Result<Prediction> {
        let mut conviction = 0.0;
        let mut features = HashMap::new();
        let mut rationale_parts = Vec::new();

        if let Some(ref fund) = ctx.fundamental {
            let rating_signal = match fund.rating {
                CompanyRating::Excellent => 0.6,
                CompanyRating::Good => 0.3,
                CompanyRating::Fair => 0.0,
                CompanyRating::Poor => -0.3,
                CompanyRating::VeryPoor => -0.6,
            };
            features.insert("rating_signal".into(), rating_signal);
            rationale_parts.push(format!("rating={:?}", fund.rating));

            let pe_signal = fund
                .valuation
                .pe_ratio
                .map(|pe| {
                    if pe < 0.0 {
                        0.0
                    } else if pe < 10.0 {
                        0.3
                    } else if pe < 20.0 {
                        0.1
                    } else if pe < 30.0 {
                        -0.1
                    } else {
                        -0.3
                    }
                })
                .unwrap_or(0.0);
            features.insert("pe_signal".into(), pe_signal);

            let roe_signal = fund
                .profitability
                .roe
                .map(|roe| {
                    if roe > 20.0 {
                        0.4
                    } else if roe > 10.0 {
                        0.2
                    } else if roe > 5.0 {
                        0.0
                    } else {
                        -0.2
                    }
                })
                .unwrap_or(0.0);
            features.insert("roe_signal".into(), roe_signal);

            let growth_signal = fund
                .growth
                .revenue_growth_yoy
                .map(|g| {
                    if g > 20.0 {
                        0.4
                    } else if g > 10.0 {
                        0.2
                    } else if g > 0.0 {
                        0.0
                    } else {
                        -0.3
                    }
                })
                .unwrap_or(0.0);
            features.insert("growth_signal".into(), growth_signal);

            conviction = self.rating_weight * rating_signal
                + self.pe_weight * pe_signal
                + self.roe_weight * roe_signal
                + self.growth_weight * growth_signal;
        }

        let action = if conviction > 0.15 {
            Action::Buy
        } else if conviction < -0.15 {
            Action::Sell
        } else {
            Action::Hold
        };

        Ok(Prediction {
            action,
            confidence: conviction.abs().clamp(0.1, 0.9),
            conviction,
            features,
            metadata: serde_json::json!({"method": "fundamental_weighted"}),
            provider: "fundamental".to_string(),
            rationale: rationale_parts.join(", "),
        })
    }
}
