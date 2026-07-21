use anyhow::Result;
use std::collections::HashMap;
use std::sync::Arc;

use crate::core::{DataSource, DataSourceKind, InstrumentInfo};

/// Registry of available data sources.
pub struct DataSourceRegistry {
    sources: HashMap<String, Arc<dyn DataSource>>,
}

impl DataSourceRegistry {
    pub fn new() -> Self {
        DataSourceRegistry {
            sources: HashMap::new(),
        }
    }

    pub fn register(&mut self, source: Arc<dyn DataSource>) {
        let name = source.name().to_string();
        self.sources.insert(name, source);
    }

    pub fn get(&self, name: &str) -> Option<&Arc<dyn DataSource>> {
        self.sources.get(name)
    }

    pub fn get_by_kind(&self, kind: DataSourceKind) -> Option<&Arc<dyn DataSource>> {
        self.sources.values().find(|s| s.source_kind() == kind)
    }

    pub fn all(&self) -> Vec<&Arc<dyn DataSource>> {
        self.sources.values().collect()
    }

    pub fn list_names(&self) -> Vec<String> {
        self.sources.keys().cloned().collect()
    }

    /// Fetch candles from all sources (returns first successful).
    pub async fn fetch_candles_any(
        &self,
        ticker: &str,
        interval: crate::core::CandleInterval,
        days: u32,
    ) -> Result<Vec<crate::core::Candle>> {
        for source in self.sources.values() {
            if let Ok(candles) = source.candles(ticker, interval, days).await {
                if !candles.is_empty() {
                    return Ok(candles);
                }
            }
        }
        anyhow::bail!("No data source returned candles for {}", ticker)
    }
}
