# Grid Bot

Grid Bot automatically places buy and sell orders on a predefined price grid. The strategy performs best in sideways (flat) markets.

## How It Works

### Grid Visualization

```
Price (RUB)
  300 -----------------------+ Sell 5
  295 --------------------+  | Sell 4
  290 ------------------+  |  | Sell 3
  285 ----------------+  |  |  | Sell 2
  280 -------------+  |  |  |  | Sell 1
  275 -------------+--+--+--+--+-- Current price
  270 -------------+  |  |  |  | Buy 1
  265 ----------------+  |  |  | Buy 2
  260 ------------------+  |  | Buy 3
  255 --------------------+  | Buy 4
  250 -----------------------+ Buy 5
       |<- step = 5 RUB ->|
```

### Algorithm

1. **Initialize**: Calculate grid levels
2. **Place**: Buy orders below price, Sell above
3. **Execute**: On buy, place a Sell order for profit
4. **Rebalance**: Update on 2%+ price change

## Configuration

### Basic Setup

```json
{
  "accounts": [{
    "account_id": "grid_sber",
    "strategy": {
      "strategy": "grid",
      "parameters": {
        "grid_config": {
          "lower_price": 250.0,
          "upper_price": 300.0,
          "grid_levels": 11,
          "order_size": 10,
          "grid_ratio": 0.5
        }
      }
    },
    "instruments": [{
      "figi": "BBG004730N88",
      "ticker": "SBER",
      "enabled": true
    }]
  }]
}
```

### GridConfig Parameters

| Parameter | Type | Description | Example |
|-----------|------|-------------|---------|
| `lower_price` | f64 | Lower range bound | 250.0 |
| `upper_price` | f64 | Upper range bound | 300.0 |
| `grid_levels` | u32 | Number of levels | 11 |
| `order_size` | u32 | Order size in lots | 10 |
| `grid_ratio` | f64 | Buy/sell ratio | 0.5 |

### Parameter Calculation

```python
# Grid step
step = (upper_price - lower_price) / (grid_levels - 1)
# For SBER: (300 - 250) / (11 - 1) = 5 RUB

# Number of Buy levels
buy_levels = int(grid_levels * grid_ratio)
# For grid_ratio=0.5: 11 * 0.5 = 5 levels

# Number of Sell levels
sell_levels = grid_levels - buy_levels
# 11 - 5 = 6 levels
```

## Configuration Examples

### Conservative (Wide Range)

```json
{
  "grid_config": {
    "lower_price": 200.0,
    "upper_price": 350.0,
    "grid_levels": 31,
    "order_size": 5,
    "grid_ratio": 0.5
  }
}
```

**Characteristics:**
- Step: 5 RUB
- Buy levels: 15
- Sell levels: 16
- Required capital: ~7,500 RUB per level

### Aggressive (Narrow Range)

```json
{
  "grid_config": {
    "lower_price": 270.0,
    "upper_price": 280.0,
    "grid_levels": 21,
    "order_size": 20,
    "grid_ratio": 0.5
  }
}
```

**Characteristics:**
- Step: 0.5 RUB
- Buy levels: 10
- Sell levels: 11
- Required capital: ~5,400 RUB per level

### For Expensive Stocks (LKOH)

```json
{
  "grid_config": {
    "lower_price": 6000.0,
    "upper_price": 7000.0,
    "grid_levels": 11,
    "order_size": 1,
    "grid_ratio": 0.5
  }
}
```

**Characteristics:**
- Step: 100 RUB
- Buy levels: 5
- Sell levels: 6
- Required capital: ~6,500 RUB per level

## Range Selection Recommendations

### Historical Analysis

1. Open a 1-3 month chart
2. Find the period's low and high
3. Add 10-15% margin on each side

### Example for SBER

```
Historical range (3 months): 260-290 RUB
Recommended range: 250-300 RUB
Margin: ~10%
```

### Volatility Guide

| Volatility | Range | Step | Levels |
|------------|-------|------|--------|
| Low | Narrow (5-10%) | Small | Many (20+) |
| Medium | Medium (10-20%) | Medium | Medium (11-20) |
| High | Wide (20-30%) | Large | Few (<11) |

## Risk Management

### Stop Loss Setup

```json
{
  "risk_management": {
    "stop_loss_pct": 0.15,
    "max_loss_pct": 0.10,
    "min_balance_reserve": 50000.0
  }
}
```

### Capital Requirements Calculation

```python
# For SBER 250-300 RUB, order_size=10, grid_levels=11
buy_levels = 5
avg_buy_price = 260  # Average buy price
total_buy_capital = buy_levels * order_size * avg_buy_price
# 5 * 10 * 260 = 13,000 RUB

# Recommended reserve
reserve = total_buy_capital * 1.5
# 13,000 * 1.5 = 19,500 RUB
```

### Limits

```json
{
  "risk_management": {
    "max_open_positions": 10,
    "max_position_pct": 0.30,
    "min_balance_reserve": 50000.0
  }
}
```

## Monitoring

### Initialization Logs

```
[INFO] Starting Grid bot for SBER (BBG004730N88)
[INFO] Current price: 275.50
[INFO] Grid initialized, orders placed: 10
[INFO] Order placed: level=0, price=250.00, side=Buy
[INFO] Order placed: level=1, price=255.00, side=Buy
...
```

### Rebalance Logs

```
[INFO] Grid rebalanced: cancelled=2, placed=3
[INFO] Old price: 275.50, new price: 280.20
```

### Execution Logs

```
[INFO] Order filled (Buy level), opposite Sell placed
[INFO] Profit: 5.00 RUB per lot
```

## Troubleshooting

### Price Out of Range

**Problem:** All orders filled, price moved above/below

**Solutions:**
1. Widen the range
2. Increase number of levels
3. Pause the bot

### Insufficient Funds

**Problem:** Error placing orders

**Solutions:**
1. Reduce `order_size`
2. Increase `min_balance_reserve`
3. Reduce number of levels

### Frequent Rebalancing

**Problem:** Orders frequently cancelled and replaced

**Solutions:**
1. Increase rebalancing threshold (default 2%)
2. Widen the range
3. Increase `check_interval`

## Advanced Settings

### Asymmetric Grid

```json
{
  "grid_config": {
    "lower_price": 250.0,
    "upper_price": 300.0,
    "grid_levels": 11,
    "order_size": 10,
    "grid_ratio": 0.6
  }
}
```

**Effect:** 60% buy levels, 40% sell levels

### Dynamic Order Size

```json
{
  "grid_config": {
    "order_size": 10,
    "dynamic_order_size": {
      "enabled": true,
      "min_size": 5,
      "max_size": 20,
      "step_multiplier": 1.5
    }
  }
}
```

**Effect:** Increases order size as price drops

## Next Steps

- **[API Documentation](../developer-guide/api.md)** — API reference
