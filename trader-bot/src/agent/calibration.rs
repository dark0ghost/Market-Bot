use crate::agent::Action;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct CalibrationBin {
    pub lower: f64,
    pub upper: f64,
    pub predictions: HashMap<Action, usize>,
    pub correct: usize,
    pub total: usize,
}

impl CalibrationBin {
    pub fn new(lower: f64, upper: f64) -> Self {
        CalibrationBin {
            lower,
            upper,
            predictions: HashMap::new(),
            correct: 0,
            total: 0,
        }
    }

    pub const fn accuracy(&self) -> f64 {
        if self.total == 0 {
            return 0.0;
        }
        self.correct as f64 / self.total as f64
    }

    pub const fn mean_confidence(&self) -> f64 {
        (self.lower + self.upper) / 2.0
    }

    pub fn record_prediction(
        &mut self,
        predicted_action: &Action,
        actual_correct: bool,
        _confidence: f64,
    ) {
        *self
            .predictions
            .entry(predicted_action.clone())
            .or_insert(0) += 1;
        self.total += 1;
        if actual_correct {
            self.correct += 1;
        }
    }
}

pub struct PredictionTracker {
    provider_results: HashMap<String, ProviderStats>,
    calibration_bins: Vec<CalibrationBin>,
    recent_predictions: Vec<PredictionRecord>,
    max_recent: usize,
}

#[derive(Debug, Clone)]
pub struct ProviderStats {
    pub total: usize,
    pub correct: usize,
    pub avg_confidence: f64,
}

#[derive(Debug, Clone)]
pub struct PredictionRecord {
    pub provider: String,
    pub confidence: f64,
    pub conviction: f64,
    pub action: Action,
    pub correct: Option<bool>,
    pub actual_pnl: Option<f64>,
}

impl PredictionTracker {
    pub fn new() -> Self {
        PredictionTracker {
            provider_results: HashMap::new(),
            calibration_bins: Self::default_bins(),
            recent_predictions: Vec::new(),
            max_recent: 1000,
        }
    }

    fn default_bins() -> Vec<CalibrationBin> {
        let mut bins = Vec::new();
        let mut lower: f64 = 0.0;
        while lower < 1.0 {
            let upper = (lower + 0.1).min(1.0);
            bins.push(CalibrationBin::new(lower, upper));
            lower = upper;
        }
        bins
    }

    pub fn record_outcome(
        &mut self,
        provider: &str,
        confidence: f64,
        conviction: f64,
        action: Action,
        correct: bool,
        pnl: Option<f64>,
    ) {
        let stats = self
            .provider_results
            .entry(provider.to_string())
            .or_insert(ProviderStats {
                total: 0,
                correct: 0,
                avg_confidence: 0.0,
            });
        stats.total += 1;
        if correct {
            stats.correct += 1;
        }
        stats.avg_confidence =
            stats.avg_confidence + (confidence - stats.avg_confidence) / stats.total as f64;

        let bin_idx =
            ((confidence * 10.0) as usize).min(self.calibration_bins.len().saturating_sub(1));
        if let Some(bin) = self.calibration_bins.get_mut(bin_idx) {
            bin.record_prediction(&action, correct, confidence);
        }

        self.recent_predictions.push(PredictionRecord {
            provider: provider.to_string(),
            confidence,
            conviction,
            action,
            correct: Some(correct),
            actual_pnl: pnl,
        });
        if self.recent_predictions.len() > self.max_recent {
            self.recent_predictions.remove(0);
        }
    }

    pub fn provider_accuracy(&self, provider: &str) -> f64 {
        self.provider_results
            .get(provider)
            .map(|s| {
                if s.total == 0 {
                    return 0.5;
                }
                s.correct as f64 / s.total as f64
            })
            .unwrap_or(0.5)
    }

    pub fn provider_stats(&self, provider: &str) -> Option<&ProviderStats> {
        self.provider_results.get(provider)
    }

    pub fn calibration_error(&self) -> f64 {
        if self.calibration_bins.is_empty() {
            return 0.0;
        }
        let mut total_error = 0.0;
        let mut total_weight = 0.0;
        for bin in &self.calibration_bins {
            if bin.total > 0 {
                let ece = (bin.accuracy() - bin.mean_confidence()).abs();
                total_error += ece * bin.total as f64;
                total_weight += bin.total as f64;
            }
        }
        if total_weight == 0.0 {
            0.0
        } else {
            total_error / total_weight
        }
    }

    pub fn ece(&self) -> f64 {
        self.calibration_error()
    }
}
