use std::collections::VecDeque;

#[derive(Debug, Clone)]
pub struct StatArbConfig {
    pub lookback_period: usize,
    pub entry_z_threshold: f64,
    pub exit_z_threshold: f64,
}

#[derive(Debug, Clone)]
pub struct StatArbSignal {
    pub z_score: f64,
    pub signal: f64,
    pub upper_band: f64,
    pub lower_band: f64,
    pub mean: f64,
}

pub struct StatisticalArbitrage {
    config: StatArbConfig,
    prices: VecDeque<f64>,
}

impl StatisticalArbitrage {
    pub fn new(config: StatArbConfig) -> Self {
        let lookback = config.lookback_period;
        StatisticalArbitrage {
            config,
            prices: VecDeque::with_capacity(lookback),
        }
    }

    pub fn update(&mut self, price: f64) {
        if self.prices.len() >= self.config.lookback_period {
            self.prices.pop_front();
        }
        self.prices.push_back(price);
    }

    pub fn analyze(&self) -> Option<StatArbSignal> {
        if self.prices.len() < self.config.lookback_period {
            return None;
        }

        let prices: Vec<f64> = self.prices.iter().copied().collect();
        let mean = prices.iter().sum::<f64>() / prices.len() as f64;

        let variance: f64 =
            prices.iter().map(|p| (p - mean).powi(2)).sum::<f64>() / prices.len() as f64;

        let std = variance.sqrt();
        if std == 0.0 {
            return None;
        }

        let last_price = *self.prices.back()?;
        let z_score = (last_price - mean) / std;

        Some(StatArbSignal {
            z_score,
            signal: -z_score,
            upper_band: mean + std * self.config.entry_z_threshold,
            lower_band: mean - std * self.config.entry_z_threshold,
            mean,
        })
    }
}
