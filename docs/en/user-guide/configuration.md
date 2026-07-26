# Configuration

The `account.json` configuration file defines all bot operating parameters.

## File Structure

```json
{
  "type": "trading",
  "creditional": { "token": "..." },
  "accounts": [...],
  "mode": "sandbox",
  "llm_config": {...}
}
```

## Core Parameters

### Root Parameters

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `type` | string | ✅ | Config type ("trading") |
| `creditional.token` | string | ✅ | Tinkoff Invest token |
| `accounts` | array | ✅ | Array of accounts |
| `mode` | string | ✅ | Mode ("sandbox" or "prod") |
| `llm_config` | object | ❌ | LLM settings |

### Account Config

| Parameter | Type | Description |
|-----------|------|-------------|
| `account_id` | string | Account ID |
| `instruments` | array | Trading instruments |
| `strategy` | object | Strategy settings |
| `risk_management` | object | Risk management |

### Instrument Config

| Parameter | Type | Description |
|-----------|------|-------------|
| `figi` | string | Instrument FIGI |
| `ticker` | string | Ticker |
| `name` | string | Name |
| `enabled` | bool | Is active |
| `max_position_pct` | f64 | Max portfolio share |
| `analysis_config` | object | Analysis settings |

## Examples

### Minimal Configuration

```json
{
  "type": "trading",
  "creditional": { "token": "YOUR_TOKEN" },
  "accounts": [{
    "account_id": "main",
    "instruments": [{
      "figi": "TQBR",
      "ticker": "TTECH",
      "name": "T-Technologies",
      "enabled": true,
      "max_position_pct": 0.1
    }],
    "strategy": {
      "strategy": "interval",
      "parameters": {
        "interval_size": "1h",
        "days_back_to_consider": 30,
        "quantity_limit": 1000,
        "check_interval": 60
      }
    }
  }],
  "mode": "sandbox"
}
```

### Grid Strategy

```json
{
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
  }
}
```

### Risk Management

```json
{
  "risk_management": {
    "max_loss_pct": 0.05,
    "take_profit_pct": 0.10,
    "stop_loss_pct": 0.03,
    "max_open_positions": 5,
    "min_balance_reserve": 100000.0
  }
}
```

### LLM Configuration

```json
{
  "llm_config": {
    "model": "fin-expert",
    "host": "http://localhost",
    "port": 11435,
    "temperature": 0.3,
    "context_window": 4096
  }
}
```

## Environment Variables

Instead of specifying the token in the file, you can use an environment variable:

```bash
export API_TOKEN="your_token"
```

Leave the token empty in the config:

```json
{
  "creditional": { "token": "" }
}
```
