use anyhow::Result;
use chrono::{Duration, Utc};
use t_invest_sdk::TInvestSdk;
use t_invest_sdk::api::{CandleInterval, GetCandlesRequest, GetCandlesResponse, HistoricCandle};

/// Service for working with market data
pub struct MarketDataService {
    sdk: TInvestSdk,
}

impl MarketDataService {
    pub fn new(sdk: TInvestSdk) -> Self {
        MarketDataService { sdk }
    }

    /// Get SDK clone
    pub fn sdk_clone(&self) -> TInvestSdk {
        self.sdk.clone()
    }

    /// Get historical candles
    ///
    /// # Arguments
    /// * `instrument_id` - Instrument ID
    /// * `interval` - Candle interval
    /// * `days` - Number of days to load
    ///
    /// # Returns
    /// Vector of candles sorted by time (oldest to newest)
    pub async fn get_historical_candles(
        &self,
        instrument_id: &str,
        interval: CandleInterval,
        days: u32,
    ) -> Result<Vec<HistoricCandle>> {
        let now = Utc::now();
        let from = now - Duration::days(days as i64);

        #[allow(deprecated)]
        let request = GetCandlesRequest {
            instrument_id: Some(instrument_id.to_string()),
            interval: interval as i32,
            from: Some(prost_types::Timestamp {
                seconds: from.timestamp(),
                nanos: from.timestamp_subsec_nanos() as i32,
            }),
            to: Some(prost_types::Timestamp {
                seconds: now.timestamp(),
                nanos: now.timestamp_subsec_nanos() as i32,
            }),
            figi: None,
            limit: None,
            candle_source_type: None,
        };

        let mut client = self.sdk.market_data();
        let response = client.get_candles(request).await?;
        let candles_response: GetCandlesResponse = response.into_inner();

        Ok(candles_response.candles)
    }

    /// Get last candles for N days with 1 minute interval
    pub async fn get_minute_candles(
        &self,
        instrument_id: &str,
        days: u32,
    ) -> Result<Vec<t_invest_sdk::api::HistoricCandle>> {
        self.get_historical_candles(instrument_id, CandleInterval::CandleInterval1Min, days)
            .await
    }

    /// Get last candles for N days with 5 minute interval
    pub async fn get_5min_candles(
        &self,
        instrument_id: &str,
        days: u32,
    ) -> Result<Vec<t_invest_sdk::api::HistoricCandle>> {
        self.get_historical_candles(instrument_id, CandleInterval::CandleInterval5Min, days)
            .await
    }

    /// Get last price by instrument
    pub async fn get_last_price(&self, instrument_id: &str) -> Result<f64> {
        let candles = self.get_minute_candles(instrument_id, 1).await?;

        if let Some(last_candle) = candles.last() {
            return extract_price(&last_candle.close);
        }

        anyhow::bail!("No price data for instrument: {}", instrument_id)
    }
}

/// Extract price from Quotation
pub fn extract_price(quotation: &Option<t_invest_sdk::api::Quotation>) -> Result<f64> {
    match quotation {
        Some(q) => {
            let price = q.units as f64 + (q.nano as f64 / 1_000_000_000.0);
            Ok(price)
        }
        None => Ok(0.0),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_price() {
        let quotation = Some(t_invest_sdk::api::Quotation {
            units: 100,
            nano: 500_000_000,
        });

        let price = extract_price(&quotation).unwrap();
        assert!((price - 100.5).abs() < 0.001);
    }

    #[test]
    fn test_extract_price_zero() {
        let price = extract_price(&None).unwrap();
        assert_eq!(price, 0.0);
    }
}
