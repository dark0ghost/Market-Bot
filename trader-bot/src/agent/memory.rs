use std::collections::VecDeque;
use std::time::{SystemTime, UNIX_EPOCH};
use crate::agent::Action;

#[derive(Debug, Clone)]
pub struct DecisionRecord {
    pub timestamp: u64,
    pub ticker: String,
    pub action: Action,
    pub conviction: f64,
    pub entry_price: f64,
    pub exit_price: Option<f64>,
    pub pnl: Option<f64>,
    pub pnl_pct: Option<f64>,
    pub rationale: String,
    pub provider: String,
    pub successful: Option<bool>,
}

impl DecisionRecord {
    pub fn new(ticker: &str, action: Action, conviction: f64, entry_price: f64, rationale: &str, provider: &str) -> Self {
        let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs();
        DecisionRecord {
            timestamp,
            ticker: ticker.to_string(),
            action,
            conviction,
            entry_price,
            exit_price: None,
            pnl: None,
            pnl_pct: None,
            rationale: rationale.to_string(),
            provider: provider.to_string(),
            successful: None,
        }
    }

    pub fn close(&mut self, exit_price: f64) {
        let pnl = match self.action {
            Action::Buy => exit_price - self.entry_price,
            Action::Sell => self.entry_price - exit_price,
            Action::Hold => 0.0,
        };
        self.exit_price = Some(exit_price);
        self.pnl = Some(pnl);
        self.pnl_pct = Some(pnl / self.entry_price * 100.0);
        self.successful = Some(pnl > 0.0);
    }
}

pub struct DecisionMemory {
    records: VecDeque<DecisionRecord>,
    max_records: usize,
}

impl DecisionMemory {
    pub fn new(max_records: usize) -> Self {
        DecisionMemory {
            records: VecDeque::with_capacity(max_records),
            max_records,
        }
    }

    pub fn add(&mut self, record: DecisionRecord) {
        if self.records.len() >= self.max_records {
            self.records.pop_front();
        }
        self.records.push_back(record);
    }

    pub fn records(&self) -> &VecDeque<DecisionRecord> {
        &self.records
    }

    pub fn recent(&self, n: usize) -> Vec<&DecisionRecord> {
        self.records.iter().rev().take(n).collect()
    }

    pub fn win_rate(&self, n: usize) -> f64 {
        let recent: Vec<&DecisionRecord> = self.recent(n);
        let completed: Vec<&&DecisionRecord> = recent.iter().filter(|r| r.successful.is_some()).collect();
        if completed.is_empty() {
            return 0.0;
        }
        let wins = completed.iter().filter(|r| r.successful == Some(true)).count();
        wins as f64 / completed.len() as f64
    }

    pub fn provider_win_rate(&self, provider: &str, n: usize) -> f64 {
        let recent = self.recent(n);
        let from_provider: Vec<&&DecisionRecord> = recent.iter()
            .filter(|r| r.provider == provider && r.successful.is_some()).collect();
        if from_provider.is_empty() {
            return 0.5;
        }
        let wins = from_provider.iter().filter(|r| r.successful == Some(true)).count();
        wins as f64 / from_provider.len() as f64
    }

    pub fn avg_profit_pct(&self, n: usize) -> f64 {
        let recent = self.recent(n);
        let with_pnl: Vec<&&DecisionRecord> = recent.iter().filter(|r| r.pnl_pct.is_some()).collect();
        if with_pnl.is_empty() {
            return 0.0;
        }
        with_pnl.iter().filter_map(|r| r.pnl_pct).sum::<f64>() / with_pnl.len() as f64
    }

    pub fn total_trades(&self) -> usize {
        self.records.len()
    }
}
