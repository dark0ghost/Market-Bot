# API Documentation

Documentation for the AI Trade Bot internal API.

## Modules

### Strategy Module

#### GridStrategy

```rust
pub struct GridStrategy {
    config: GridConfig,
}

impl GridStrategy {
    pub fn new(config: GridConfig) -> Self;
    pub fn calculate_grid_levels(&self) -> Vec<GridLevel>;
    pub fn get_levels_to_place(&self, current_price: f64) -> Vec<GridLevel>;
    pub fn needs_rebalance(&self, state: &GridState, current_price: f64, price_threshold: f64) -> bool;
}
```

#### GridLevel

```rust
pub struct GridLevel {
    pub price: f64,
    pub order_type: OrderSide,
    pub level_index: u32,
}

pub enum OrderSide {
    Buy,
    Sell,
}
```

### Execution Module

#### GridExecutor

```rust
pub struct GridExecutor {
    position_manager: PositionManager,
    grid_strategy: GridStrategy,
    grid_state: Option<GridState>,
    figi: String,
    order_size: u32,
}

impl GridExecutor {
    pub fn new(sdk: TInvestSdk, account_id: String, grid_strategy: GridStrategy, figi: String) -> Self;
    pub async fn initialize_grid(&mut self, current_price: f64) -> Result<Vec<GridOrderResult>>;
    pub async fn rebalance_grid(&mut self, current_price: f64) -> Result<RebalanceResult>;
    pub async fn on_order_filled(&mut self, level_index: u32) -> Result<()>;
    pub async fn stop_grid(&mut self) -> Result<()>;
}
```

### Analysis Module

#### TechnicalAnalyzer

```rust
pub struct TechnicalAnalyzer {
    rsi_period: usize,
    macd_fast: usize,
    macd_slow: usize,
    macd_signal: usize,
    bollinger_period: usize,
    bollinger_std_dev: f64,
}

impl TechnicalAnalyzer {
    pub fn new() -> Self;
    pub fn analyze(&self, ticker: &str, candles: &[HistoricCandle]) -> Result<TechnicalAnalysis>;
}
```

#### TechnicalAnalysis

```rust
pub struct TechnicalAnalysis {
    pub ticker: String,
    pub timestamp: DateTime<Utc>,
    pub current_price: f64,
    pub trend: Trend,
    pub rsi: Option<f64>,
    pub macd: Option<MacdValues>,
    pub bollinger: Option<BollingerValues>,
    pub volume_analysis: VolumeAnalysis,
    pub support_levels: Vec<f64>,
    pub resistance_levels: Vec<f64>,
    pub recommendation: Recommendation,
}
```

### Client Module

#### MarketDataService

```rust
pub struct MarketDataService {
    sdk: TInvestSdk,
}

impl MarketDataService {
    pub fn new(sdk: TInvestSdk) -> Self;
    pub async fn get_historical_candles(&self, instrument_id: &str, interval: CandleInterval, days: u32) -> Result<Vec<HistoricCandle>>;
    pub async fn get_5min_candles(&self, instrument_id: &str, days: u32) -> Result<Vec<HistoricCandle>>;
    pub async fn get_last_price(&self, instrument_id: &str) -> Result<f64>;
}
```

#### PortfolioService

```rust
pub struct PortfolioService {
    sdk: TInvestSdk,
    account_id: String,
}

impl PortfolioService {
    pub fn new(sdk: TInvestSdk, account_id: String) -> Self;
    pub async fn get_accounts(&self) -> Result<Vec<AccountInfo>>;
    pub async fn get_portfolio(&self) -> Result<PortfolioInfo>;
    pub async fn get_available_balance(&self) -> Result<f64>;
    pub async fn get_position(&self, instrument_uid: &str) -> Result<Option<CurrentPosition>>;
}
```

## Configuration

### GridConfig

```rust
pub struct GridConfig {
    pub lower_price: f64,
    pub upper_price: f64,
    pub grid_levels: u32,
    pub order_size: u32,
    pub grid_ratio: f64,
}
```

### RiskManagementConfig

```rust
pub struct RiskManagementConfig {
    pub max_loss_pct: f64,
    pub take_profit_pct: f64,
    pub stop_loss_pct: f64,
    pub max_open_positions: u32,
    pub min_balance_reserve: f64,
}
```

## Usage Examples

### Creating a Grid Strategy

```rust
use strategy::{GridStrategy, GridConfig};

let config = GridConfig {
    lower_price: 250.0,
    upper_price: 300.0,
    grid_levels: 11,
    order_size: 10,
    grid_ratio: 0.5,
};

let strategy = GridStrategy::new(config);
let levels = strategy.calculate_grid_levels();
```

### Running GridExecutor

```rust
use strategy::{GridStrategy, GridExecutor};
use t_invest_sdk::TInvestSdk;

let sdk = TInvestSdk::new(&token, Environment::Sandbox).await?;
let strategy = GridStrategy::new(grid_config);
let mut executor = GridExecutor::new(sdk, "account_id".to_string(), strategy, "FIGI".to_string());

let current_price = 275.50;
let results = executor.initialize_grid(current_price).await?;
```

### Technical Analysis

```rust
use analysis::TechnicalAnalyzer;

let analyzer = TechnicalAnalyzer::new();
let analysis = analyzer.analyze("TTECH", &candles)?;

println!("Trend: {:?}", analysis.trend);
println!("Recommendation: {:?}", analysis.recommendation);
println!("RSI: {:?}", analysis.rsi);
```

## Error Handling

All API methods return `Result<T, E>` where `E` is `anyhow::Error`.

```rust
use anyhow::Result;

pub async fn some_operation(&self) -> Result<()> {
    // May return an error
    Err(anyhow::anyhow!("Operation error"))
}
```

## Logging

Uses the `log` crate:

```rust
use log::{info, warn, error, debug};

info!("Informational message");
warn!("Warning message");
error!("Error message");
debug!("Debug message");
```

## Testing

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_grid_calculation() {
        let config = GridConfig {
            lower_price: 100.0,
            upper_price: 200.0,
            grid_levels: 11,
            order_size: 10,
            grid_ratio: 0.5,
        };

        let strategy = GridStrategy::new(config);
        let levels = strategy.calculate_grid_levels();

        assert_eq!(levels.len(), 11);
        assert_eq!(levels[0].price, 100.0);
        assert_eq!(levels[10].price, 200.0);
    }
}
```
