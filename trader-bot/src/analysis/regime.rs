use std::collections::VecDeque;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum MarketRegime {
    Trending,
    Ranging,
    Volatile,
    Quiet,
}

impl MarketRegime {
    pub fn weight_adjustment(&self) -> f64 {
        match self {
            MarketRegime::Trending => 1.2,
            MarketRegime::Ranging => 1.0,
            MarketRegime::Volatile => 0.7,
            MarketRegime::Quiet => 0.9,
        }
    }
}

pub struct RegimeDetector {
    prices: VecDeque<f64>,
    adx_period: usize,
    atr_period: usize,
    atr_threshold_high: f64,
    atr_threshold_low: f64,
    adx_threshold: f64,
}

impl RegimeDetector {
    pub fn new(adx_period: usize, atr_period: usize) -> Self {
        let size = adx_period.max(atr_period) + 1;
        RegimeDetector {
            prices: VecDeque::with_capacity(size),
            adx_period,
            atr_period,
            atr_threshold_high: 0.03,
            atr_threshold_low: 0.01,
            adx_threshold: 25.0,
        }
    }

    pub fn add_price(&mut self, price: f64) {
        if self.prices.len() >= self.prices.capacity() {
            self.prices.pop_front();
        }
        self.prices.push_back(price);
    }

    pub fn detect(&self) -> MarketRegime {
        if self.prices.len() < self.adx_period.max(self.atr_period) + 1 {
            return MarketRegime::Quiet;
        }
        let vec: Vec<f64> = self.prices.iter().copied().collect();
        let atr = Self::atr(&vec, self.atr_period);
        let adx = Self::adx(&vec, self.adx_period);
        let avg_price = vec.iter().sum::<f64>() / vec.len() as f64;
        let atr_pct = atr / avg_price;
        match (adx > self.adx_threshold, atr_pct > self.atr_threshold_high, atr_pct < self.atr_threshold_low) {
            (true, _, _) => MarketRegime::Trending,
            (false, true, _) => MarketRegime::Volatile,
            (false, _, true) => MarketRegime::Quiet,
            (false, false, false) => MarketRegime::Ranging,
        }
    }

    fn atr(values: &[f64], period: usize) -> f64 {
        if values.len() < 2 { return 0.0; }
        let ranges: Vec<f64> = values.windows(2)
            .map(|w| (w[1] - w[0]).abs())
            .collect();
        let start = ranges.len().saturating_sub(period);
        let recent: &[f64] = &ranges[start..];
        recent.iter().sum::<f64>() / recent.len() as f64
    }

    fn adx(values: &[f64], period: usize) -> f64 {
        if values.len() < period + 1 { return 0.0; }
        let start = values.len().saturating_sub(period + 1);
        let slice = &values[start..];
        let up_moves: f64 = slice.windows(2).filter(|w| w[1] > w[0]).map(|w| (w[1] - w[0])).sum();
        let down_moves: f64 = slice.windows(2).filter(|w| w[1] < w[0]).map(|w| (w[0] - w[1])).sum();
        let total = up_moves + down_moves;
        if total == 0.0 { return 0.0; }
        let di_diff = (up_moves - down_moves).abs() / total * 100.0;
        di_diff
    }
}
