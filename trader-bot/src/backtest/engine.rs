use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use t_invest_sdk::api::HistoricCandle;
use std::collections::VecDeque;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BacktestConfig {
    pub initial_balance: f64,
    pub commission_pct: f64,
    pub slippage_pct: f64,
    pub max_positions: u32,
    pub max_position_pct: f64,
}

#[derive(Debug, Clone)]
pub struct BacktestTrade {
    pub timestamp: DateTime<Utc>,
    pub ticker: String,
    pub side: String,
    pub price: f64,
    pub quantity: i32,
    pub pnl: f64,
    pub balance_after: f64,
}

#[derive(Debug, Clone)]
pub struct BacktestResult {
    pub total_trades: u32,
    pub winning_trades: u32,
    pub losing_trades: u32,
    pub win_rate: f64,
    pub total_pnl: f64,
    pub max_drawdown: f64,
    pub sharpe_ratio: f64,
    pub final_balance: f64,
    pub total_return_pct: f64,
    pub trades: Vec<BacktestTrade>,
}

pub fn run_backtest(
    candles: &[HistoricCandle],
    config: &BacktestConfig,
    strategy_fn: impl Fn(&[f64], &[f64]) -> f64,
) -> Result<BacktestResult> {
    let closes: Vec<f64> = candles.iter()
        .filter_map(|c| c.close.as_ref())
        .map(|q| q.units as f64 + q.nano as f64 / 1_000_000_000.0)
        .collect();

    if closes.is_empty() {
        anyhow::bail!("No price data for backtest");
    }

    let mut balance = config.initial_balance;
    let mut position = 0.0;
    let mut trades = Vec::new();
    let mut peak_balance = config.initial_balance;
    let mut max_drawdown: f64 = 0.0;
    let mut total_pnl = 0.0;
    let mut winners: u32 = 0;
    let mut losers: u32 = 0;
    let mut returns = VecDeque::new();

    let lookback = 20;

    for i in lookback..closes.len() {
        let window_prices = &closes[i - lookback..=i];
        let window_volumes: Vec<f64> = candles[i - lookback..=i].iter()
            .map(|c| c.volume as f64)
            .collect();

        let signal = strategy_fn(window_prices, &window_volumes);
        let current_price = closes[i];
        let timestamp = candles[i].time
            .as_ref()
            .and_then(|t| DateTime::from_timestamp(t.seconds as i64, 0))
            .unwrap_or(Utc::now());

        if signal > 0.5 && position == 0.0 {
            let max_position_value = balance * config.max_position_pct;
            let quantity = (max_position_value / current_price) as i32;
            let cost = quantity as f64 * current_price;
            let commission = cost * config.commission_pct;
            let fill_price = current_price * (1.0 + config.slippage_pct);

            if cost + commission <= balance {
                balance -= cost + commission;
                position = quantity as f64;
                trades.push(BacktestTrade {
                    timestamp,
                    ticker: String::new(),
                    side: "BUY".to_string(),
                    price: fill_price,
                    quantity,
                    pnl: 0.0,
                    balance_after: balance,
                });
            }
        } else if signal < -0.5 && position > 0.0 {
            let cost = position * current_price;
            let commission = cost * config.commission_pct;
            let fill_price = current_price * (1.0 - config.slippage_pct);
            let pnl = (fill_price - (balance + commission).max(0.0)) + commission;

            balance += cost - commission;
            let trade_pnl = cost - commission - (balance - cost);
            total_pnl += trade_pnl;

            if trade_pnl > 0.0 { winners += 1; } else { losers += 1; }

            trades.push(BacktestTrade {
                timestamp,
                ticker: String::new(),
                side: "SELL".to_string(),
                price: fill_price,
                quantity: position as i32,
                pnl: trade_pnl,
                balance_after: balance,
            });

            position = 0.0;
        }

        peak_balance = peak_balance.max(balance + position * current_price);
        let current_value = balance + position * current_price;
        let drawdown = (peak_balance - current_value) / peak_balance;
        max_drawdown = max_drawdown.max(drawdown);

        if i > lookback + 1 {
            let prev_value = if i > 0 {
                balance + position * closes[i - 1]
            } else {
                config.initial_balance
            };
            let ret = (current_value - prev_value) / prev_value;
            returns.push_back(ret);
        }
    }

    let final_value = balance + position * closes[closes.len() - 1];
    let total_return = (final_value - config.initial_balance) / config.initial_balance;

    let avg_return = if !returns.is_empty() {
        returns.iter().sum::<f64>() / returns.len() as f64
    } else {
        0.0
    };
    let variance = if returns.len() > 1 {
        returns.iter()
            .map(|r| (r - avg_return).powi(2))
            .sum::<f64>() / (returns.len() - 1) as f64
    } else {
        0.0
    };
    let std_dev = variance.sqrt();
    let sharpe_ratio = if std_dev > 0.0 {
        avg_return / std_dev * (252.0_f64).sqrt()
    } else {
        0.0
    };

    let total = trades.len() as u32;

    Ok(BacktestResult {
        total_trades: total,
        winning_trades: winners,
        losing_trades: losers,
        win_rate: if total > 0 { winners as f64 / total as f64 } else { 0.0 },
        total_pnl,
        max_drawdown,
        sharpe_ratio,
        final_balance: final_value,
        total_return_pct: total_return * 100.0,
        trades,
    })
}

pub fn backtest_grid(
    candles: &[HistoricCandle],
    lower_price: f64,
    upper_price: f64,
    grid_levels: u32,
    order_size: u32,
    config: &BacktestConfig,
) -> Result<BacktestResult> {
    let closes: Vec<f64> = candles.iter()
        .filter_map(|c| c.close.as_ref())
        .map(|q| q.units as f64 + q.nano as f64 / 1_000_000_000.0)
        .collect();

    if closes.is_empty() {
        anyhow::bail!("No price data for backtest");
    }

    let step = (upper_price - lower_price) / (grid_levels - 1) as f64;
    let mut balance = config.initial_balance;
    let mut grid_positions: Vec<(f64, f64)> = Vec::new();
    let mut trades = Vec::new();
    let mut peak_balance = config.initial_balance;
    let mut max_drawdown: f64 = 0.0;

    for &price in &closes {
        if price < lower_price || price > upper_price {
            continue;
        }

        let current_value = balance + grid_positions.iter()
            .map(|(buy_price, qty)| qty * (price - buy_price))
            .sum::<f64>();

        peak_balance = peak_balance.max(current_value);
        let drawdown = (peak_balance - current_value) / peak_balance;
        max_drawdown = max_drawdown.max(drawdown);

        // Place buy orders below current price
        let mut level = lower_price;
        while level < price {
            let buy_price = level;
            let cost = order_size as f64 * buy_price;
            let with_commission = cost * (1.0 + config.commission_pct);

            if with_commission <= balance && !grid_positions.iter().any(|(p, _)| (p - buy_price).abs() < 0.01) {
                balance -= with_commission;
                grid_positions.push((buy_price, order_size as f64));
            }
            level += step;
        }

        // Check sell orders
        let mut i = 0;
        while i < grid_positions.len() {
            let (buy_price, qty) = grid_positions[i];
            let sell_price = buy_price + step;
            if price >= sell_price {
                let revenue = qty * sell_price;
                let profit = revenue * (1.0 - config.commission_pct);
                balance += profit;
                grid_positions.swap_remove(i);
            } else {
                i += 1;
            }
        }
    }

    let final_value = balance + grid_positions.iter()
        .map(|(buy_price, qty)| qty * (closes[closes.len() - 1] - buy_price))
        .sum::<f64>();

    let total_return = (final_value - config.initial_balance) / config.initial_balance;

    Ok(BacktestResult {
        total_trades: trades.len() as u32,
        winning_trades: 0,
        losing_trades: 0,
        win_rate: 0.0,
        total_pnl: final_value - config.initial_balance,
        max_drawdown,
        sharpe_ratio: 0.0,
        final_balance: final_value,
        total_return_pct: total_return * 100.0,
        trades,
    })
}
