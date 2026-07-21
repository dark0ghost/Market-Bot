use crate::client::MarketDataService;
use crate::client::OrderBookService;
use crate::client::{LiquidityInfo, OrderBook, OrderBookLevel};
use anyhow::Result;
use t_invest_sdk::api::{CandleInterval, HistoricCandle, Quotation};

pub trait MarketDataProvider {
    async fn get_historical_candles(
        &self,
        instrument_id: &str,
        interval: CandleInterval,
        days: u32,
    ) -> Result<Vec<HistoricCandle>>;

    async fn get_last_price(&self, instrument_id: &str) -> Result<f64>;

    async fn get_order_book(&self, figi: &str, depth: i32) -> Result<OrderBook>;

    async fn get_spread(&self, figi: &str) -> Result<f64>;

    async fn get_mid_price(&self, figi: &str) -> Result<f64>;

    async fn get_liquidity(&self, figi: &str, depth: i32) -> Result<LiquidityInfo>;
}

impl MarketDataProvider for MarketDataService {
    async fn get_historical_candles(
        &self,
        instrument_id: &str,
        interval: CandleInterval,
        days: u32,
    ) -> Result<Vec<HistoricCandle>> {
        self.get_historical_candles(instrument_id, interval, days)
            .await
    }

    async fn get_last_price(&self, instrument_id: &str) -> Result<f64> {
        self.get_last_price(instrument_id).await
    }

    async fn get_order_book(&self, figi: &str, depth: i32) -> Result<OrderBook> {
        let ob = OrderBookService::new(self.sdk_clone());
        ob.get_order_book(figi, depth).await
    }

    async fn get_spread(&self, figi: &str) -> Result<f64> {
        let ob = OrderBookService::new(self.sdk_clone());
        ob.get_spread(figi).await
    }

    async fn get_mid_price(&self, figi: &str) -> Result<f64> {
        let ob = OrderBookService::new(self.sdk_clone());
        ob.get_mid_price(figi).await
    }

    async fn get_liquidity(&self, figi: &str, depth: i32) -> Result<LiquidityInfo> {
        let ob = OrderBookService::new(self.sdk_clone());
        ob.get_liquidity(figi, depth).await
    }
}
