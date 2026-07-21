use anyhow::Result;
use chrono::Utc;
use std::collections::HashMap;
use std::sync::Arc;

use crate::backtest::{BacktestConfig, BacktestResult, run_backtest};
use crate::core::*;

/// Strategy function type used by the optimizer.
pub type StrategyFn = Arc<dyn Fn(&[f64], &[f64], &HashMap<String, f64>) -> f64 + Send + Sync>;

/// Parameter optimizer for trading strategies.
pub struct Optimizer {
    config: OptimizerConfig,
    strategy_fn: StrategyFn,
    backtest_config: BacktestConfig,
    candles: Vec<crate::core::Candle>,
}

impl Optimizer {
    pub fn new(
        config: OptimizerConfig,
        strategy_fn: StrategyFn,
        backtest_config: BacktestConfig,
    ) -> Self {
        Optimizer {
            config,
            strategy_fn,
            backtest_config,
            candles: Vec::new(),
        }
    }

    pub fn with_data(mut self, candles: Vec<crate::core::Candle>) -> Self {
        self.candles = candles;
        self
    }

    /// Run the optimization.
    pub async fn optimize(&self) -> Result<OptimizationReport> {
        let start = std::time::Instant::now();

        let trials = match self.config.method {
            OptimizationMethod::GridSearch => self.grid_search()?,
            OptimizationMethod::RandomSearch => self.random_search()?,
        };

        let elapsed = start.elapsed();

        let best = trials
            .iter()
            .max_by(|a, b| {
                a.score
                    .partial_cmp(&b.score)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .cloned()
            .unwrap_or(OptimizationTrial {
                params: HashMap::new(),
                score: 0.0,
                metrics: HashMap::new(),
            });

        Ok(OptimizationReport {
            strategy_name: "optimized".to_string(),
            config: self.config.clone(),
            best_params: best.params,
            best_score: best.score,
            trials,
            total_time_ms: elapsed.as_millis() as u64,
        })
    }

    fn grid_search(&self) -> Result<Vec<OptimizationTrial>> {
        let param_names: Vec<String> = self
            .config
            .param_ranges
            .iter()
            .map(|p| p.name.clone())
            .collect();

        let ranges: Vec<Vec<f64>> = self
            .config
            .param_ranges
            .iter()
            .map(|p| {
                let mut values = Vec::new();
                let mut v = p.min;
                while v <= p.max {
                    values.push(v);
                    v += p.step;
                }
                values
            })
            .collect();

        if ranges.is_empty() {
            anyhow::bail!("No parameter ranges defined");
        }

        let mut trials = Vec::new();

        // Generate all combinations via recursion
        let mut current = vec![0.0; ranges.len()];
        self.grid_recursive(&ranges, &param_names, 0, &mut current, &mut trials);

        Ok(trials)
    }

    fn grid_recursive(
        &self,
        ranges: &[Vec<f64>],
        param_names: &[String],
        depth: usize,
        current: &mut Vec<f64>,
        trials: &mut Vec<OptimizationTrial>,
    ) {
        if depth == ranges.len() {
            let params: HashMap<String, f64> = param_names
                .iter()
                .enumerate()
                .map(|(i, name)| (name.clone(), current[i]))
                .collect();
            let trial = self.evaluate(&params);
            trials.push(trial);
            return;
        }

        for &value in &ranges[depth] {
            current[depth] = value;
            self.grid_recursive(ranges, param_names, depth + 1, current, trials);
        }
    }

    fn random_search(&self) -> Result<Vec<OptimizationTrial>> {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        let mut trials = Vec::new();

        for _ in 0..self.config.max_iterations {
            let params: HashMap<String, f64> = self
                .config
                .param_ranges
                .iter()
                .map(|p| {
                    let value = rng.gen_range(p.min..=p.max);
                    // Round to step
                    let stepped = (value / p.step).round() * p.step;
                    (p.name.clone(), stepped.max(p.min).min(p.max))
                })
                .collect();

            let trial = self.evaluate(&params);
            trials.push(trial);
        }

        Ok(trials)
    }

    fn evaluate(&self, params: &HashMap<String, f64>) -> OptimizationTrial {
        // Convert core::Candle -> backtest format (Vec<f64> closes + Vec<f64> volumes)
        let closes: Vec<f64> = self.candles.iter().map(|c| c.close).collect();
        let volumes: Vec<f64> = self.candles.iter().map(|c| c.volume).collect();

        let strategy_fn = self.strategy_fn.clone();
        let params_clone = params.clone();

        let result = run_backtest_with_fn(
            &closes,
            &volumes,
            &self.backtest_config,
            move |prices, vols| strategy_fn(prices, vols, &params_clone),
        );

        let score = match self.config.metric {
            OptimizationMetric::SharpeRatio => result.sharpe_ratio,
            OptimizationMetric::TotalReturn => result.total_return_pct,
            OptimizationMetric::WinRate => result.win_rate,
            OptimizationMetric::CalmarRatio => {
                if result.max_drawdown > 0.0 {
                    result.total_return_pct / result.max_drawdown
                } else {
                    0.0
                }
            }
            OptimizationMetric::ProfitFactor => {
                if result.losing_trades > 0 {
                    result.winning_trades as f64 / result.losing_trades as f64
                } else {
                    result.winning_trades as f64
                }
            }
        };

        let mut metrics = HashMap::new();
        metrics.insert("sharpe_ratio".to_string(), result.sharpe_ratio);
        metrics.insert("total_return_pct".to_string(), result.total_return_pct);
        metrics.insert("max_drawdown".to_string(), result.max_drawdown);
        metrics.insert("win_rate".to_string(), result.win_rate);
        metrics.insert("total_trades".to_string(), result.total_trades as f64);

        OptimizationTrial {
            params: params.clone(),
            score,
            metrics,
        }
    }
}

/// Run backtest using pre-extracted price data.
fn run_backtest_with_fn(
    closes: &[f64],
    _volumes: &[f64],
    config: &BacktestConfig,
    strategy_fn: impl Fn(&[f64], &[f64]) -> f64,
) -> BacktestResult {
    if closes.is_empty() {
        return BacktestResult {
            total_trades: 0,
            winning_trades: 0,
            losing_trades: 0,
            win_rate: 0.0,
            total_pnl: 0.0,
            max_drawdown: 0.0,
            sharpe_ratio: 0.0,
            final_balance: config.initial_balance,
            total_return_pct: 0.0,
            trades: vec![],
        };
    }

    let mut balance = config.initial_balance;
    let mut position = 0.0;
    let mut trades = Vec::new();
    let mut peak_balance = config.initial_balance;
    let mut max_drawdown: f64 = 0.0;
    let mut total_pnl = 0.0;
    let mut winners: u32 = 0;
    let mut losers: u32 = 0;
    let mut returns = std::collections::VecDeque::new();
    let lookback = 20;

    for i in lookback..closes.len() {
        let window_prices = &closes[i - lookback..=i];
        let _window_volumes: Vec<f64> = _volumes[i - lookback..=i].to_vec();

        let signal = strategy_fn(window_prices, &_window_volumes);
        let current_price = closes[i];

        if signal > 0.5 && position == 0.0 {
            let max_position_value = balance * config.max_position_pct;
            let quantity = (max_position_value / current_price) as i32;
            let cost = quantity as f64 * current_price;
            let commission = cost * config.commission_pct;
            let fill_price = current_price * (1.0 + config.slippage_pct);

            if cost + commission <= balance {
                balance -= cost + commission;
                position = quantity as f64;
                trades.push(crate::backtest::BacktestTrade {
                    timestamp: Utc::now(),
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
            let trade_pnl = position * fill_price - commission - balance;

            balance += cost - commission;
            total_pnl += trade_pnl;

            if trade_pnl > 0.0 {
                winners += 1;
            } else {
                losers += 1;
            }

            trades.push(crate::backtest::BacktestTrade {
                timestamp: Utc::now(),
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
            let prev_value = balance + position * closes[i - 1];
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
        returns
            .iter()
            .map(|r| (r - avg_return).powi(2))
            .sum::<f64>()
            / (returns.len() - 1) as f64
    } else {
        0.0
    };

    let std_dev = variance.sqrt();
    let sharpe_ratio = if std_dev > 0.0 {
        avg_return / std_dev * (252.0_f64).sqrt()
    } else {
        0.0
    };

    BacktestResult {
        total_trades: trades.len() as u32,
        winning_trades: winners,
        losing_trades: losers,
        win_rate: if !trades.is_empty() {
            winners as f64 / trades.len() as f64
        } else {
            0.0
        },
        total_pnl,
        max_drawdown,
        sharpe_ratio,
        final_balance: final_value,
        total_return_pct: total_return * 100.0,
        trades,
    }
}
