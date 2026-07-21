use crate::agent::Action;
use crate::provider::prediction::{Prediction, PredictionContext};
use anyhow::Result;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct TechnicalPredictor {
    rsi_weight: f64,
    macd_weight: f64,
    bb_weight: f64,
    volume_weight: f64,
    trend_weight: f64,
}

impl TechnicalPredictor {
    pub fn new() -> Self {
        TechnicalPredictor {
            rsi_weight: 0.20,
            macd_weight: 0.25,
            bb_weight: 0.15,
            volume_weight: 0.15,
            trend_weight: 0.25,
        }
    }

    pub async fn predict(&self, ctx: &PredictionContext) -> Result<Prediction> {
        let mut action = Action::Hold;
        let mut conviction = 0.0;
        let mut features = HashMap::new();
        let mut rationale_parts = Vec::new();

        if let Some(ref tech) = ctx.technical {
            let rsi_signal = tech
                .rsi
                .map(|r| {
                    if r < 30.0 {
                        0.6
                    } else if r < 40.0 {
                        0.3
                    } else if r > 70.0 {
                        -0.6
                    } else if r > 60.0 {
                        -0.3
                    } else {
                        0.0
                    }
                })
                .unwrap_or(0.0);
            features.insert("rsi_signal".into(), rsi_signal);

            let macd_signal = tech
                .macd
                .as_ref()
                .map(|m| {
                    if m.histogram > 0.0 && m.macd_line > m.signal_line {
                        0.5
                    } else if m.histogram < 0.0 && m.macd_line < m.signal_line {
                        -0.5
                    } else {
                        0.0
                    }
                })
                .unwrap_or(0.0);
            features.insert("macd_signal".into(), macd_signal);

            let bb_signal = if let Some(ref bb) = tech.bollinger {
                let p = ctx.current_price;
                if p <= bb.lower {
                    0.4
                } else if p >= bb.upper {
                    -0.4
                } else {
                    0.0
                }
            } else {
                0.0
            };
            features.insert("bb_signal".into(), bb_signal);

            let trend_signal = match tech.trend {
                crate::analysis::Trend::Bullish => 0.4,
                crate::analysis::Trend::Bearish => -0.4,
                crate::analysis::Trend::Sideways => 0.0,
            };
            features.insert("trend_signal".into(), trend_signal);

            let volume_signal = if tech.volume_analysis.is_unusual {
                if tech.volume_analysis.volume_ratio > 1.5 {
                    0.3
                } else {
                    -0.1
                }
            } else {
                0.0
            };
            features.insert("volume_signal".into(), volume_signal);

            conviction = self.rsi_weight * rsi_signal
                + self.macd_weight * macd_signal
                + self.bb_weight * bb_signal
                + self.trend_weight * trend_signal
                + self.volume_weight * volume_signal;

            if let Some(rsi_val) = tech.rsi {
                rationale_parts.push(format!("RSI={:.1}", rsi_val));
            }
            if let Some(ref m) = tech.macd {
                rationale_parts.push(format!("MACD hist={:.4}", m.histogram));
            }
            if let Some(ref bb) = tech.bollinger {
                rationale_parts.push(format!("BB spread={:.2}", bb.bandwidth));
            }
            rationale_parts.push(format!("trend={:?}", tech.trend));
        }

        action = if conviction > 0.15 {
            Action::Buy
        } else if conviction < -0.15 {
            Action::Sell
        } else {
            Action::Hold
        };

        Ok(Prediction {
            action,
            confidence: conviction.abs().min(0.9).max(0.1),
            conviction,
            features,
            metadata: serde_json::json!({"method": "technical_indicator_weighted"}),
            provider: "technical".to_string(),
            rationale: rationale_parts.join(", "),
        })
    }
}
