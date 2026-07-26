pub mod cisd;
pub mod displacement;
pub mod failure_swings;
pub mod fvg;
pub mod smt;
pub mod swings;

pub use cisd::{CISDSignal, detect_cisd};
pub use displacement::{Displacement, detect_displacement};
pub use fvg::{FairValueGap, detect_fvg, update_fvg_states};
pub use swings::{Swing, SwingType, detect_swings};

/// Minimal OHLC candle for detector functions.
/// Pure data — no dependency on t_invest_sdk.
#[derive(Debug, Clone, Copy)]
pub struct Candle {
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: f64,
}
