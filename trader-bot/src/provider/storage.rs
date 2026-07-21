use anyhow::Result;
use crate::storage::clickhouse::{CandleRow, TradeRow, OrderBookSnapshotRow, SignalRow, TimeSeriesDb};

pub trait StorageProvider {
    async fn initialize(&self) -> Result<()>;

    async fn insert_candle(&self, row: CandleRow) -> Result<()>;

    async fn insert_trade(&self, row: TradeRow) -> Result<()>;

    async fn insert_order_book(&self, row: OrderBookSnapshotRow) -> Result<()>;

    async fn insert_signal(&self, row: SignalRow) -> Result<()>;
}

impl StorageProvider for TimeSeriesDb {
    async fn initialize(&self) -> Result<()> {
        self.initialize().await
    }

    async fn insert_candle(&self, row: CandleRow) -> Result<()> {
        self.insert_candle(row).await
    }

    async fn insert_trade(&self, row: TradeRow) -> Result<()> {
        self.insert_trade(row).await
    }

    async fn insert_order_book(&self, row: OrderBookSnapshotRow) -> Result<()> {
        self.insert_order_book(row).await
    }

    async fn insert_signal(&self, row: SignalRow) -> Result<()> {
        self.insert_signal(row).await
    }
}
