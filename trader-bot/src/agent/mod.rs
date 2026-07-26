pub mod calibration;
pub mod memory;
pub mod multi_agent;
pub mod trading_agent;

pub use calibration::PredictionTracker;
pub use memory::{DecisionMemory, DecisionRecord};
pub use trading_agent::{
    Action, CurrentPosition, DecisionContext, LlmQuery, OllamaQuery, TimeHorizon, TradingAgent,
    TradingDecision,
};
