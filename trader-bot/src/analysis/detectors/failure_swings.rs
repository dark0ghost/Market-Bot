use crate::analysis::detectors::{Swing, SwingType};

#[derive(Debug, Clone, PartialEq)]
pub struct FailureSwing {
    pub level: f64,
    pub count: usize,
    pub swing_type: SwingType,
}

/// Detect failure swings - clusters of swing highs/lows at similar price levels.
/// Multiple touching of the same level = stronger draw on liquidity.
pub fn detect_failure_swings(swings: &[Swing], tolerance_pct: f64) -> Vec<FailureSwing> {
    let mut clusters: Vec<FailureSwing> = Vec::new();
    let mut used = vec![false; swings.len()];

    for i in 0..swings.len() {
        if used[i] {
            continue;
        }

        let mut cluster = vec![i];
        used[i] = true;

        for j in i + 1..swings.len() {
            if used[j] || swings[i].swing_type != swings[j].swing_type {
                continue;
            }

            let diff = (swings[i].price - swings[j].price).abs() / swings[i].price;
            if diff <= tolerance_pct / 100.0 {
                cluster.push(j);
                used[j] = true;
            }
        }

        if cluster.len() >= 2 {
            let avg_price: f64 =
                cluster.iter().map(|&idx| swings[idx].price).sum::<f64>() / cluster.len() as f64;
            clusters.push(FailureSwing {
                level: avg_price,
                count: cluster.len(),
                swing_type: swings[i].swing_type,
            });
        }
    }

    clusters
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_failure_swing_detected() {
        let swings = vec![
            Swing {
                swing_type: SwingType::High,
                price: 100.0,
                index: 0,
                strength: 3,
            },
            Swing {
                swing_type: SwingType::High,
                price: 100.5,
                index: 5,
                strength: 3,
            },
        ];
        let failures = detect_failure_swings(&swings, 1.0);
        assert_eq!(failures.len(), 1);
        assert_eq!(failures[0].count, 2);
    }

    #[test]
    fn test_failure_swing_no_cluster() {
        let swings = vec![
            Swing {
                swing_type: SwingType::High,
                price: 100.0,
                index: 0,
                strength: 3,
            },
            Swing {
                swing_type: SwingType::High,
                price: 110.0,
                index: 5,
                strength: 3,
            },
        ];
        let failures = detect_failure_swings(&swings, 1.0);
        assert!(failures.is_empty());
    }

    #[test]
    fn test_failure_swing_different_types_no_cluster() {
        let swings = vec![
            Swing {
                swing_type: SwingType::High,
                price: 100.0,
                index: 0,
                strength: 3,
            },
            Swing {
                swing_type: SwingType::Low,
                price: 100.5,
                index: 5,
                strength: 3,
            },
        ];
        let failures = detect_failure_swings(&swings, 1.0);
        assert!(failures.is_empty());
    }

    #[test]
    fn test_failure_swing_three_touches() {
        let swings = vec![
            Swing {
                swing_type: SwingType::Low,
                price: 100.0,
                index: 0,
                strength: 3,
            },
            Swing {
                swing_type: SwingType::Low,
                price: 100.2,
                index: 5,
                strength: 3,
            },
            Swing {
                swing_type: SwingType::Low,
                price: 99.8,
                index: 10,
                strength: 3,
            },
        ];
        let failures = detect_failure_swings(&swings, 1.0);
        assert_eq!(failures.len(), 1);
        assert_eq!(failures[0].count, 3);
    }
}
