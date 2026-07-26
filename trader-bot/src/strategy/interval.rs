use anyhow::Result;
use async_trait::async_trait;

use crate::core::*;

/// Interval strategy placeholder.
pub struct IntervalStrategy;

#[async_trait]
impl Strategy for IntervalStrategy {
    fn name(&self) -> &str {
        "interval"
    }

    fn kind(&self) -> StrategyKind {
        StrategyKind::Interval
    }

    async fn on_start(&mut self, _broker: &dyn Broker) -> Result<()> {
        log::info!("IntervalStrategy: starting");
        Ok(())
    }

    async fn analyze(&self, _broker: &dyn Broker, instrument: &str) -> Result<Vec<Signal>> {
        log::info!("IntervalStrategy: analyzing {}", instrument);
        Ok(Vec::new())
    }

    async fn on_tick(&mut self, _broker: &dyn Broker) -> Result<()> {
        Ok(())
    }

    fn validate(&self) -> Result<()> {
        Ok(())
    }
}
