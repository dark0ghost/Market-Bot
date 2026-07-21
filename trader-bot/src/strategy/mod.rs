pub mod strategy;
mod interval;
pub mod grid;
pub mod grid_executor;
pub mod grid_bot;
pub mod pairs;
pub mod stat_arb;
pub mod registry;

pub use grid::{GridStrategy, GridState, GridLevel, OrderSide};
pub use grid_executor::{GridExecutor, GridOrderResult, RebalanceResult};
pub use grid_bot::{GridBot, GridBotConfig};
pub use pairs::{PairsTrader, PairConfig, PairSignal, PairAction};
pub use stat_arb::{StatisticalArbitrage, StatArbConfig, StatArbSignal};
pub use strategy::Strategy;
pub use registry::StrategyRegistry;