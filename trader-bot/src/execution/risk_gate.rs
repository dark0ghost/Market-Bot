use crate::config::RiskManagementConfig;

/// Result of a pre-trade risk evaluation.
#[derive(Debug, Clone, PartialEq)]
pub enum PreTradeCheck {
    Allow,
    Reject(String),
}

/// Inputs for a single pre-trade risk evaluation.
#[derive(Debug, Clone)]
pub struct PreTradeInput {
    /// Available (free) balance before this order, same currency as order_value.
    pub available_balance: f64,
    /// Number of currently open positions on the account.
    pub open_positions: u32,
    /// Notional value of the proposed order (quantity * price).
    pub order_value: f64,
    /// Current daily PnL (can be negative for losses).
    pub daily_pnl: f64,
    /// Total starting balance for the day (used to compute loss percentage).
    pub daily_starting_balance: f64,
}

/// Evaluate hard risk limits before placing an order.
/// When `risk` is None, only the structural checks (non-positive order value) apply.
pub fn evaluate_pre_trade(
    input: &PreTradeInput,
    risk: Option<&RiskManagementConfig>,
) -> PreTradeCheck {
    // 1. Structural check: order value must be a positive, finite number.
    if !input.order_value.is_finite() || input.order_value <= 0.0 {
        return PreTradeCheck::Reject("non-positive order value".to_string());
    }

    // 2. Risk-limit checks (only when a risk config is provided).
    if let Some(risk) = risk {
        // 2a. Daily loss limit / circuit breaker.
        // If daily loss exceeds max_loss_pct of starting balance, halt all trading.
        if input.daily_starting_balance > 0.0 {
            let daily_loss_pct = (-input.daily_pnl / input.daily_starting_balance) * 100.0;
            if daily_loss_pct >= risk.max_loss_pct {
                return PreTradeCheck::Reject(format!(
                    "daily loss limit reached: {:.2}% loss >= {:.2}% max (circuit breaker engaged)",
                    daily_loss_pct, risk.max_loss_pct
                ));
            }
        }

        // 2b. Open-position cap.
        if input.open_positions >= risk.max_open_positions {
            return PreTradeCheck::Reject(format!(
                "max open positions reached ({}/{})",
                input.open_positions, risk.max_open_positions
            ));
        }

        // 2c. Minimum balance reserve.
        let remaining = input.available_balance - input.order_value;
        if remaining < risk.min_balance_reserve {
            return PreTradeCheck::Reject(format!(
                "order would breach the min balance reserve: remaining {:.4} < reserve {:.4} (shortfall {:.4})",
                remaining,
                risk.min_balance_reserve,
                risk.min_balance_reserve - remaining
            ));
        }
    }

    // 3. All checks passed.
    PreTradeCheck::Allow
}

#[cfg(test)]
mod tests {
    use super::*;

    fn risk() -> RiskManagementConfig {
        RiskManagementConfig {
            max_loss_pct: 5.0,
            take_profit_pct: 2.0,
            stop_loss_pct: 1.0,
            max_open_positions: 3,
            min_balance_reserve: 100.0,
        }
    }

    fn input(balance: f64, positions: u32, order_value: f64) -> PreTradeInput {
        PreTradeInput {
            available_balance: balance,
            open_positions: positions,
            order_value,
            daily_pnl: 0.0,
            daily_starting_balance: 10000.0,
        }
    }

    #[test]
    fn allows_within_limits() {
        let input = input(1000.0, 1, 200.0);
        assert_eq!(
            evaluate_pre_trade(&input, Some(&risk())),
            PreTradeCheck::Allow
        );
    }

    #[test]
    fn rejects_zero_order_value() {
        let input = input(1000.0, 0, 0.0);
        assert_eq!(
            evaluate_pre_trade(&input, Some(&risk())),
            PreTradeCheck::Reject("non-positive order value".to_string())
        );
    }

    #[test]
    fn rejects_negative_order_value() {
        let input = input(1000.0, 0, -50.0);
        assert_eq!(
            evaluate_pre_trade(&input, Some(&risk())),
            PreTradeCheck::Reject("non-positive order value".to_string())
        );
    }

    #[test]
    fn rejects_nan_order_value() {
        let input = input(1000.0, 0, f64::NAN);
        assert_eq!(
            evaluate_pre_trade(&input, Some(&risk())),
            PreTradeCheck::Reject("non-positive order value".to_string())
        );
    }

    #[test]
    fn rejects_infinite_order_value() {
        let input = input(1000.0, 0, f64::INFINITY);
        assert_eq!(
            evaluate_pre_trade(&input, Some(&risk())),
            PreTradeCheck::Reject("non-positive order value".to_string())
        );
    }

    #[test]
    fn rejects_when_open_positions_equals_max() {
        let input = input(1000.0, 3, 100.0);
        match evaluate_pre_trade(&input, Some(&risk())) {
            PreTradeCheck::Reject(msg) => assert!(msg.contains("max open positions reached (3/3)")),
            other => panic!("expected reject, got {other:?}"),
        }
    }

    #[test]
    fn rejects_when_open_positions_exceeds_max() {
        let input = input(1000.0, 4, 100.0);
        match evaluate_pre_trade(&input, Some(&risk())) {
            PreTradeCheck::Reject(msg) => assert!(msg.contains("max open positions reached (4/3)")),
            other => panic!("expected reject, got {other:?}"),
        }
    }

    #[test]
    fn rejects_when_reserve_breached() {
        // remaining = 150 - 100 = 50, which is < reserve of 100.
        let input = input(150.0, 0, 100.0);
        match evaluate_pre_trade(&input, Some(&risk())) {
            PreTradeCheck::Reject(msg) => assert!(msg.contains("min balance reserve")),
            other => panic!("expected reject, got {other:?}"),
        }
    }

    #[test]
    fn allows_when_risk_is_none_and_order_value_positive() {
        let input = input(0.0, 999, 10.0);
        assert_eq!(evaluate_pre_trade(&input, None), PreTradeCheck::Allow);
    }

    #[test]
    fn rejects_when_risk_is_none_and_order_value_non_positive() {
        let input = input(0.0, 0, 0.0);
        assert_eq!(
            evaluate_pre_trade(&input, None),
            PreTradeCheck::Reject("non-positive order value".to_string())
        );
    }

    #[test]
    fn allows_at_reserve_boundary() {
        // remaining = 300 - 200 = 100, exactly equal to the reserve -> Allow
        let input = input(300.0, 0, 200.0);
        assert_eq!(
            evaluate_pre_trade(&input, Some(&risk())),
            PreTradeCheck::Allow
        );
    }

    #[test]
    fn rejects_daily_loss_limit_reached() {
        // Daily loss = 600, which is 6% of 10000 starting balance > 5% max
        let input = PreTradeInput {
            available_balance: 1000.0,
            open_positions: 1,
            order_value: 100.0,
            daily_pnl: -600.0,
            daily_starting_balance: 10000.0,
        };
        match evaluate_pre_trade(&input, Some(&risk())) {
            PreTradeCheck::Reject(msg) => {
                assert!(msg.contains("daily loss limit reached"));
                assert!(msg.contains("circuit breaker engaged"));
            }
            other => panic!("expected reject, got {other:?}"),
        }
    }

    #[test]
    fn allows_within_daily_loss_limit() {
        // Daily loss = 400, which is 4% of 10000 starting balance < 5% max
        let input = PreTradeInput {
            available_balance: 1000.0,
            open_positions: 1,
            order_value: 100.0,
            daily_pnl: -400.0,
            daily_starting_balance: 10000.0,
        };
        assert_eq!(
            evaluate_pre_trade(&input, Some(&risk())),
            PreTradeCheck::Allow
        );
    }

    #[test]
    fn allows_no_daily_pnl() {
        // No daily PnL, should pass
        let input = PreTradeInput {
            available_balance: 1000.0,
            open_positions: 1,
            order_value: 100.0,
            daily_pnl: 0.0,
            daily_starting_balance: 10000.0,
        };
        assert_eq!(
            evaluate_pre_trade(&input, Some(&risk())),
            PreTradeCheck::Allow
        );
    }

    #[test]
    fn rejects_daily_loss_at_exact_boundary() {
        // Daily loss = 500, which is exactly 5% of 10000 -> should reject (>=)
        let input = PreTradeInput {
            available_balance: 1000.0,
            open_positions: 0,
            order_value: 100.0,
            daily_pnl: -500.0,
            daily_starting_balance: 10000.0,
        };
        match evaluate_pre_trade(&input, Some(&risk())) {
            PreTradeCheck::Reject(msg) => assert!(msg.contains("daily loss limit reached")),
            other => panic!("expected reject, got {other:?}"),
        }
    }
}
