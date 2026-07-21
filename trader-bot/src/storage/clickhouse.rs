use anyhow::Result;
use chrono::{DateTime, Utc};
use clickhouse::{Client, Row};
use serde::{Deserialize, Serialize};

#[derive(Row, Serialize, Deserialize)]
pub struct CandleRow {
    pub figi: String,
    pub timestamp: DateTime<Utc>,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: i64,
    pub interval: String,
}

#[derive(Row, Serialize, Deserialize)]
pub struct TradeRow {
    pub figi: String,
    pub timestamp: DateTime<Utc>,
    pub price: f64,
    pub quantity: i64,
    pub direction: String,
}

#[derive(Row, Serialize, Deserialize)]
pub struct OrderBookSnapshotRow {
    pub figi: String,
    pub timestamp: DateTime<Utc>,
    pub bid_prices: String,
    pub bid_volumes: String,
    pub ask_prices: String,
    pub ask_volumes: String,
    pub spread: f64,
    pub mid_price: f64,
}

#[derive(Row, Serialize, Deserialize)]
pub struct SignalRow {
    pub timestamp: DateTime<Utc>,
    pub ticker: String,
    pub action: String,
    pub confidence: f64,
    pub price: f64,
    pub reason: String,
    pub strategy: String,
}

pub struct TimeSeriesDb {
    client: Client,
    database: String,
}

impl TimeSeriesDb {
    pub fn new(url: &str, database: &str) -> Self {
        let client = Client::default()
            .with_url(url)
            .with_option("insert_quorum", "2")
            .with_option("select_sequential_consistency", "1");

        TimeSeriesDb {
            client,
            database: database.to_string(),
        }
    }

    async fn exec(&self, sql: String) -> Result<()> {
        self.client.query(&sql).execute().await?;
        Ok(())
    }

    pub async fn initialize(&self) -> Result<()> {
        let db = &self.database;
        self.exec(format!(
            "CREATE DATABASE IF NOT EXISTS {db} ENGINE = Atomic"
        ))
        .await?;

        self.exec(format!(
            "CREATE TABLE IF NOT EXISTS {db}.candles (
                figi String,
                timestamp DateTime64(3, 'UTC'),
                open Float64,
                high Float64,
                low Float64,
                close Float64,
                volume Int64,
                interval String
            ) ENGINE = MergeTree()
            ORDER BY (figi, timestamp)
            PARTITION BY toYYYYMM(timestamp)"
        ))
        .await?;

        self.exec(format!(
            "CREATE TABLE IF NOT EXISTS {db}.trades (
                figi String,
                timestamp DateTime64(3, 'UTC'),
                price Float64,
                quantity Int64,
                direction String
            ) ENGINE = MergeTree()
            ORDER BY (figi, timestamp)
            PARTITION BY toYYYYMM(timestamp)"
        ))
        .await?;

        self.exec(format!(
            "CREATE TABLE IF NOT EXISTS {db}.order_book_snapshots (
                figi String,
                timestamp DateTime64(3, 'UTC'),
                bid_prices String,
                bid_volumes String,
                ask_prices String,
                ask_volumes String,
                spread Float64,
                mid_price Float64
            ) ENGINE = MergeTree()
            ORDER BY (figi, timestamp)"
        ))
        .await?;

        self.exec(format!(
            "CREATE TABLE IF NOT EXISTS {db}.signals (
                timestamp DateTime64(3, 'UTC'),
                ticker String,
                action String,
                confidence Float64,
                price Float64,
                reason String,
                strategy String
            ) ENGINE = MergeTree()
            ORDER BY (ticker, timestamp)"
        ))
        .await?;

        Ok(())
    }

    pub async fn insert_candle(&self, row: CandleRow) -> Result<()> {
        let mut insert = self.client.insert(format!("{}.candles", self.database).as_str())?;
        insert.write(&row).await?;
        insert.end().await?;
        Ok(())
    }

    pub async fn insert_trade(&self, row: TradeRow) -> Result<()> {
        let mut insert = self.client.insert(format!("{}.trades", self.database).as_str())?;
        insert.write(&row).await?;
        insert.end().await?;
        Ok(())
    }

    pub async fn insert_order_book(&self, row: OrderBookSnapshotRow) -> Result<()> {
        let mut insert = self.client.insert(format!("{}.order_book_snapshots", self.database).as_str())?;
        insert.write(&row).await?;
        insert.end().await?;
        Ok(())
    }

    pub async fn insert_signal(&self, row: SignalRow) -> Result<()> {
        let mut insert = self.client.insert(format!("{}.signals", self.database).as_str())?;
        insert.write(&row).await?;
        insert.end().await?;
        Ok(())
    }
}
