use crate::analysis::detectors::Candle;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FvgType {
    Bullish,
    Bearish,
}

#[derive(Debug, Clone, Copy)]
pub struct FvgState {
    pub filled: bool,
    pub inversed: bool,
    pub respected: bool,
}

#[derive(Debug, Clone)]
pub struct FairValueGap {
    pub fvg_type: FvgType,
    pub high: f64,
    pub low: f64,
    pub candle_index: usize,
    pub state: FvgState,
}

/// Detect Fair Value Gaps.
/// Bullish FVG: candle[i-2].high < candle[i].low (gap up)
/// Bearish FVG: candle[i-2].low > candle[i].high (gap down)
pub fn detect_fvg(candles: &[Candle]) -> Vec<FairValueGap> {
    let mut fvgs = Vec::new();
    if candles.len() < 3 {
        return fvgs;
    }

    for i in 2..candles.len() {
        let c1 = &candles[i - 2];
        let c3 = &candles[i];

        if c1.high < c3.low {
            fvgs.push(FairValueGap {
                fvg_type: FvgType::Bullish,
                high: c3.low,
                low: c1.high,
                candle_index: i,
                state: FvgState {
                    filled: false,
                    inversed: false,
                    respected: false,
                },
            });
        }

        if c1.low > c3.high {
            fvgs.push(FairValueGap {
                fvg_type: FvgType::Bearish,
                high: c1.low,
                low: c3.high,
                candle_index: i,
                state: FvgState {
                    filled: false,
                    inversed: false,
                    respected: false,
                },
            });
        }
    }

    fvgs
}

/// Update FVG states based on subsequent price action.
pub fn update_fvg_states(fvgs: &mut [FairValueGap], candles: &[Candle]) {
    for fvg in fvgs.iter_mut() {
        let start = (fvg.candle_index + 1).min(candles.len());
        for candle in &candles[start..] {
            match fvg.fvg_type {
                FvgType::Bullish => {
                    if candle.low <= fvg.high {
                        fvg.state.filled = true;
                    }
                    if candle.close < fvg.low {
                        fvg.state.inversed = true;
                        break;
                    }
                    if fvg.state.filled && candle.close > fvg.high {
                        fvg.state.respected = true;
                    }
                }
                FvgType::Bearish => {
                    if candle.high >= fvg.low {
                        fvg.state.filled = true;
                    }
                    if candle.close > fvg.high {
                        fvg.state.inversed = true;
                        break;
                    }
                    if fvg.state.filled && candle.close < fvg.low {
                        fvg.state.respected = true;
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_bullish_fvg() {
        let candles = vec![
            Candle {
                open: 100.0,
                high: 101.0,
                low: 99.0,
                close: 100.0,
                volume: 0.0,
            },
            Candle {
                open: 101.0,
                high: 102.0,
                low: 100.0,
                close: 101.0,
                volume: 0.0,
            },
            Candle {
                open: 103.0,
                high: 104.0,
                low: 102.0,
                close: 103.0,
                volume: 0.0,
            },
        ];
        let fvgs = detect_fvg(&candles);
        assert_eq!(fvgs.len(), 1);
        assert_eq!(fvgs[0].fvg_type, FvgType::Bullish);
    }

    #[test]
    fn test_detect_bearish_fvg() {
        let candles = vec![
            Candle {
                open: 103.0,
                high: 104.0,
                low: 102.0,
                close: 103.0,
                volume: 0.0,
            },
            Candle {
                open: 101.0,
                high: 102.0,
                low: 100.0,
                close: 101.0,
                volume: 0.0,
            },
            Candle {
                open: 100.0,
                high: 101.0,
                low: 99.0,
                close: 100.0,
                volume: 0.0,
            },
        ];
        let fvgs = detect_fvg(&candles);
        assert_eq!(fvgs.len(), 1);
        assert_eq!(fvgs[0].fvg_type, FvgType::Bearish);
    }

    #[test]
    fn test_no_fvg() {
        let candles = vec![
            Candle {
                open: 100.0,
                high: 101.0,
                low: 99.0,
                close: 100.0,
                volume: 0.0,
            },
            Candle {
                open: 100.0,
                high: 101.0,
                low: 99.0,
                close: 100.0,
                volume: 0.0,
            },
            Candle {
                open: 100.0,
                high: 101.0,
                low: 99.0,
                close: 100.0,
                volume: 0.0,
            },
        ];
        let fvgs = detect_fvg(&candles);
        assert!(fvgs.is_empty());
    }

    #[test]
    fn test_fvg_filled_and_respected() {
        // C1: 99-101, C2: any, C3: 102-104 => bullish FVG at 101-102
        let candles = vec![
            Candle {
                open: 100.0,
                high: 101.0,
                low: 99.0,
                close: 100.0,
                volume: 0.0,
            },
            Candle {
                open: 100.0,
                high: 101.0,
                low: 99.0,
                close: 100.0,
                volume: 0.0,
            },
            Candle {
                open: 103.0,
                high: 104.0,
                low: 102.0,
                close: 103.0,
                volume: 0.0,
            },
            // Price dips into gap (101-102)
            Candle {
                open: 103.0,
                high: 103.0,
                low: 101.5,
                close: 103.0,
                volume: 0.0,
            },
        ];
        let mut fvgs = detect_fvg(&candles);
        update_fvg_states(&mut fvgs, &candles);
        assert!(fvgs[0].state.filled);
        // Not inversed since close > low
        assert!(!fvgs[0].state.inversed);
    }
}
