# Trade API Skill

Multi-broker algorithmic trading platform with ML inference.

## Capabilities

- **Broker API** — Tinkoff (gRPC), Finam (REST), Mock (in-memory)
- **Market Data** — OHLCV candles, order book, last price, liquidity
- **ML Inference** — FinBERT sentiment via ONNX
- **Strategies** — Grid, Interval, Momentum, Mean Reversion, Pairs
- **Backtesting** — Parameter optimization, Sharpe/Calmar metrics

## Commands

- `analyze` — Deep portfolio analysis with FinBERT sentiment
- `scan` — Find instruments by volatility, volume, momentum
- `backtest` — Backtest a strategy with parameter ranges
- `train` — Run FinBERT SFT training pipeline

## Environment

| Variable | Description |
|----------|-------------|
| `API_TOKEN` | Tinkoff Invest API token |
| `FINAM_API_KEY` | Finam Trade API token |
| `FINAM_ACCOUNT_ID` | Finam account number |

## Architecture

```
Rust Trading Core (trader-bot/)
├── core/          — Broker/DataSource/Strategy traits
├── broker/        — Tinkoff, Finam, Mock
├── ml_inference/  — ONNX FinBERT NLP
├── strategy/      — Grid, Interval, Pairs
└── api/           — Axum dashboard

Python Training (training/)
├── finbert_sft/   — SFT pipeline → ONNX
└── data_collection/ — RSS + Perplexica labeling
```
