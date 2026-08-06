use std::collections::VecDeque;

#[derive(Debug, Clone)]
pub struct PairConfig {
    pub ticker_a: String,
    pub ticker_b: String,
    pub entry_z_score: f64,
    pub exit_z_score: f64,
    pub lookback_period: usize,
    pub stop_loss_z: f64,
}

#[derive(Debug, Clone)]
pub struct PairSignal {
    pub action: PairAction,
    pub z_score: f64,
    pub hedge_ratio: f64,
    pub confidence: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PairAction {
    LongShort,
    ShortLong,
    Neutral,
}

pub struct PairsTrader {
    config: PairConfig,
    prices_a: VecDeque<f64>,
    prices_b: VecDeque<f64>,
}

impl PairsTrader {
    pub fn new(config: PairConfig) -> Self {
        let lookback = config.lookback_period;
        PairsTrader {
            config,
            prices_a: VecDeque::with_capacity(lookback),
            prices_b: VecDeque::with_capacity(lookback),
        }
    }

    pub fn update(&mut self, price_a: f64, price_b: f64) {
        if self.prices_a.len() >= self.config.lookback_period {
            self.prices_a.pop_front();
            self.prices_b.pop_front();
        }
        self.prices_a.push_back(price_a);
        self.prices_b.push_back(price_b);
    }

    fn calculate_hedge_ratio(&self) -> f64 {
        if self.prices_a.len() < 2 {
            return 1.0;
        }

        let returns_a: Vec<f64> = self
            .prices_a
            .iter()
            .zip(self.prices_a.iter().skip(1))
            .map(|(a, b)| (b - a) / a)
            .collect();

        let returns_b: Vec<f64> = self
            .prices_b
            .iter()
            .zip(self.prices_b.iter().skip(1))
            .map(|(a, b)| (b - a) / a)
            .collect();

        let mean_a = returns_a.iter().sum::<f64>() / returns_a.len() as f64;
        let mean_b = returns_b.iter().sum::<f64>() / returns_b.len() as f64;

        let num: f64 = returns_a
            .iter()
            .zip(returns_b.iter())
            .map(|(ra, rb)| (ra - mean_a) * (rb - mean_b))
            .sum();

        let den: f64 = returns_b.iter().map(|rb| (rb - mean_b).powi(2)).sum();

        if den == 0.0 { 1.0 } else { num / den }
    }

    fn calculate_z_score(&self, spread: &[f64]) -> f64 {
        if spread.len() < 2 {
            return 0.0;
        }
        let mean = spread.iter().sum::<f64>() / spread.len() as f64;
        let variance: f64 =
            spread.iter().map(|s| (s - mean).powi(2)).sum::<f64>() / spread.len() as f64;
        let std = variance.sqrt();

        if std == 0.0 {
            0.0
        } else {
            (spread[spread.len() - 1] - mean) / std
        }
    }

    pub fn analyze(&self) -> Option<PairSignal> {
        if self.prices_a.len() < self.config.lookback_period {
            return None;
        }

        let hedge_ratio = self.calculate_hedge_ratio();
        let spread_values: Vec<f64> = self
            .prices_a
            .iter()
            .zip(self.prices_b.iter())
            .map(|(pa, pb)| pa - hedge_ratio * pb)
            .collect();

        let z_score = self.calculate_z_score(&spread_values);

        let action = if z_score > self.config.entry_z_score {
            PairAction::ShortLong
        } else if z_score < -self.config.entry_z_score {
            PairAction::LongShort
        } else {
            PairAction::Neutral
        };

        let confidence = (z_score.abs() / self.config.entry_z_score).min(1.0);

        Some(PairSignal {
            action,
            z_score,
            hedge_ratio,
            confidence,
        })
    }
}
