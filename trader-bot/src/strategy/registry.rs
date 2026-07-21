use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::core::{Broker, Signal, Strategy, StrategyKind};
use anyhow::Result;

/// Thread-safe registry of trading strategies.
pub struct StrategyRegistry {
    strategies: HashMap<String, Arc<Mutex<Box<dyn Strategy>>>>,
}

impl StrategyRegistry {
    pub fn new() -> Self {
        StrategyRegistry {
            strategies: HashMap::new(),
        }
    }

    pub fn register(&mut self, strategy: Box<dyn Strategy>) {
        let name = strategy.name().to_string();
        self.strategies.insert(name, Arc::new(Mutex::new(strategy)));
    }

    pub fn get(&self, name: &str) -> Option<&Arc<Mutex<Box<dyn Strategy>>>> {
        self.strategies.get(name)
    }

    pub fn get_by_kind(&self, kind: StrategyKind) -> Option<&Arc<Mutex<Box<dyn Strategy>>>> {
        self.strategies.values().find(|s| {
            let guard = s.try_lock();
            guard.map(|g| g.kind() == kind).unwrap_or(false)
        })
    }

    pub fn list_names(&self) -> Vec<String> {
        self.strategies.keys().cloned().collect()
    }

    pub fn count(&self) -> usize {
        self.strategies.len()
    }

    /// Initialize all registered strategies.
    pub async fn init_all(&self, broker: &dyn Broker) -> Result<()> {
        for (name, strategy) in &self.strategies {
            let mut guard = strategy.lock().await;
            log::info!("Initializing strategy: {}", name);
            guard.on_start(broker).await?;
        }
        Ok(())
    }

    /// Analyze an instrument across all strategies.
    pub async fn analyze_all(
        &self,
        broker: &dyn Broker,
        instrument: &str,
    ) -> Result<Vec<(String, Vec<Signal>)>> {
        let mut results = Vec::new();
        for (name, strategy) in &self.strategies {
            let guard = strategy.lock().await;
            match guard.analyze(broker, instrument).await {
                Ok(signals) => results.push((name.clone(), signals)),
                Err(e) => log::warn!("Strategy {} error on {}: {}", name, instrument, e),
            }
        }
        Ok(results)
    }

    /// Tick all strategies.
    pub async fn tick_all(&self, broker: &dyn Broker) -> Result<()> {
        for (name, strategy) in &self.strategies {
            let mut guard = strategy.lock().await;
            if let Err(e) = guard.on_tick(broker).await {
                log::warn!("Strategy {} tick error: {}", name, e);
            }
        }
        Ok(())
    }
}
