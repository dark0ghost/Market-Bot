# Broker Setup

The project supports three broker backends: Tinkoff, Finam, and Mock (in-memory simulator). Each provides the full `Broker` trait interface for market data, order management, and portfolio queries.

## Supported Brokers Overview

| Feature | Tinkoff | Finam | Mock |
|---------|---------|-------|------|
| Market data (candles) | ✅ gRPC | ✅ REST | ✅ synthetic |
| Last price | ✅ | ✅ | ✅ configured |
| Order book | ✅ | ✅ | ✅ synthetic |
| Place/cancel orders | ✅ | ✅ | ✅ simulated |
| Portfolio & balance | ✅ | ✅ | ✅ simulated |
| Sandbox/paper mode | ✅ `Environment::Sandbox` | ❌ production only | ✅ built-in |
| Production mode | ✅ | ✅ | — |
| Requires API token | ✅ | ✅ | ❌ |
| SDK | `t-invest-sdk 0.17` | raw `reqwest` | none |

---

## Tinkoff Invest

[Tinkoff Invest](https://www.tinkoff.ru/invest/) provides both a **sandbox** environment (paper trading with virtual money) and a **production** environment (real money).

### 1. Get an API Token

1. Go to [Tinkoff Invest API](https://www.tinkoff.ru/invest/settings/api/)
2. Click **"Create a token"**
3. Select the required permissions:
   - For sandbox: any permissions (virtual money)
   - For production: grant read + trade permissions
4. Copy the token (starts with `t.`)

### 2. Configure Sandbox (Paper Trading)

Set the mode to `sandbox` in `trader-bot/config/account.json`:

```json
{
  "creditional": {
    "token": "t.YOUR_TINKOFF_TOKEN"
  },
  "mode": "sandbox",
  "accounts": [
    {
      "account_id": "main",
      "broker": "tinkoff",
      "instruments": [
        {
          "figi": "BBG004730N88",
          "ticker": "SBER",
          "name": "Sberbank",
          "enabled": true,
          "max_position_pct": 0.2
        }
      ],
      "strategy": {
        "strategy": "interval",
        "parameters": {
          "interval_size": "1h",
          "days_back_to_consider": 30,
          "quantity_limit": 1000,
          "check_interval": 60
        }
      },
      "risk_management": {
        "max_loss_pct": 0.05,
        "take_profit_pct": 0.10,
        "stop_loss_pct": 0.03,
        "max_open_positions": 5,
        "min_balance_reserve": 100000.0
      }
    }
  ]
}
```

The `mode: "sandbox"` field causes the SDK to connect to `Environment::Sandbox` — all orders are executed with virtual money. No real funds are used.

You can also pass the token via environment variable (useful for CI/CD or keeping secrets out of VCS):

```bash
export API_TOKEN="t.YOUR_TINKOFF_TOKEN"
```

Leave the token empty in the config:

```json
{
  "creditional": { "token": "" },
  "mode": "sandbox"
}
```

### 3. Configure Production (Real Money)

Change the mode to `prod`:

```json
{
  "mode": "prod"
}
```

**⚠️ WARNING:** In production mode the bot will use **real money**. Make sure your strategy is thoroughly tested in sandbox first.

### 4. Sandbox Account Auto-Creation

When `mode` is `"sandbox"`, you can optionally configure automatic account creation and funding:

```json
{
  "mode": "sandbox",
  "sandbox": {
    "open_account": true,
    "pay_in_amount": 30000000
  }
}
```

| Field | Default | Description |
|-------|---------|-------------|
| `open_account` | `true` | Auto-create sandbox account via `OpenSandboxAccount` if `account_id` is empty or not provided |
| `pay_in_amount` | `30000000` | Amount in RUB to deposit via `SandboxPayIn` (max 30M) |

If `account_id` is provided in the config, it will be used as-is (no account creation). If it's empty or missing, the bot calls `OpenSandboxAccount`, logs the new account ID, and deposits the configured amount.

**Important:** Sandbox accounts expire after 3 months of inactivity. Always check the logs for the created `account_id` — you can hardcode it later to reuse the same account across restarts.

---

## Finam Trade API

[Finam Trade API](https://www.finam.ru/) provides REST API access to real trading (sandbox/paper trading is not available — only production).

### 1. Get API Credentials

1. Contact Finam support or your broker to obtain API access
2. You will receive an API secret/token

### 2. Configuration

Finam credentials go in `additional_keys`:

```json
{
  "creditional": {
    "token": "",
    "additional_keys": [
      {
        "broker": "finam",
        "api_key": "YOUR_FINAM_API_SECRET",
        "secret_key": null,
        "extra": null
      }
    ]
  },
  "mode": "sandbox",
  "accounts": [
    {
      "account_id": "finam_main",
      "broker": "finam",
      "instruments": [...]
    }
  ]
}
```

### 3. Symbol Format

Finam uses `@` as a separator in instrument identifiers (e.g., `SBER@TQBR`). The codebase internally converts between `_` and `@`:

| Internal | Finam API |
|----------|-----------|
| `SBER_TQBR` | `SBER@TQBR` |

---

## Mock Broker (Built-in Simulator)

The `MockBroker` is an in-memory broker implementation for testing and backtesting. It requires **no API tokens, no network, and no external services**.

### Usage

```rust
use trader_bot::broker::MockBroker;

let broker = MockBroker::new("test".to_string(), 1_000_000.0);
broker.set_price("SBER", 250.0);
broker.set_candles("SBER", candles); // seed historical data
```

Mock broker features:
- **Instant order filling** — all limit/market orders execute immediately
- **Balance tracking** — buy/sell operations update cash balance
- **Position tracking** — weighted-average cost for accumulated positions
- **PnL calculation** — realized profit/loss computed on sell
- **Synthetic order book** — bid = price × 0.999, ask = price × 1.001
- **Synthetic liquidity** — hardcoded constant values

### Testing with MockBroker

```rust
#[tokio::test]
async fn test_strategy_with_mock() {
    let broker = MockBroker::new("test".to_string(), 100_000.0);
    broker.set_price("SBER", 250.0);

    let price = broker.last_price("SBER").await.unwrap();
    assert_eq!(price, 250.0);

    let balance = broker.balance().await.unwrap();
    assert_eq!(balance, 100_000.0);
}
```

---

## Feature Matrix by Broker Kind

| `BrokerKind` | Implemented | Sandbox | Production | Notes |
|-------------|-------------|---------|------------|-------|
| `Tinkoff` | ✅ | ✅ `Environment::Sandbox` | ✅ | Primary broker |
| `Mock` | ✅ | N/A (always simulated) | N/A | Testing only |
| `Other("finam")` | ✅ | ❌ | ✅ | REST API |
| `Alor` | ❌ enum only | — | — | Not implemented |
| `Binance` | ❌ enum only | — | — | Not implemented |
| `ByBit` | ❌ enum only | — | — | Not implemented |
| `InteractiveBrokers` | ❌ enum only | — | — | Not implemented |

---

## Environment Variables Summary

| Variable | Purpose | Example |
|----------|---------|---------|
| `API_TOKEN` | Tinkoff Invest API token | `t.YOUR_TOKEN` |
| `RUST_LOG` | Logging level | `info`, `debug`, `warn` |
| `LOG_FILE` | File log path | `/var/log/trader-bot.log` |
| `LOG_WEBHOOK` | Network log webhook URL | `https://logs.example.com/ingest` |
| `LOG_LEVEL` | Log level override | `debug` |

