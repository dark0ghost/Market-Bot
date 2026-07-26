use crate::agent::{Action, DecisionMemory, DecisionRecord};
use std::sync::{Arc, RwLock};

#[derive(Debug, Clone)]
pub struct TrackedPosition {
    pub ticker: String,
    pub entry_price: f64,
    pub quantity: f64,
    pub action: Action,
    pub stop_loss: Option<f64>,
    pub take_profit: Option<f64>,
}

#[derive(Debug, Clone)]
pub struct ClosedPosition {
    pub ticker: String,
    pub exit_price: f64,
    pub reason: ExitReason,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ExitReason {
    StopLoss,
    TakeProfit,
}

pub struct PositionTracker {
    positions: Vec<TrackedPosition>,
    memory: Option<Arc<RwLock<DecisionMemory>>>,
}

impl PositionTracker {
    pub fn new(memory: Option<Arc<RwLock<DecisionMemory>>>) -> Self {
        PositionTracker {
            positions: Vec::new(),
            memory,
        }
    }

    pub fn open(
        &mut self,
        ticker: &str,
        entry_price: f64,
        quantity: f64,
        action: Action,
        stop_loss: Option<f64>,
        take_profit: Option<f64>,
    ) {
        self.positions.push(TrackedPosition {
            ticker: ticker.to_string(),
            entry_price,
            quantity,
            action,
            stop_loss,
            take_profit,
        });
    }

    /// Wick-aware exit check. Returns positions closed this tick.
    /// SL takes priority if both SL and TP hit in the same candle.
    pub fn check_candle(
        &mut self,
        ticker: &str,
        high: f64,
        low: f64,
        _close: f64,
    ) -> Vec<ClosedPosition> {
        let mut closed = Vec::new();
        let mut to_remove = Vec::new();

        for (i, pos) in self.positions.iter().enumerate() {
            if pos.ticker != ticker {
                continue;
            }
            let decision = match pos.action {
                Action::Buy => {
                    let sl_hit = pos.stop_loss.map(|sl| low <= sl).unwrap_or(false);
                    let tp_hit = pos.take_profit.map(|tp| high >= tp).unwrap_or(false);
                    if sl_hit {
                        Some((ExitReason::StopLoss, pos.stop_loss.unwrap()))
                    } else if tp_hit {
                        Some((ExitReason::TakeProfit, pos.take_profit.unwrap()))
                    } else {
                        None
                    }
                }
                Action::Sell => {
                    let sl_hit = pos.stop_loss.map(|sl| high >= sl).unwrap_or(false);
                    let tp_hit = pos.take_profit.map(|tp| low <= tp).unwrap_or(false);
                    if sl_hit {
                        Some((ExitReason::StopLoss, pos.stop_loss.unwrap()))
                    } else if tp_hit {
                        Some((ExitReason::TakeProfit, pos.take_profit.unwrap()))
                    } else {
                        None
                    }
                }
                Action::Hold => None,
            };
            if let Some((reason, exit_price)) = decision {
                closed.push(ClosedPosition {
                    ticker: ticker.to_string(),
                    exit_price,
                    reason,
                });
                to_remove.push(i);
            }
        }

        for pos in &closed {
            self.record_close(&pos.ticker, pos.exit_price);
        }

        for i in to_remove.into_iter().rev() {
            self.positions.swap_remove(i);
        }

        closed
    }

    pub fn positions(&self) -> &[TrackedPosition] {
        &self.positions
    }

    pub fn has_position(&self, ticker: &str) -> bool {
        self.positions.iter().any(|p| p.ticker == ticker)
    }

    fn record_close(&self, ticker: &str, exit_price: f64) {
        if let Some(ref memory) = self.memory
            && let Ok(mut mem) = memory.write()
        {
            let mut to_close: Vec<DecisionRecord> = mem
                .records()
                .iter()
                .filter(|r| r.ticker == ticker && r.successful.is_none())
                .cloned()
                .collect();
            if let Some(ref mut rec) = to_close.last_mut() {
                rec.close(exit_price);
                let _ = mem.add(rec.clone());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_tracker() -> PositionTracker {
        PositionTracker::new(None)
    }

    #[test]
    fn test_long_sl_hit_by_low() {
        let mut tracker = make_tracker();
        tracker.open("AAPL", 100.0, 10.0, Action::Buy, Some(95.0), Some(110.0));
        let closed = tracker.check_candle("AAPL", 102.0, 94.0, 101.0);
        assert_eq!(closed.len(), 1);
        assert_eq!(closed[0].reason, ExitReason::StopLoss);
    }

    #[test]
    fn test_long_tp_hit_by_high() {
        let mut tracker = make_tracker();
        tracker.open("AAPL", 100.0, 10.0, Action::Buy, Some(95.0), Some(110.0));
        let closed = tracker.check_candle("AAPL", 112.0, 99.0, 111.0);
        assert_eq!(closed.len(), 1);
        assert_eq!(closed[0].reason, ExitReason::TakeProfit);
    }

    #[test]
    fn test_long_sl_priority_over_tp() {
        let mut tracker = make_tracker();
        tracker.open("AAPL", 100.0, 10.0, Action::Buy, Some(95.0), Some(110.0));
        let closed = tracker.check_candle("AAPL", 111.0, 94.0, 105.0);
        assert_eq!(closed.len(), 1);
        assert_eq!(closed[0].reason, ExitReason::StopLoss);
    }

    #[test]
    fn test_short_sl_hit_by_high() {
        let mut tracker = make_tracker();
        tracker.open("AAPL", 100.0, 10.0, Action::Sell, Some(105.0), Some(90.0));
        let closed = tracker.check_candle("AAPL", 106.0, 98.0, 104.0);
        assert_eq!(closed.len(), 1);
        assert_eq!(closed[0].reason, ExitReason::StopLoss);
    }

    #[test]
    fn test_short_tp_hit_by_low() {
        let mut tracker = make_tracker();
        tracker.open("AAPL", 100.0, 10.0, Action::Sell, Some(105.0), Some(90.0));
        let closed = tracker.check_candle("AAPL", 101.0, 88.0, 89.0);
        assert_eq!(closed.len(), 1);
        assert_eq!(closed[0].reason, ExitReason::TakeProfit);
    }

    #[test]
    fn test_no_hit_keeps_position() {
        let mut tracker = make_tracker();
        tracker.open("AAPL", 100.0, 10.0, Action::Buy, Some(95.0), Some(110.0));
        let closed = tracker.check_candle("AAPL", 105.0, 98.0, 103.0);
        assert!(closed.is_empty());
        assert!(tracker.has_position("AAPL"));
    }

    #[test]
    fn test_different_ticker_ignored() {
        let mut tracker = make_tracker();
        tracker.open("AAPL", 100.0, 10.0, Action::Buy, Some(95.0), Some(110.0));
        let closed = tracker.check_candle("GOOG", 120.0, 80.0, 100.0);
        assert!(closed.is_empty());
        assert!(tracker.has_position("AAPL"));
    }

    #[test]
    fn test_close_removes_position() {
        let mut tracker = make_tracker();
        tracker.open("AAPL", 100.0, 10.0, Action::Buy, Some(95.0), Some(110.0));
        tracker.check_candle("AAPL", 102.0, 94.0, 100.0);
        assert!(!tracker.has_position("AAPL"));
    }
}
