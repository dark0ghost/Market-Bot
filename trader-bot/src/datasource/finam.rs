use anyhow::{Result, anyhow};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use reqwest::Client;

use crate::core::*;

/// Finam Trade API data source.
///
/// Uses JSON REST API (gRPC-web transcoding).
/// Symbol format: TICKER@MIC (e.g. "SBER@MISX")
pub struct FinamDataSource {
    http: Client,
    base_url: String,
    token: String,
    name: String,
}

#[derive(serde::Deserialize)]
struct AuthResponse {
    token: String,
}

#[derive(serde::Deserialize)]
struct BarsResponse {
    bars: Option<Vec<BarResponse>>,
}

#[derive(serde::Deserialize)]
struct BarResponse {
    timestamp: Option<TimestampResponse>,
    open: Option<DecimalValue>,
    high: Option<DecimalValue>,
    low: Option<DecimalValue>,
    close: Option<DecimalValue>,
    volume: Option<DecimalValue>,
}

#[derive(serde::Deserialize)]
struct TimestampResponse {
    seconds: Option<i64>,
    nanos: Option<i32>,
}

#[derive(serde::Deserialize)]
struct DecimalValue {
    value: Option<String>,
}

#[derive(serde::Deserialize)]
struct AssetsResponse {
    assets: Option<Vec<AssetResponse>>,
}

#[derive(serde::Deserialize)]
struct AssetResponse {
    symbol: Option<String>,
    ticker: Option<String>,
    #[allow(dead_code)]
    mic: Option<String>,
    name: Option<String>,
    r#type: Option<String>,
}

fn parse_decimal(d: &Option<DecimalValue>) -> Option<f64> {
    let v = d.as_ref()?.value.as_ref()?;
    v.parse::<f64>().ok()
}

fn parse_timestamp(ts: &Option<TimestampResponse>) -> Option<DateTime<Utc>> {
    let t = ts.as_ref()?;
    DateTime::from_timestamp(t.seconds.unwrap_or(0), t.nanos.unwrap_or(0) as u32)
}

impl FinamDataSource {
    pub async fn new(api_token: &str, name: Option<String>) -> Result<Self> {
        let http = Client::builder().user_agent("ai-trade-bot/0.2").build()?;

        let base_url = "https://api.finam.ru".to_string();

        let auth_resp: AuthResponse = http
            .post(format!("{}/v1/sessions", base_url))
            .json(&serde_json::json!({"secret": api_token}))
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;

        log::info!("Finam DataSource auth successful");
        let final_name = name.unwrap_or_else(|| "finam_data".to_string());

        Ok(FinamDataSource {
            http,
            base_url,
            token: auth_resp.token,
            name: final_name,
        })
    }

    pub fn new_with_token(_api_token: String, token: String, name: Option<String>) -> Self {
        let http = Client::builder()
            .user_agent("ai-trade-bot/0.2")
            .build()
            .expect("Failed to build HTTP client");

        let base_url = "https://api.finam.ru".to_string();
        let final_name = name.unwrap_or_else(|| "finam_data".to_string());

        FinamDataSource {
            http,
            base_url,
            token,
            name: final_name,
        }
    }

    fn headers(&self) -> reqwest::header::HeaderMap {
        let mut h = reqwest::header::HeaderMap::new();
        if let Ok(val) = format!("Bearer {}", self.token).parse() {
            h.insert(reqwest::header::AUTHORIZATION, val);
        }
        h
    }
}

#[async_trait]
impl DataSource for FinamDataSource {
    fn name(&self) -> &str {
        &self.name
    }

    fn source_kind(&self) -> DataSourceKind {
        DataSourceKind::Finam
    }

    async fn candles(
        &self,
        ticker: &str,
        interval: CandleInterval,
        days: u32,
    ) -> Result<Vec<Candle>> {
        let symbol = if ticker.contains('@') {
            ticker.to_string()
        } else {
            ticker.replace('_', "@")
        };

        let timeframe = match interval {
            CandleInterval::Min1 => "TIME_FRAME_M1",
            CandleInterval::Min5 => "TIME_FRAME_M5",
            CandleInterval::Min15 => "TIME_FRAME_M15",
            CandleInterval::Hour1 => "TIME_FRAME_H1",
            CandleInterval::Hour4 => "TIME_FRAME_H4",
            CandleInterval::Day1 => "TIME_FRAME_D",
            _ => "TIME_FRAME_D",
        };

        let end = Utc::now();
        let start = end - chrono::Duration::days(days as i64);

        let url = format!(
            "{}/v1/instruments/{}/bars?timeframe={}&interval.start_time={}&interval.end_time={}",
            self.base_url,
            symbol,
            timeframe,
            start.format("%Y-%m-%dT%H:%M:%SZ"),
            end.format("%Y-%m-%dT%H:%M:%SZ"),
        );

        let resp: BarsResponse = self
            .http
            .get(&url)
            .headers(self.headers())
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;

        let bars = resp.bars.unwrap_or_default();
        let mut candles = Vec::with_capacity(bars.len());

        for bar in bars {
            let time =
                parse_timestamp(&bar.timestamp).ok_or_else(|| anyhow!("Invalid bar timestamp"))?;
            candles.push(Candle {
                open: parse_decimal(&bar.open).unwrap_or(0.0),
                high: parse_decimal(&bar.high).unwrap_or(0.0),
                low: parse_decimal(&bar.low).unwrap_or(0.0),
                close: parse_decimal(&bar.close).unwrap_or(0.0),
                volume: parse_decimal(&bar.volume).unwrap_or(0.0),
                time,
                ticker: ticker.to_string(),
            });
        }

        Ok(candles)
    }

    async fn find_instrument(&self, query: &str) -> Result<Vec<InstrumentInfo>> {
        let url = format!("{}/v1/assets?search={}", self.base_url, query);

        let resp: AssetsResponse = self
            .http
            .get(&url)
            .headers(self.headers())
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;

        let assets = resp.assets.unwrap_or_default();
        Ok(assets
            .iter()
            .filter_map(|a| {
                let ticker = a.ticker.clone().or_else(|| a.symbol.clone())?;
                let figi = a.symbol.clone().unwrap_or_default();
                let name = a.name.clone().unwrap_or_default();
                let kind = match a.r#type.as_deref() {
                    Some("SHARE") => InstrumentKind::Share,
                    Some("BOND") => InstrumentKind::Bond,
                    Some("ETF") => InstrumentKind::Etf,
                    Some("CURRENCY") => InstrumentKind::Currency,
                    _ => InstrumentKind::Share,
                };
                Some(InstrumentInfo {
                    ticker,
                    figi: figi.replace('@', "_"),
                    name,
                    currency: "RUB".to_string(),
                    instrument_type: kind,
                    lot_size: 1,
                    min_price_step: 0.01,
                })
            })
            .collect())
    }

    async fn instruments(&self, _kind: Option<InstrumentKind>) -> Result<Vec<InstrumentInfo>> {
        let url = format!("{}/v1/assets", self.base_url);

        let resp: AssetsResponse = self
            .http
            .get(&url)
            .headers(self.headers())
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;

        let assets = resp.assets.unwrap_or_default();
        Ok(assets
            .into_iter()
            .filter_map(|a| {
                let ticker = a.ticker.or_else(|| a.symbol.clone())?;
                let figi = a.symbol.unwrap_or_default();
                let name = a.name.unwrap_or_default();
                let kind = match a.r#type.as_deref() {
                    Some("SHARE") => InstrumentKind::Share,
                    Some("BOND") => InstrumentKind::Bond,
                    Some("ETF") => InstrumentKind::Etf,
                    Some("CURRENCY") => InstrumentKind::Currency,
                    _ => InstrumentKind::Share,
                };
                Some(InstrumentInfo {
                    ticker,
                    figi: figi.replace('@', "_"),
                    name,
                    currency: "RUB".to_string(),
                    instrument_type: kind,
                    lot_size: 1,
                    min_price_step: 0.01,
                })
            })
            .collect())
    }
}
