use anyhow::Result;
use chrono::{DateTime, Utc};
use t_invest_sdk::api::Quotation;
use ta::{
    Next,
    indicators::{BollingerBands, MovingAverageConvergenceDivergence, RelativeStrengthIndex},
};

/// Extract price from Quotation
fn extract_price(quotation: &Option<Quotation>) -> Result<f64> {
    match quotation {
        Some(q) => Ok(q.units as f64 + (q.nano as f64 / 1_000_000_000.0)),
        None => Ok(0.0),
    }
}

/// Technical analysis result
#[derive(Debug, Clone)]
pub struct TechnicalAnalysis {
    pub ticker: String,
    pub timestamp: DateTime<Utc>,
    pub current_price: f64,
    pub trend: Trend,
    pub rsi: Option<f64>,
    pub macd: Option<MacdValues>,
    pub bollinger: Option<BollingerValues>,
    pub volume_analysis: VolumeAnalysis,
    pub support_levels: Vec<f64>,
    pub resistance_levels: Vec<f64>,
    pub recommendation: Recommendation,
}

/// Trend direction
#[derive(Debug, Clone, PartialEq)]
pub enum Trend {
    Bullish,
    Bearish,
    Sideways,
}

/// MACD values
#[derive(Debug, Clone)]
pub struct MacdValues {
    pub macd_line: f64,
    pub signal_line: f64,
    pub histogram: f64,
}

/// Bollinger Bands
#[derive(Debug, Clone)]
pub struct BollingerValues {
    pub upper: f64,
    pub middle: f64,
    pub lower: f64,
    pub bandwidth: f64,
}

/// Volume analysis
#[derive(Debug, Clone)]
pub struct VolumeAnalysis {
    pub current_volume: f64,
    pub avg_volume: f64,
    pub volume_ratio: f64,
    pub is_unusual: bool,
}

/// Action recommendation
#[derive(Debug, Clone, PartialEq)]
pub enum Recommendation {
    StrongBuy,
    Buy,
    Hold,
    Sell,
    StrongSell,
}

/// Technical analysis service
pub struct TechnicalAnalyzer {
    rsi_period: usize,
    macd_fast: usize,
    macd_slow: usize,
    macd_signal: usize,
    bollinger_period: usize,
    bollinger_std_dev: f64,
}

impl TechnicalAnalyzer {
    pub fn new() -> Self {
        TechnicalAnalyzer {
            rsi_period: 14,
            macd_fast: 12,
            macd_slow: 26,
            macd_signal: 9,
            bollinger_period: 20,
            bollinger_std_dev: 2.0,
        }
    }

    /// Analyze candlestick data
    pub fn analyze(
        &self,
        ticker: &str,
        candles: &[t_invest_sdk::api::HistoricCandle],
    ) -> Result<TechnicalAnalysis> {
        if candles.is_empty() {
            // Return default analysis if no data
            return Ok(TechnicalAnalysis {
                ticker: ticker.to_string(),
                timestamp: chrono::Utc::now(),
                current_price: 0.0,
                trend: Trend::Sideways,
                rsi: None,
                macd: None,
                bollinger: None,
                volume_analysis: VolumeAnalysis {
                    current_volume: 0.0,
                    avg_volume: 0.0,
                    volume_ratio: 0.0,
                    is_unusual: false,
                },
                support_levels: vec![],
                resistance_levels: vec![],
                recommendation: Recommendation::Hold,
            });
        }

        let current_candle = candles.last().unwrap();
        let current_price = extract_price(&current_candle.close)?;
        let current_time = current_candle.time.unwrap_or_default();

        // Extract closing prices
        let closes: Vec<f64> = candles
            .iter()
            .filter_map(|c| extract_price(&c.close).ok())
            .collect();

        // Extract volumes - HistoricCandle.volume is i64
        let volumes: Vec<f64> = candles.iter().map(|c| c.volume as f64).collect();

        // RSI
        let rsi = self.calculate_rsi(&closes);

        // MACD
        let macd = self.calculate_macd(&closes);

        // Bollinger Bands
        let bollinger = self.calculate_bollinger(&closes);

        // Volume analysis
        let volume_analysis = self.analyze_volume(&volumes);

        // Determine trend
        let trend = self.determine_trend(&closes, rsi.as_ref(), macd.as_ref());

        // Support and resistance levels
        let (support, resistance) = self.find_support_resistance(&closes);

        // Final recommendation
        let recommendation = self.generate_recommendation(
            &trend,
            rsi.as_ref(),
            macd.as_ref(),
            &bollinger,
            &volume_analysis,
            current_price,
        );

        Ok(TechnicalAnalysis {
            ticker: ticker.to_string(),
            timestamp: DateTime::from_timestamp(current_time.seconds, 0).unwrap_or_else(Utc::now),
            current_price,
            trend,
            rsi,
            macd,
            bollinger: Some(bollinger),
            volume_analysis,
            support_levels: support,
            resistance_levels: resistance,
            recommendation,
        })
    }

    /// Calculate RSI
    fn calculate_rsi(&self, prices: &[f64]) -> Option<f64> {
        if prices.len() < self.rsi_period {
            return None;
        }

        let mut rsi = RelativeStrengthIndex::new(self.rsi_period).unwrap();
        let mut result = None;

        for price in prices {
            result = Some(rsi.next(*price));
        }

        result
    }

    /// Calculate MACD
    fn calculate_macd(&self, prices: &[f64]) -> Option<MacdValues> {
        if prices.len() < self.macd_slow {
            return None;
        }

        let mut macd_indicator = MovingAverageConvergenceDivergence::new(
            self.macd_fast,
            self.macd_slow,
            self.macd_signal,
        )
        .unwrap();

        let mut macd_line = 0.0;
        let mut signal_line = 0.0;

        for price in prices {
            let macd_value = macd_indicator.next(*price);
            macd_line = macd_value.macd;
            signal_line = macd_value.signal;
        }

        Some(MacdValues {
            macd_line,
            signal_line,
            histogram: macd_line - signal_line,
        })
    }

    /// Calculate Bollinger Bands
    fn calculate_bollinger(&self, prices: &[f64]) -> BollingerValues {
        let period = if prices.len() >= self.bollinger_period {
            self.bollinger_period
        } else {
            prices.len().max(5)
        };

        let mut bb = BollingerBands::new(period, self.bollinger_std_dev).unwrap();

        let mut upper = 0.0;
        let mut middle = 0.0;
        let mut lower = 0.0;

        for price in prices {
            let bb_value = bb.next(*price);
            upper = bb_value.upper;
            middle = bb_value.average;
            lower = bb_value.lower;
        }

        let bandwidth = if middle > 0.0 {
            (upper - lower) / middle
        } else {
            0.0
        };

        BollingerValues {
            upper,
            middle,
            lower,
            bandwidth,
        }
    }

    /// Volume analysis
    fn analyze_volume(&self, volumes: &[f64]) -> VolumeAnalysis {
        if volumes.is_empty() {
            return VolumeAnalysis {
                current_volume: 0.0,
                avg_volume: 0.0,
                volume_ratio: 0.0,
                is_unusual: false,
            };
        }

        let current_volume = *volumes.last().unwrap();
        let avg_volume: f64 = volumes.iter().sum::<f64>() / volumes.len() as f64;
        let volume_ratio = if avg_volume > 0.0 {
            current_volume / avg_volume
        } else {
            0.0
        };

        // Unusual volume - if 2x above average
        let is_unusual = volume_ratio > 2.0;

        VolumeAnalysis {
            current_volume,
            avg_volume,
            volume_ratio,
            is_unusual,
        }
    }

    /// Determine trend
    fn determine_trend(
        &self,
        prices: &[f64],
        rsi: Option<&f64>,
        macd: Option<&MacdValues>,
    ) -> Trend {
        // Price analysis
        let price_trend = if prices.len() >= 10 {
            let recent_avg: f64 = prices.iter().rev().take(5).sum::<f64>() / 5.0;
            let older_avg: f64 = prices.iter().rev().skip(5).take(5).sum::<f64>() / 5.0;

            if recent_avg > older_avg * 1.02 {
                Trend::Bullish
            } else if recent_avg < older_avg * 0.98 {
                Trend::Bearish
            } else {
                Trend::Sideways
            }
        } else {
            Trend::Sideways
        };

        // RSI confirmation
        let rsi_confirmation = rsi.map_or(0, |&v| {
            if v > 50.0 {
                1
            } else if v < 50.0 {
                -1
            } else {
                0
            }
        });

        // MACD confirmation
        let macd_confirmation = macd.map_or(0, |m| {
            if m.histogram > 0.0 {
                1
            } else if m.histogram < 0.0 {
                -1
            } else {
                0
            }
        });

        // Combined signal
        let combined = match price_trend {
            Trend::Bullish => 1,
            Trend::Bearish => -1,
            Trend::Sideways => 0,
        } + rsi_confirmation
            + macd_confirmation;

        if combined >= 2 {
            Trend::Bullish
        } else if combined <= -2 {
            Trend::Bearish
        } else {
            Trend::Sideways
        }
    }

    /// Find support and resistance levels
    fn find_support_resistance(&self, prices: &[f64]) -> (Vec<f64>, Vec<f64>) {
        if prices.is_empty() {
            return (vec![], vec![]);
        }

        let min_price = prices.iter().cloned().fold(f64::INFINITY, f64::min);
        let max_price = prices.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let current_price = *prices.last().unwrap();

        // Simple levels based on extremes
        let mut supports = vec![min_price];
        let mut resistances = vec![max_price];

        // Add intermediate levels
        let range = max_price - min_price;
        if range > 0.0 {
            let step = range / 5.0;
            for i in 1..5 {
                let level = min_price + step * i as f64;
                if level < current_price {
                    supports.push(level);
                } else {
                    resistances.push(level);
                }
            }
        }

        supports.sort_by(|a, b| b.partial_cmp(a).unwrap());
        resistances.sort_by(|a, b| a.partial_cmp(b).unwrap());

        (supports, resistances)
    }

    /// Generate recommendation
    fn generate_recommendation(
        &self,
        trend: &Trend,
        rsi: Option<&f64>,
        macd: Option<&MacdValues>,
        bollinger: &BollingerValues,
        volume: &VolumeAnalysis,
        current_price: f64,
    ) -> Recommendation {
        let mut score = 0;

        // Trend
        match trend {
            Trend::Bullish => score += 2,
            Trend::Bearish => score -= 2,
            Trend::Sideways => score += 0,
        }

        // RSI
        if let Some(&rsi_value) = rsi {
            if rsi_value < 30.0 {
                score += 2; // Oversold
            } else if rsi_value > 70.0 {
                score -= 2; // Overbought
            } else if rsi_value < 40.0 {
                score += 1;
            } else if rsi_value > 60.0 {
                score -= 1;
            }
        }

        // MACD
        if let Some(m) = macd {
            if m.histogram > 0.0 && m.macd_line > m.signal_line {
                score += 1;
            } else if m.histogram < 0.0 && m.macd_line < m.signal_line {
                score -= 1;
            }
        }

        // Bollinger Bands
        if current_price < bollinger.lower {
            score += 1; // Price at lower band
        } else if current_price > bollinger.upper {
            score -= 1; // Price at upper band
        }

        // Volume
        if volume.is_unusual && *trend == Trend::Bullish {
            score += 1; // Volume confirmation of trend
        }

        // Convert to recommendation
        match score {
            s if s >= 4 => Recommendation::StrongBuy,
            s if s >= 2 => Recommendation::Buy,
            s if s <= -4 => Recommendation::StrongSell,
            s if s <= -2 => Recommendation::Sell,
            _ => Recommendation::Hold,
        }
    }
}

impl Default for TechnicalAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_recommendation_from_score() {
        let analyzer = TechnicalAnalyzer::new();

        // Test data
        let trend = Trend::Bullish;
        let rsi = Some(35.0);
        let macd = Some(MacdValues {
            macd_line: 1.5,
            signal_line: 1.2,
            histogram: 0.3,
        });
        let bollinger = BollingerValues {
            upper: 110.0,
            middle: 100.0,
            lower: 90.0,
            bandwidth: 0.2,
        };
        let volume = VolumeAnalysis {
            current_volume: 1000.0,
            avg_volume: 500.0,
            volume_ratio: 2.0,
            is_unusual: true,
        };

        let recommendation = analyzer.generate_recommendation(
            &trend,
            rsi.as_ref(),
            macd.as_ref(),
            &bollinger,
            &volume,
            95.0,
        );

        assert_eq!(recommendation, Recommendation::StrongBuy);
    }
}
