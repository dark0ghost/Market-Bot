use crate::analysis::detectors::Candle;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SwingType {
    High,
    Low,
}

#[derive(Debug, Clone, Copy)]
pub struct Swing {
    pub swing_type: SwingType,
    pub price: f64,
    pub index: usize,
    pub strength: usize,
}

/// Detect swing highs/lows.
/// A swing high has lower highs on both sides (lookback candles each way).
pub fn detect_swings(candles: &[Candle], lookback: usize) -> Vec<Swing> {
    let mut swings = Vec::new();
    if candles.len() < 2 * lookback + 1 {
        return swings;
    }

    for i in lookback..candles.len() - lookback {
        let is_swing_high = (1..=lookback).all(|j| {
            candles[i].high > candles[i - j].high && candles[i].high > candles[i + j].high
        });

        let is_swing_low = (1..=lookback)
            .all(|j| candles[i].low < candles[i - j].low && candles[i].low < candles[i + j].low);

        if is_swing_high {
            swings.push(Swing {
                swing_type: SwingType::High,
                price: candles[i].high,
                index: i,
                strength: lookback,
            });
        }
        if is_swing_low {
            swings.push(Swing {
                swing_type: SwingType::Low,
                price: candles[i].low,
                index: i,
                strength: lookback,
            });
        }
    }

    swings
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_candles() -> Vec<Candle> {
        vec![
            Candle {
                open: 100.0,
                high: 102.0,
                low: 99.0,
                close: 101.0,
                volume: 0.0,
            },
            Candle {
                open: 101.0,
                high: 103.0,
                low: 100.0,
                close: 102.0,
                volume: 0.0,
            },
            Candle {
                open: 102.0,
                high: 105.0,
                low: 101.0,
                close: 104.0,
                volume: 0.0,
            },
            Candle {
                open: 104.0,
                high: 106.0,
                low: 103.0,
                close: 105.0,
                volume: 0.0,
            },
            Candle {
                open: 105.0,
                high: 104.0,
                low: 101.0,
                close: 102.0,
                volume: 0.0,
            },
            Candle {
                open: 102.0,
                high: 103.0,
                low: 99.0,
                close: 100.0,
                volume: 0.0,
            },
            Candle {
                open: 100.0,
                high: 101.0,
                low: 98.0,
                close: 99.0,
                volume: 0.0,
            },
        ]
    }

    #[test]
    fn test_detect_swing_high() {
        let candles = sample_candles();
        let swings = detect_swings(&candles, 2);
        let highs: Vec<_> = swings
            .iter()
            .filter(|s| matches!(s.swing_type, SwingType::High))
            .collect();
        assert_eq!(highs.len(), 1);
        assert!((highs[0].price - 106.0).abs() < 0.01);
        assert_eq!(highs[0].index, 3);
    }

    #[test]
    fn test_detect_swing_low() {
        let candles = vec![
            Candle {
                open: 100.0,
                high: 101.0,
                low: 99.0,
                close: 100.0,
                volume: 0.0,
            },
            Candle {
                open: 99.0,
                high: 100.0,
                low: 98.0,
                close: 99.0,
                volume: 0.0,
            },
            Candle {
                open: 98.0,
                high: 99.0,
                low: 97.0,
                close: 98.0,
                volume: 0.0,
            },
            Candle {
                open: 97.0,
                high: 98.0,
                low: 95.0,
                close: 96.0,
                volume: 0.0,
            },
            Candle {
                open: 96.0,
                high: 97.0,
                low: 96.0,
                close: 97.0,
                volume: 0.0,
            },
            Candle {
                open: 97.0,
                high: 98.0,
                low: 97.0,
                close: 98.0,
                volume: 0.0,
            },
            Candle {
                open: 98.0,
                high: 99.0,
                low: 98.0,
                close: 99.0,
                volume: 0.0,
            },
        ];
        // Index 3 has low=95, sides have lows 97 and 96 — both higher → swing low
        let swings = detect_swings(&candles, 2);
        let lows: Vec<_> = swings
            .iter()
            .filter(|s| matches!(s.swing_type, SwingType::Low))
            .collect();
        assert_eq!(lows.len(), 1);
        assert!((lows[0].price - 95.0).abs() < 0.01);
    }

    #[test]
    fn test_swing_not_enough_data() {
        let candles = vec![
            Candle {
                open: 100.0,
                high: 101.0,
                low: 99.0,
                close: 100.0,
                volume: 0.0
            };
            3
        ];
        let swings = detect_swings(&candles, 3);
        assert!(swings.is_empty());
    }
}
