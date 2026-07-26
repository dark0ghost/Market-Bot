use crate::analysis::detectors::{Swing, SwingType};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SMType {
    Bullish,
    Bearish,
}

#[derive(Debug, Clone, Copy)]
pub struct SMTSignal {
    pub smt_type: SMType,
    pub strength: SMTStrength,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SMTStrength {
    Strong,
    Weak,
}

/// Detect SMT divergence between two correlated assets.
/// Bearish SMT: asset1 makes higher high, asset2 doesn't.
/// Bullish SMT: asset1 makes lower low, asset2 doesn't.
pub fn detect_smt(asset1_swings: &[Swing], asset2_swings: &[Swing]) -> Vec<SMTSignal> {
    let mut signals = Vec::new();

    let a1_highs: Vec<&Swing> = asset1_swings
        .iter()
        .filter(|s| matches!(s.swing_type, SwingType::High))
        .rev()
        .take(5)
        .collect();
    let a2_highs: Vec<&Swing> = asset2_swings
        .iter()
        .filter(|s| matches!(s.swing_type, SwingType::High))
        .rev()
        .take(5)
        .collect();

    if a1_highs.len() >= 2 && a2_highs.len() >= 2 {
        let a1_hh = a1_highs[0].price > a1_highs[1].price;
        let a2_hh = a2_highs[0].price > a2_highs[1].price;

        if a1_hh && !a2_hh {
            let strength =
                if (a1_highs[0].index as isize - a2_highs[0].index as isize).unsigned_abs() < 3 {
                    SMTStrength::Strong
                } else {
                    SMTStrength::Weak
                };
            signals.push(SMTSignal {
                smt_type: SMType::Bearish,
                strength,
            });
        }
    }

    let a1_lows: Vec<&Swing> = asset1_swings
        .iter()
        .filter(|s| matches!(s.swing_type, SwingType::Low))
        .rev()
        .take(5)
        .collect();
    let a2_lows: Vec<&Swing> = asset2_swings
        .iter()
        .filter(|s| matches!(s.swing_type, SwingType::Low))
        .rev()
        .take(5)
        .collect();

    if a1_lows.len() >= 2 && a2_lows.len() >= 2 {
        let a1_ll = a1_lows[0].price < a1_lows[1].price;
        let a2_ll = a2_lows[0].price < a2_lows[1].price;

        if a1_ll && !a2_ll {
            let strength =
                if (a1_lows[0].index as isize - a2_lows[0].index as isize).unsigned_abs() < 3 {
                    SMTStrength::Strong
                } else {
                    SMTStrength::Weak
                };
            signals.push(SMTSignal {
                smt_type: SMType::Bullish,
                strength,
            });
        }
    }

    signals
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bearish_smt() {
        let a1 = vec![
            Swing {
                swing_type: SwingType::High,
                price: 100.0,
                index: 0,
                strength: 3,
            },
            Swing {
                swing_type: SwingType::High,
                price: 105.0,
                index: 5,
                strength: 3,
            },
        ];
        let a2 = vec![
            Swing {
                swing_type: SwingType::High,
                price: 100.0,
                index: 0,
                strength: 3,
            },
            Swing {
                swing_type: SwingType::High,
                price: 100.0,
                index: 6,
                strength: 3,
            },
        ];
        let signals = detect_smt(&a1, &a2);
        assert_eq!(signals.len(), 1);
        assert_eq!(signals[0].smt_type, SMType::Bearish);
    }

    #[test]
    fn test_bullish_smt() {
        let a1 = vec![
            Swing {
                swing_type: SwingType::Low,
                price: 100.0,
                index: 0,
                strength: 3,
            },
            Swing {
                swing_type: SwingType::Low,
                price: 95.0,
                index: 5,
                strength: 3,
            },
        ];
        let a2 = vec![
            Swing {
                swing_type: SwingType::Low,
                price: 100.0,
                index: 0,
                strength: 3,
            },
            Swing {
                swing_type: SwingType::Low,
                price: 100.0,
                index: 6,
                strength: 3,
            },
        ];
        let signals = detect_smt(&a1, &a2);
        assert_eq!(signals.len(), 1);
        assert_eq!(signals[0].smt_type, SMType::Bullish);
    }

    #[test]
    fn test_no_smt() {
        let a1 = vec![
            Swing {
                swing_type: SwingType::High,
                price: 100.0,
                index: 0,
                strength: 3,
            },
            Swing {
                swing_type: SwingType::High,
                price: 105.0,
                index: 5,
                strength: 3,
            },
        ];
        let a2 = vec![
            Swing {
                swing_type: SwingType::High,
                price: 100.0,
                index: 0,
                strength: 3,
            },
            Swing {
                swing_type: SwingType::High,
                price: 106.0,
                index: 6,
                strength: 3,
            },
        ];
        let signals = detect_smt(&a1, &a2);
        assert!(signals.is_empty());
    }

    #[test]
    fn test_smt_strong_vs_weak() {
        let a1 = vec![
            Swing {
                swing_type: SwingType::High,
                price: 100.0,
                index: 0,
                strength: 3,
            },
            Swing {
                swing_type: SwingType::High,
                price: 105.0,
                index: 5,
                strength: 3,
            },
        ];
        let a2 = vec![
            Swing {
                swing_type: SwingType::High,
                price: 100.0,
                index: 0,
                strength: 3,
            },
            Swing {
                swing_type: SwingType::High,
                price: 100.0,
                index: 5,
                strength: 3,
            },
        ];
        let signals = detect_smt(&a1, &a2);
        assert_eq!(signals[0].strength, SMTStrength::Strong);
    }
}
