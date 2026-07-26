use crate::analysis::detectors::Candle;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DisplacementDirection {
    Bullish,
    Bearish,
}

#[derive(Debug, Clone, Copy)]
pub struct Displacement {
    pub index: usize,
    pub direction: DisplacementDirection,
    pub size_ratio: f64,
}

/// Detect displacement — candles significantly larger than the recent average.
pub fn detect_displacement(
    candles: &[Candle],
    lookback: usize,
    threshold_multiplier: f64,
) -> Vec<Displacement> {
    let mut displacements = Vec::new();
    let total = candles.len();

    if total < 3 * lookback + lookback {
        return displacements;
    }

    // Compute average size from candles before the lookback window
    let window_start = total - 3 * lookback;
    let window_end = total - lookback;

    let sizes: Vec<f64> = candles[window_start..window_end]
        .iter()
        .map(|c| (c.close - c.open).abs())
        .collect();

    let avg_size: f64 = if sizes.is_empty() {
        0.0
    } else {
        sizes.iter().sum::<f64>() / sizes.len() as f64
    };

    if avg_size <= 0.0 {
        return displacements;
    }

    for (offset, candle) in candles[window_end..].iter().enumerate() {
        let size = (candle.close - candle.open).abs();
        if size > avg_size * threshold_multiplier {
            displacements.push(Displacement {
                index: window_end + offset,
                direction: if candle.close > candle.open {
                    DisplacementDirection::Bullish
                } else {
                    DisplacementDirection::Bearish
                },
                size_ratio: size / avg_size,
            });
        }
    }

    displacements
}

#[cfg(test)]
mod tests {
    use super::*;

    fn normal_candles(n: usize, open: f64) -> Vec<Candle> {
        let mut candles = Vec::new();
        for _i in 0..n {
            candles.push(Candle {
                open,
                high: open + 1.0,
                low: open - 1.0,
                close: open + 0.5,
                volume: 0.0,
            });
        }
        candles
    }

    #[test]
    fn test_displacement_detected() {
        let mut candles = normal_candles(20, 100.0);
        // Add a large bullish candle
        candles.push(Candle {
            open: 100.0,
            high: 110.0,
            low: 100.0,
            close: 110.0,
            volume: 0.0,
        });
        let displacements = detect_displacement(&candles, 5, 2.0);
        assert_eq!(displacements.len(), 1);
        assert_eq!(displacements[0].direction, DisplacementDirection::Bullish);
    }

    #[test]
    fn test_no_displacement_with_normal_candles() {
        let candles = normal_candles(20, 100.0);
        let displacements = detect_displacement(&candles, 5, 2.0);
        assert!(displacements.is_empty());
    }

    #[test]
    fn test_displacement_bearish() {
        let mut candles = normal_candles(20, 100.0);
        candles.push(Candle {
            open: 100.0,
            high: 100.0,
            low: 90.0,
            close: 90.0,
            volume: 0.0,
        });
        let displacements = detect_displacement(&candles, 5, 2.0);
        assert_eq!(displacements.len(), 1);
        assert_eq!(displacements[0].direction, DisplacementDirection::Bearish);
    }

    #[test]
    fn test_not_enough_candles() {
        let candles = normal_candles(5, 100.0);
        let displacements = detect_displacement(&candles, 5, 2.0);
        assert!(displacements.is_empty());
    }
}
