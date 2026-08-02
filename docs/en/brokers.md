# Supported Brokers

## MockBroker

In-memory broker for backtesting and integration testing. All state lives in `Arc<Mutex<MockState>>` - no network calls.

**What it's for:** testing strategies without real money or API keys. You seed it with candles and prices, then run decisions against it. Every order fills instantly at current price. Useful in unit tests and CI.

```rust
let broker = MockBroker::new("test".into(), 100_000.0);
broker.set_candles("SBER", candles);
broker.set_price("SBER", 285.0);
broker.set_position("SBER", 100, 280.0);
```

Limitations: no partial fills, no latency, no external state.

## TinkoffBroker

Full-featured broker via `t-invest-sdk` (gRPC). Uses Tinkoff Invest API.

| Feature | Status |
|---------|--------|
| Candles (M1, M5, M15, H1, H4, D) | ✅ |
| Order book | ✅ |
| Place/cancel orders (Limit, Market) | ✅ |
| Stop-loss / Take-profit | ✅ (SDK) |
| Portfolio & positions | ✅ |
| Sandbox mode | ✅ |
| Streaming (order book, trades) | ✅ (gRPC streams) |
| Instrument search | ✅ |
| GridBot support | ✅ (streaming-dependent) |

Configured via `api_key` and optional `secret_key`. Account type `"tinkoff"` in config selects this broker.

## FinamBroker

REST/JSON broker over `https://api.finam.ru`. No SDK - raw `reqwest` calls.

| Feature | Status |
|---------|--------|
| Candles (M1, M5, M15, H1, H4, D) | ✅ |
| Order book | ✅ |
| Place/cancel orders (Limit, Market) | ✅ |
| Stop-loss / Take-profit | ❌ |
| Portfolio & positions | ✅ |
| Sandbox mode | ❌ |
| Streaming | ❌ |
| Instrument search | ✅ |
| GridBot support | ❌ (no Tinkoff SDK) |

Authentication: `POST /v1/sessions` with API secret → bearer token. Symbol format: `TICKER@MIC` (e.g. `SBER@MISX`).

Configured via `additional_keys` with `broker: "finam"`.

## Comparison

| | Mock | Tinkoff | Finam |
|--|------|---------|-------|
| Transport | In-memory | gRPC | REST |
| Real money | No | Yes (or sandbox) | Yes |
| Speed | Instant | Network | Network |
| Best for | Tests, CI | Live trading | Live trading (alt broker) |
| BrokerKind | `Mock` | `Tinkoff` | `Other("finam")` |

## Config Location

`trader-bot/config/account.json`
