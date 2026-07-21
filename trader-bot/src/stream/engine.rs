use anyhow::Result;
use chrono::{DateTime, Utc};
use futures::stream::iter;
use std::collections::HashMap;
use t_invest_sdk::api::{
    self, CandleInstrument, MarketDataRequest,
    SubscribeCandlesRequest, SubscriptionAction, SubscriptionInterval,
};
use t_invest_sdk::TInvestSdk;
use tokio::sync::mpsc;
use tonic;

#[derive(Debug, Clone)]
pub enum MarketEvent {
    Candle {
        figi: String,
        time: DateTime<Utc>,
        open: f64,
        high: f64,
        low: f64,
        close: f64,
        volume: i64,
        interval: SubscriptionInterval,
    },
    OrderBook {
        figi: String,
        time: DateTime<Utc>,
        bids: Vec<(f64, i64)>,
        asks: Vec<(f64, i64)>,
    },
    Trade {
        figi: String,
        time: DateTime<Utc>,
        price: f64,
        quantity: i64,
        direction: String,
    },
    LastPrice {
        figi: String,
        price: f64,
        time: DateTime<Utc>,
    },
    Ping {
        time: DateTime<Utc>,
    },
}

pub struct MarketDataStream {
    sdk: TInvestSdk,
    event_tx: mpsc::Sender<MarketEvent>,
    subscriptions: HashMap<String, Vec<SubscriptionInterval>>,
}

impl MarketDataStream {
    pub fn new(sdk: TInvestSdk) -> (Self, mpsc::Receiver<MarketEvent>) {
        let (tx, rx) = mpsc::channel(1024);
        (
            MarketDataStream {
                sdk,
                event_tx: tx,
                subscriptions: HashMap::new(),
            },
            rx,
        )
    }

    pub async fn subscribe_candles(
        &mut self,
        figi: &str,
        interval: SubscriptionInterval,
    ) -> Result<()> {
        self.subscriptions
            .entry(figi.to_string())
            .or_default()
            .push(interval);
        Ok(())
    }

    pub async fn run(&mut self) -> Result<()> {
        let mut client = self.sdk.market_data_stream();
        let mut subs_candles = SubscribeCandlesRequest {
            subscription_action: SubscriptionAction::Subscribe as i32,
            instruments: vec![],
            waiting_close: false,
            candle_source_type: None,
        };

        for (figi, intervals) in &self.subscriptions {
            for &interval in intervals {
                subs_candles.instruments.push(CandleInstrument {
                    figi: figi.clone(),
                    interval: interval as i32,
                    instrument_id: figi.clone(),
                });
            }
        }

        let request = tonic::Request::new(iter(vec![
            MarketDataRequest {
                payload: Some(
                    t_invest_sdk::api::market_data_request::Payload::SubscribeCandlesRequest(
                        subs_candles,
                    ),
                ),
            },
            MarketDataRequest {
                payload: Some(
                    t_invest_sdk::api::market_data_request::Payload::PingSettings(
                        api::PingDelaySettings {
                            ping_delay_ms: Some(30000),
                        },
                    ),
                ),
            },
        ]));

        let response = client.market_data_stream(request).await?;
        let stream = response.into_inner();
        let tx = self.event_tx.clone();

        tokio::spawn(async move {
            use tokio_stream::StreamExt;

            let mut stream = stream;

            while let Some(item) = stream.next().await {
                match item {
                    Ok(response) => {
                        if let Some(payload) = response.payload {
                            handle_market_response(payload, &tx).await;
                        }
                    }
                    Err(e) => {
                        log::error!("Market data stream error: {}", e);
                        break;
                    }
                }
            }
        });

        Ok(())
    }
}

async fn handle_market_response(
    payload: t_invest_sdk::api::market_data_response::Payload,
    tx: &mpsc::Sender<MarketEvent>,
) {
    use t_invest_sdk::api::market_data_response::Payload;

    match payload {
        Payload::Candle(candle) => {
            let event = MarketEvent::Candle {
                figi: candle.figi,
                time: candle
                    .time
                    .and_then(|t| DateTime::from_timestamp(t.seconds as i64, 0))
                    .unwrap_or(Utc::now()),
                open: candle
                    .open
                    .as_ref()
                    .map(|q| q.units as f64 + q.nano as f64 / 1_000_000_000.0)
                    .unwrap_or(0.0),
                high: candle
                    .high
                    .as_ref()
                    .map(|q| q.units as f64 + q.nano as f64 / 1_000_000_000.0)
                    .unwrap_or(0.0),
                low: candle
                    .low
                    .as_ref()
                    .map(|q| q.units as f64 + q.nano as f64 / 1_000_000_000.0)
                    .unwrap_or(0.0),
                close: candle
                    .close
                    .as_ref()
                    .map(|q| q.units as f64 + q.nano as f64 / 1_000_000_000.0)
                    .unwrap_or(0.0),
                volume: candle.volume,
                interval: SubscriptionInterval::try_from(candle.interval)
                    .unwrap_or(SubscriptionInterval::Unspecified),
            };
            let _ = tx.try_send(event);
        }
        Payload::Orderbook(book) => {
            let event = MarketEvent::OrderBook {
                figi: book.figi,
                time: book
                    .time
                    .and_then(|t| DateTime::from_timestamp(t.seconds as i64, 0))
                    .unwrap_or(Utc::now()),
                bids: book
                    .bids
                    .iter()
                    .map(|l| {
                        let price = l
                            .price
                            .as_ref()
                            .map(|q| q.units as f64 + q.nano as f64 / 1_000_000_000.0)
                            .unwrap_or(0.0);
                        (price, l.quantity)
                    })
                    .collect(),
                asks: book
                    .asks
                    .iter()
                    .map(|l| {
                        let price = l
                            .price
                            .as_ref()
                            .map(|q| q.units as f64 + q.nano as f64 / 1_000_000_000.0)
                            .unwrap_or(0.0);
                        (price, l.quantity)
                    })
                    .collect(),
            };
            let _ = tx.try_send(event);
        }
        Payload::Trade(trade) => {
            let event = MarketEvent::Trade {
                figi: trade.figi,
                time: trade
                    .time
                    .and_then(|t| DateTime::from_timestamp(t.seconds as i64, 0))
                    .unwrap_or(Utc::now()),
                price: trade
                    .price
                    .as_ref()
                    .map(|q| q.units as f64 + q.nano as f64 / 1_000_000_000.0)
                    .unwrap_or(0.0),
                quantity: trade.quantity,
                direction: format!("{:?}", trade.direction),
            };
            let _ = tx.try_send(event);
        }
        Payload::LastPrice(last) => {
            let event = MarketEvent::LastPrice {
                figi: last.figi,
                price: last
                    .price
                    .as_ref()
                    .map(|q| q.units as f64 + q.nano as f64 / 1_000_000_000.0)
                    .unwrap_or(0.0),
                time: last
                    .time
                    .and_then(|t| DateTime::from_timestamp(t.seconds as i64, 0))
                    .unwrap_or(Utc::now()),
            };
            let _ = tx.try_send(event);
        }
        Payload::Ping(ping) => {
            let event = MarketEvent::Ping {
                time: ping
                    .time
                    .and_then(|t| DateTime::from_timestamp(t.seconds as i64, 0))
                    .unwrap_or(Utc::now()),
            };
            let _ = tx.try_send(event);
        }
        _ => {}
    }
}
