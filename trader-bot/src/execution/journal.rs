use chrono::{DateTime, Utc};
use serde::Serialize;
use std::fs::{self, File, OpenOptions};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize)]
pub struct TradeRecord {
    pub timestamp: DateTime<Utc>,
    pub ticker: String,
    pub figi: String,
    pub side: String,
    pub quantity: i32,
    pub price: f64,
    pub volume: f64,
    pub commission: f64,
    pub pnl: f64,
    pub strategy: String,
    pub trade_id: String,
    pub order_id: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct SignalRecord {
    pub timestamp: DateTime<Utc>,
    pub ticker: String,
    pub action: String,
    pub confidence: f64,
    pub price: f64,
    pub reason: String,
    pub strategy: String,
}

pub struct TradeJournal {
    trades_file: PathBuf,
    signals_file: PathBuf,
    trades_writer: Option<csv::Writer<File>>,
    signals_writer: Option<csv::Writer<File>>,
}

impl TradeJournal {
    pub fn new(dir: &str) -> Result<Self, anyhow::Error> {
        fs::create_dir_all(dir)?;

        let trades_path = PathBuf::from(dir).join("trades.csv");
        let signals_path = PathBuf::from(dir).join("signals.csv");

        let trades_exists = trades_path.exists();
        let signals_exists = signals_path.exists();

        let trades_writer = csv::Writer::from_writer(
            OpenOptions::new()
                .create(true)
                .append(true)
                .open(&trades_path)?
        );

        let signals_writer = csv::Writer::from_writer(
            OpenOptions::new()
                .create(true)
                .append(true)
                .open(&signals_path)?
        );

        Ok(TradeJournal {
            trades_file: trades_path,
            signals_file: signals_path,
            trades_writer: Some(trades_writer),
            signals_writer: Some(signals_writer),
        })
    }

    pub fn log_trade(&mut self, record: TradeRecord) {
        if let Some(ref mut w) = self.trades_writer {
            if let Err(e) = w.serialize(&record) {
                log::error!("Failed to write trade record: {}", e);
            }
            if let Err(e) = w.flush() {
                log::error!("Failed to flush trade journal: {}", e);
            }
        }
    }

    pub fn log_signal(&mut self, record: SignalRecord) {
        if let Some(ref mut w) = self.signals_writer {
            if let Err(e) = w.serialize(&record) {
                log::error!("Failed to write signal record: {}", e);
            }
            if let Err(e) = w.flush() {
                log::error!("Failed to flush signal journal: {}", e);
            }
        }
    }
}
