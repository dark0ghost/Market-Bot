use anyhow::Result;
use async_trait::async_trait;
use chrono::DateTime;
use t_invest_sdk::TInvestSdk;
use t_invest_sdk::api::{FindInstrumentRequest, InstrumentType};

use crate::client::MarketDataService;
use crate::core::*;

/// Tinkoff implementation of DataSource using existing client wrappers.
pub struct TinkoffDataSource {
    sdk: TInvestSdk,
    name: String,
}

impl TinkoffDataSource {
    pub fn new(sdk: TInvestSdk) -> Self {
        TinkoffDataSource {
            sdk,
            name: "tinkoff_data".to_string(),
        }
    }

    fn market_data(&self) -> MarketDataService {
        MarketDataService::new(self.sdk.clone())
    }
}

#[async_trait]
impl DataSource for TinkoffDataSource {
    fn name(&self) -> &str {
        &self.name
    }

    fn source_kind(&self) -> DataSourceKind {
        DataSourceKind::Tinkoff
    }

    async fn candles(
        &self,
        ticker: &str,
        interval: CandleInterval,
        days: u32,
    ) -> Result<Vec<Candle>> {
        use t_invest_sdk::api::CandleInterval as TI;
        let ti = match interval {
            CandleInterval::Min1 => TI::CandleInterval1Min,
            CandleInterval::Min5 => TI::CandleInterval5Min,
            CandleInterval::Min15 => TI::CandleInterval15Min,
            CandleInterval::Hour1 => t_invest_sdk::api::CandleInterval::Hour,
            CandleInterval::Hour4 => TI::CandleInterval4Hour,
            CandleInterval::Day1 => t_invest_sdk::api::CandleInterval::Day,
            _ => t_invest_sdk::api::CandleInterval::Day,
        };

        let hist = self
            .market_data()
            .get_historical_candles(ticker, ti, days)
            .await?;

        Ok(hist
            .iter()
            .filter_map(|c| {
                let time = c
                    .time
                    .as_ref()
                    .and_then(|t| DateTime::from_timestamp(t.seconds, t.nanos as u32))?;
                Some(Candle {
                    open: crate::client::market_data::extract_price(&c.open).ok()?,
                    high: crate::client::market_data::extract_price(&c.high).ok()?,
                    low: crate::client::market_data::extract_price(&c.low).ok()?,
                    close: crate::client::market_data::extract_price(&c.close).ok()?,
                    volume: c.volume as f64,
                    time,
                    ticker: ticker.to_string(),
                })
            })
            .collect())
    }

    async fn find_instrument(&self, query: &str) -> Result<Vec<InstrumentInfo>> {
        let req = FindInstrumentRequest {
            query: query.to_string(),
            instrument_kind: None,
            api_trade_available_flag: None,
        };
        let response = self.sdk.instruments().find_instrument(req).await?;
        let instruments = response.into_inner();

        Ok(instruments
            .instruments
            .iter()
            .map(|i| {
                let kind = match i.instrument_kind() {
                    InstrumentType::Share => InstrumentKind::Share,
                    InstrumentType::Bond => InstrumentKind::Bond,
                    InstrumentType::Etf => InstrumentKind::Etf,
                    InstrumentType::Currency => InstrumentKind::Currency,
                    _ => InstrumentKind::Share,
                };
                InstrumentInfo {
                    ticker: i.ticker.clone(),
                    figi: i.figi.clone(),
                    name: i.name.clone(),
                    currency: String::new(),
                    instrument_type: kind,
                    lot_size: i.lot,
                    min_price_step: 0.01,
                }
            })
            .collect())
    }

    async fn instruments(&self, _kind: Option<InstrumentKind>) -> Result<Vec<InstrumentInfo>> {
        let req = FindInstrumentRequest {
            query: String::new(),
            instrument_kind: Some(InstrumentType::Share as i32),
            api_trade_available_flag: None,
        };
        let response = self.sdk.instruments().find_instrument(req).await?;
        let instruments = response.into_inner();

        Ok(instruments
            .instruments
            .iter()
            .take(100)
            .map(|i| InstrumentInfo {
                ticker: i.ticker.clone(),
                figi: i.figi.clone(),
                name: i.name.clone(),
                currency: String::new(),
                instrument_type: InstrumentKind::Share,
                lot_size: i.lot,
                min_price_step: 0.01,
            })
            .collect())
    }
}
