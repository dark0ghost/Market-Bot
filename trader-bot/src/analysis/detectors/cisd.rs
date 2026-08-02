use crate::analysis::detectors::{Candle, Swing, SwingType};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CISDType {
    Bullish,
    Bearish,
}

#[derive(Debug, Clone, Copy)]
pub struct CISDSignal {
    pub cisd_type: CISDType,
    pub broken_level: f64,
    pub swing_index: usize,
}

#[derive(Debug, Clone, Copy)]
pub struct Breaker {
    pub breaker_type: BreakerType,
    pub high: f64,
    pub low: f64,
    pub index: usize,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BreakerType {
    BullishBreaker,
    BearishBreaker,
}

/// Detect market structure shifts (CISD).
/// Bearish = price broke below recent swing low.
/// Bullish = price broke above recent swing high.
pub fn detect_cisd(candles: &[Candle], swings: &[Swing]) -> Vec<CISDSignal> {
    let mut signals = Vec::new();
    if candles.is_empty() {
        return signals;
    }

    let current_price = match candles.last() {
        Some(c) => c.close,
        None => return signals,
    };

    let recent_lows: Vec<&Swing> = swings
        .iter()
        .filter(|s| matches!(s.swing_type, SwingType::Low))
        .rev()
        .take(3)
        .collect();

    let recent_highs: Vec<&Swing> = swings
        .iter()
        .filter(|s| matches!(s.swing_type, SwingType::High))
        .rev()
        .take(3)
        .collect();

    for swing in &recent_lows {
        if current_price < swing.price {
            signals.push(CISDSignal {
                cisd_type: CISDType::Bearish,
                broken_level: swing.price,
                swing_index: swing.index,
            });
            break;
        }
    }

    for swing in &recent_highs {
        if current_price > swing.price {
            signals.push(CISDSignal {
                cisd_type: CISDType::Bullish,
                broken_level: swing.price,
                swing_index: swing.index,
            });
            break;
        }
    }

    signals
}

/// Detect the breaker candle after a CISD signal.
/// Bullish breaker = red candle before bullish CISD.
/// Bearish breaker = green candle before bearish CISD.
pub fn detect_cisd_breaker(candles: &[Candle], cisd: &CISDSignal) -> Option<Breaker> {
    let start = cisd.swing_index.saturating_sub(1);
    let end = start.saturating_sub(9); // search up to 10 candles back
    let search_range = (end..=start).rev();

    match cisd.cisd_type {
        CISDType::Bearish => {
            for i in search_range {
                if i < candles.len() && candles[i].close > candles[i].open {
                    return Some(Breaker {
                        breaker_type: BreakerType::BearishBreaker,
                        high: candles[i].high,
                        low: candles[i].low,
                        index: i,
                    });
                }
            }
        }
        CISDType::Bullish => {
            for i in search_range {
                if i < candles.len() && candles[i].close < candles[i].open {
                    return Some(Breaker {
                        breaker_type: BreakerType::BullishBreaker,
                        high: candles[i].high,
                        low: candles[i].low,
                        index: i,
                    });
                }
            }
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cisd_bearish() {
        let candles = vec![
            Candle {
                open: 105.0,
                high: 106.0,
                low: 104.0,
                close: 105.0,
                volume: 0.0,
            },
            Candle {
                open: 95.0,
                high: 96.0,
                low: 94.0,
                close: 95.0,
                volume: 0.0,
            },
        ];
        let swings = vec![Swing {
            swing_type: SwingType::Low,
            price: 100.0,
            index: 0,
            strength: 3,
        }];
        let signals = detect_cisd(&candles, &swings);
        assert_eq!(signals.len(), 1);
        assert_eq!(signals[0].cisd_type, CISDType::Bearish);
    }

    #[test]
    fn test_cisd_bullish() {
        let candles = vec![
            Candle {
                open: 95.0,
                high: 96.0,
                low: 94.0,
                close: 95.0,
                volume: 0.0,
            },
            Candle {
                open: 105.0,
                high: 106.0,
                low: 104.0,
                close: 105.0,
                volume: 0.0,
            },
        ];
        let swings = vec![Swing {
            swing_type: SwingType::High,
            price: 100.0,
            index: 0,
            strength: 3,
        }];
        let signals = detect_cisd(&candles, &swings);
        assert_eq!(signals.len(), 1);
        assert_eq!(signals[0].cisd_type, CISDType::Bullish);
    }

    #[test]
    fn test_no_cisd() {
        let candles = vec![Candle {
            open: 100.0,
            high: 101.0,
            low: 99.0,
            close: 100.0,
            volume: 0.0,
        }];
        let swings = vec![
            Swing {
                swing_type: SwingType::Low,
                price: 90.0,
                index: 0,
                strength: 3,
            },
            Swing {
                swing_type: SwingType::High,
                price: 110.0,
                index: 1,
                strength: 3,
            },
        ];
        let signals = detect_cisd(&candles, &swings);
        assert!(signals.is_empty());
    }
}
