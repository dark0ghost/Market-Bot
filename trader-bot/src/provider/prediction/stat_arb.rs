use crate::agent::Action;
use crate::provider::prediction::{Prediction, PredictionContext};
use anyhow::Result;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct StatArbPredictor {
    z_entry: f64,
    lookback: usize,
}

impl StatArbPredictor {
    pub fn new(z_entry: f64, lookback: usize) -> Self {
        StatArbPredictor { z_entry, lookback }
    }

    pub async fn predict(&self, ctx: &PredictionContext) -> Result<Prediction> {
        let closes: Vec<f64> = Vec::new();
        let mut features = HashMap::new();

        let (z_score, action, conviction, rationale) = if closes.len() < self.lookback {
            (0.0, Action::Hold, 0.0, "insufficient data".into())
        } else {
            let window: Vec<f64> = closes
                .iter()
                .rev()
                .take(self.lookback)
                .copied()
                .rev()
                .collect();
            let mean = window.iter().sum::<f64>() / window.len() as f64;
            let variance =
                window.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / window.len() as f64;
            let std = variance.sqrt();

            if std < 1e-10 {
                (0.0, Action::Hold, 0.0, "no variance".into())
            } else {
                let last = ctx.current_price;
                let z = (last - mean) / std;
                let (act, conf) = if z > self.z_entry {
                    (Action::Sell, (z / self.z_entry).min(1.0))
                } else if z < -self.z_entry {
                    (Action::Buy, (z.abs() / self.z_entry).min(1.0))
                } else {
                    (Action::Hold, 0.0)
                };
                (
                    z,
                    act,
                    conf,
                    format!("z={:.2}, mean={:.2}, std={:.2}", z, mean, std),
                )
            }
        };

        features.insert("z_score".into(), z_score);

        Ok(Prediction {
            action,
            confidence: conviction.abs().clamp(0.0, 0.9),
            conviction: -z_score.signum() * conviction,
            features,
            metadata: serde_json::json!({"z_entry": self.z_entry, "lookback": self.lookback}),
            provider: "stat_arb".to_string(),
            rationale,
        })
    }
}
