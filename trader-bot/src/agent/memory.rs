use crate::agent::Action;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionRecord {
    pub timestamp: u64,
    pub ticker: String,
    pub action: Action,
    pub conviction: f64,
    pub entry_price: f64,
    pub stop_loss: Option<f64>,
    pub exit_price: Option<f64>,
    pub pnl: Option<f64>,
    pub pnl_pct: Option<f64>,
    pub r_multiple: Option<f64>,
    pub rationale: String,
    pub provider: String,
    pub successful: Option<bool>,
}

impl DecisionRecord {
    pub fn new(
        ticker: &str,
        action: Action,
        conviction: f64,
        entry_price: f64,
        stop_loss: Option<f64>,
        rationale: &str,
        provider: &str,
    ) -> Self {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        DecisionRecord {
            timestamp,
            ticker: ticker.to_string(),
            action,
            conviction,
            entry_price,
            stop_loss,
            exit_price: None,
            pnl: None,
            pnl_pct: None,
            r_multiple: None,
            rationale: rationale.to_string(),
            provider: provider.to_string(),
            successful: None,
        }
    }

    /// Close the trade and compute PnL + R-multiple.
    /// R-multiple = pnl / risk_per_unit, where risk = |entry - stop_loss|.
    pub fn close(&mut self, exit_price: f64) {
        let pnl = match self.action {
            Action::Buy => exit_price - self.entry_price,
            Action::Sell => self.entry_price - exit_price,
            Action::Hold => 0.0,
        };
        self.exit_price = Some(exit_price);
        self.pnl = Some(pnl);
        self.pnl_pct = Some(pnl / self.entry_price * 100.0);
        self.r_multiple = self.stop_loss.map(|sl| {
            let risk = (self.entry_price - sl).abs();
            if risk > 0.0 { pnl / risk } else { 0.0 }
        });
        self.successful = Some(pnl > 0.0);
    }
}

pub struct DecisionMemory {
    records: VecDeque<DecisionRecord>,
    max_records: usize,
    path: Option<PathBuf>,
}

impl DecisionMemory {
    pub fn new(max_records: usize) -> Self {
        DecisionMemory {
            records: VecDeque::with_capacity(max_records),
            max_records,
            path: None,
        }
    }

    pub fn with_persistence(max_records: usize, path: impl Into<PathBuf>) -> Result<Self> {
        let path = path.into();
        let mut mem = DecisionMemory {
            records: VecDeque::with_capacity(max_records),
            max_records,
            path: Some(path.clone()),
        };
        if path.exists() {
            mem.load(&path)?;
        }
        Ok(mem)
    }

    pub fn add(&mut self, record: DecisionRecord) -> Result<()> {
        if self.records.len() >= self.max_records {
            self.records.pop_front();
        }
        self.records.push_back(record);
        if let Some(ref path) = self.path {
            self.save(path)?;
        }
        Ok(())
    }

    pub fn save(&self, path: impl AsRef<Path>) -> Result<()> {
        if let Some(parent) = path.as_ref().parent() {
            std::fs::create_dir_all(parent).context("Failed to create memory directory")?;
        }
        let json = serde_json::to_string_pretty(&self.records)
            .context("Failed to serialize decision memory")?;
        std::fs::write(path.as_ref(), &json)
            .with_context(|| format!("Failed to write memory to {:?}", path.as_ref()))?;
        Ok(())
    }

    pub fn load(&mut self, path: impl AsRef<Path>) -> Result<()> {
        let json = std::fs::read_to_string(path.as_ref())
            .with_context(|| format!("Failed to read memory from {:?}", path.as_ref()))?;
        let records: VecDeque<DecisionRecord> =
            serde_json::from_str(&json).context("Failed to deserialize decision memory")?;
        self.records = records;
        Ok(())
    }

    pub fn records(&self) -> &VecDeque<DecisionRecord> {
        &self.records
    }

    pub fn recent(&self, n: usize) -> Vec<&DecisionRecord> {
        self.records.iter().rev().take(n).collect()
    }

    pub fn win_rate(&self, n: usize) -> f64 {
        let recent: Vec<&DecisionRecord> = self.recent(n);
        let completed: Vec<&&DecisionRecord> =
            recent.iter().filter(|r| r.successful.is_some()).collect();
        if completed.is_empty() {
            return 0.0;
        }
        let wins = completed
            .iter()
            .filter(|r| r.successful == Some(true))
            .count();
        wins as f64 / completed.len() as f64
    }

    pub fn provider_win_rate(&self, provider: &str, n: usize) -> f64 {
        let recent = self.recent(n);
        let from_provider: Vec<&&DecisionRecord> = recent
            .iter()
            .filter(|r| r.provider == provider && r.successful.is_some())
            .collect();
        if from_provider.is_empty() {
            return 0.5;
        }
        let wins = from_provider
            .iter()
            .filter(|r| r.successful == Some(true))
            .count();
        wins as f64 / from_provider.len() as f64
    }

    pub fn avg_profit_pct(&self, n: usize) -> f64 {
        let recent = self.recent(n);
        let with_pnl: Vec<&&DecisionRecord> =
            recent.iter().filter(|r| r.pnl_pct.is_some()).collect();
        if with_pnl.is_empty() {
            return 0.0;
        }
        with_pnl.iter().filter_map(|r| r.pnl_pct).sum::<f64>() / with_pnl.len() as f64
    }

    pub fn total_trades(&self) -> usize {
        self.records.len()
    }

    pub fn avg_r(&self, n: usize) -> f64 {
        let recent = self.recent(n);
        let closed: Vec<&&DecisionRecord> =
            recent.iter().filter(|r| r.r_multiple.is_some()).collect();
        if closed.is_empty() {
            return 0.0;
        }
        closed.iter().filter_map(|r| r.r_multiple).sum::<f64>() / closed.len() as f64
    }

    pub fn total_r(&self, n: usize) -> f64 {
        self.recent(n).iter().filter_map(|r| r.r_multiple).sum()
    }

    /// Profit factor = sum of winning R / sum of losing R (absolute).
    /// Returns 0 if no losing trades.
    pub fn profit_factor(&self, n: usize) -> f64 {
        let recent = self.recent(n);
        let (wins, losses): (f64, f64) =
            recent
                .iter()
                .filter_map(|r| r.r_multiple)
                .fold((0.0, 0.0), |(w, l), r| {
                    if r > 0.0 {
                        (w + r, l)
                    } else {
                        (w, l + r.abs())
                    }
                });
        if losses == 0.0 {
            return wins.max(0.0);
        }
        wins / losses
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_memory_is_empty() {
        let mem = DecisionMemory::new(100);
        assert_eq!(mem.total_trades(), 0);
    }

    #[test]
    fn test_add_record() {
        let mut mem = DecisionMemory::new(100);
        let record = DecisionRecord::new("AAPL", Action::Buy, 0.8, 150.0, None, "bullish", "test");
        mem.add(record).unwrap();
        assert_eq!(mem.total_trades(), 1);
    }

    #[test]
    fn test_max_records_respected() {
        let mut mem = DecisionMemory::new(10);
        for i in 0..15 {
            let record = DecisionRecord::new(
                "AAPL",
                Action::Buy,
                0.8,
                100.0 + i as f64,
                None,
                "test",
                "test",
            );
            mem.add(record).unwrap();
        }
        assert_eq!(mem.total_trades(), 10);
    }

    #[test]
    fn test_persistence_roundtrip() {
        let dir = std::env::temp_dir().join("test_decision_memory");
        let _ = std::fs::remove_dir_all(&dir);
        let path = dir.join("memory.json");

        {
            let mut mem = DecisionMemory::with_persistence(100, &path).unwrap();
            let record =
                DecisionRecord::new("AAPL", Action::Buy, 0.8, 150.0, None, "bullish", "test");
            mem.add(record).unwrap();
        }

        {
            let mem = DecisionMemory::with_persistence(100, &path).unwrap();
            assert_eq!(mem.total_trades(), 1);
            assert_eq!(mem.records()[0].ticker, "AAPL");
        }

        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn test_close_record() {
        let mut record =
            DecisionRecord::new("AAPL", Action::Buy, 0.8, 100.0, None, "bullish", "test");
        record.close(110.0);
        assert_eq!(record.exit_price, Some(110.0));
        assert_eq!(record.pnl, Some(10.0));
        assert_eq!(record.successful, Some(true));
    }

    #[test]
    fn test_close_sell_profit() {
        let mut record =
            DecisionRecord::new("AAPL", Action::Sell, 0.8, 100.0, None, "bearish", "test");
        record.close(90.0);
        assert_eq!(record.exit_price, Some(90.0));
        assert_eq!(record.pnl, Some(10.0));
        assert_eq!(record.successful, Some(true));
    }

    #[test]
    fn test_r_multiple_long() {
        let mut record = DecisionRecord::new(
            "AAPL",
            Action::Buy,
            0.8,
            100.0,
            Some(95.0),
            "bullish",
            "test",
        );
        record.close(110.0);
        assert!((record.r_multiple.unwrap() - 2.0).abs() < 0.01);
    }

    #[test]
    fn test_r_multiple_short() {
        let mut record = DecisionRecord::new(
            "AAPL",
            Action::Sell,
            0.8,
            100.0,
            Some(105.0),
            "bearish",
            "test",
        );
        record.close(90.0);
        assert!((record.r_multiple.unwrap() - 2.0).abs() < 0.01);
    }

    #[test]
    fn test_r_multiple_no_stop_loss() {
        let mut record =
            DecisionRecord::new("AAPL", Action::Buy, 0.8, 100.0, None, "bullish", "test");
        record.close(110.0);
        assert!(record.r_multiple.is_none());
    }

    #[test]
    fn test_win_rate() {
        let mut mem = DecisionMemory::new(100);
        for i in 0..10 {
            let mut record = DecisionRecord::new("AAPL", Action::Buy, 0.8, 100.0, None, "", "test");
            record.close(if i < 6 { 110.0 } else { 90.0 });
            mem.add(record).unwrap();
        }
        assert!((mem.win_rate(10) - 0.6).abs() < 0.01);
    }

    #[test]
    fn test_avg_r() {
        let mut mem = DecisionMemory::new(100);
        for i in 0..10 {
            let mut record =
                DecisionRecord::new("AAPL", Action::Buy, 0.8, 100.0, Some(90.0), "", "test");
            record.close(if i < 6 { 120.0 } else { 85.0 });
            mem.add(record).unwrap();
        }
        // Win: (120-100)/10 = 2R, Loss: (85-100)/10 = -1.5R, avg = (6*2 + 4*(-1.5))/10 = 0.6
        assert!((mem.avg_r(10) - 0.6).abs() < 0.01);
    }

    #[test]
    fn test_profit_factor() {
        let mut mem = DecisionMemory::new(100);
        for _i in 0..5 {
            let mut w =
                DecisionRecord::new("AAPL", Action::Buy, 0.8, 100.0, Some(90.0), "", "test");
            w.close(120.0);
            mem.add(w).unwrap();
            let mut l =
                DecisionRecord::new("AAPL", Action::Buy, 0.8, 100.0, Some(90.0), "", "test");
            l.close(85.0);
            mem.add(l).unwrap();
        }
        // Win R: 2.0, Loss R: -1.5, PF = (5*2) / (5*1.5) = 1.33
        assert!((mem.profit_factor(10) - 1.333).abs() < 0.01);
    }
}
