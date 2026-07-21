use anyhow::Result;
use t_invest_sdk::TInvestSdk;
use t_invest_sdk::api::GetOrderBookRequest;

#[derive(Debug, Clone)]
pub struct OrderBookLevel {
    pub price: f64,
    pub quantity: i64,
}

#[derive(Debug, Clone)]
pub struct OrderBook {
    pub figi: String,
    pub depth: i32,
    pub bids: Vec<OrderBookLevel>,
    pub asks: Vec<OrderBookLevel>,
    pub spread: f64,
    pub mid_price: f64,
}

pub struct OrderBookService {
    sdk: TInvestSdk,
}

impl OrderBookService {
    pub fn new(sdk: TInvestSdk) -> Self {
        OrderBookService { sdk }
    }

    pub async fn get_order_book(&self, figi: &str, depth: i32) -> Result<OrderBook> {
        let request = GetOrderBookRequest {
            figi: Some(figi.to_string()),
            depth,
            instrument_id: Some(figi.to_string()),
        };

        let mut client = self.sdk.market_data();
        let response = client.get_order_book(request).await?;
        let book = response.into_inner();

        let bids: Vec<OrderBookLevel> = book
            .bids
            .into_iter()
            .map(|l| OrderBookLevel {
                price: l
                    .price
                    .as_ref()
                    .map(|q| q.units as f64 + q.nano as f64 / 1_000_000_000.0)
                    .unwrap_or(0.0),
                quantity: l.quantity,
            })
            .collect();

        let asks: Vec<OrderBookLevel> = book
            .asks
            .into_iter()
            .map(|l| OrderBookLevel {
                price: l
                    .price
                    .as_ref()
                    .map(|q| q.units as f64 + q.nano as f64 / 1_000_000_000.0)
                    .unwrap_or(0.0),
                quantity: l.quantity,
            })
            .collect();

        let best_bid = bids.first().map(|l| l.price).unwrap_or(0.0);
        let best_ask = asks.first().map(|l| l.price).unwrap_or(0.0);
        let spread = if best_bid > 0.0 && best_ask > 0.0 {
            best_ask - best_bid
        } else {
            0.0
        };
        let mid_price = if best_bid > 0.0 && best_ask > 0.0 {
            (best_bid + best_ask) / 2.0
        } else {
            0.0
        };

        Ok(OrderBook {
            figi: figi.to_string(),
            depth,
            bids,
            asks,
            spread,
            mid_price,
        })
    }

    /// Получить только спред (быстрый запрос с depth=1)
    pub async fn get_spread(&self, figi: &str) -> Result<f64> {
        let book = self.get_order_book(figi, 1).await?;
        Ok(book.spread)
    }

    /// Получить среднюю цену (mid-price)
    pub async fn get_mid_price(&self, figi: &str) -> Result<f64> {
        let book = self.get_order_book(figi, 1).await?;
        Ok(book.mid_price)
    }

    /// Получить ликвидность на N уровней
    pub async fn get_liquidity(&self, figi: &str, depth: i32) -> Result<LiquidityInfo> {
        let book = self.get_order_book(figi, depth).await?;

        let bid_liquidity: f64 = book.bids.iter().map(|l| l.price * l.quantity as f64).sum();

        let ask_liquidity: f64 = book.asks.iter().map(|l| l.price * l.quantity as f64).sum();

        Ok(LiquidityInfo {
            figi: figi.to_string(),
            bid_liquidity,
            ask_liquidity,
            imbalance: if bid_liquidity + ask_liquidity > 0.0 {
                (bid_liquidity - ask_liquidity) / (bid_liquidity + ask_liquidity)
            } else {
                0.0
            },
        })
    }
}

#[derive(Debug, Clone)]
pub struct LiquidityInfo {
    pub figi: String,
    pub bid_liquidity: f64,
    pub ask_liquidity: f64,
    pub imbalance: f64,
}
