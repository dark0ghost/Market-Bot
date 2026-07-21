# AI Trade Bot — CLAUDE.md

## Project Overview

Multi-broker algorithmic trading system in Rust with ONNX ML inference (FinBERT), Perplexica context integration, and Python SFT pipeline.

## Architecture

```
ai-trade-bot/
├── trader-bot/                  # Rust — trading core
│   ├── src/
│   │   ├── main.rs              # Entry point, wires everything
│   │   ├── core/                # Broker-agnostic types & traits
│   │   ├── broker/              # Tinkoff, Mock, Finam
│   │   ├── datasource/          # Tinkoff, Finam
│   │   ├── ml_inference/        # ONNX (FinBERT NLP)
│   │   ├── strategy/            # Grid, Interval, Momentum, etc.
│   │   ├── execution/           # Order management
│   │   ├── config/              # Config loading
│   │   └── api/                 # Axum dashboard
│   └── config/account.json
├── mcp-client/                  # LLM integration (Ollama)
├── training/
│   ├── finbert_sft/             # FinBERT SFT pipeline
│   └── data_collection/         # RSS + Perplexica → Ollama labeling
├── models/finbert/              # ONNX artifacts
└── ollama-mcp/                  # Docker Ollama
```

**Data flow:**
```
WS OrderBook → features → ONNX FinBERT → Decision Engine → Risk → Execution
                                            ↕
Perplexica → Redis ← Context Service
```

## Build & Run

```bash
cargo build -p trader-bot
RUST_LOG=info cargo run -p trader-bot

# Python SFT
pip install -r training/requirements.txt
python training/finbert_sft/train.py
python training/finbert_sft/export_onnx.py

# Data collection
python training/data_collection/collect.py --merge
```

## Key Conventions

- **Edition:** 2024
- **Errors:** `anyhow::Result` everywhere
- **Async:** `tokio`, `async_trait` for traits
- **Broker trait:** `crate::core::traits::Broker`
- **Naming:** `snake_case` fns/vars, `PascalCase` types
- **Config:** `trader-bot/config/account.json` serde JSON

## Common Tasks

| Task | Command |
|------|---------|
| Add broker | impl `Broker` trait in `broker/`, register in `mod.rs` |
| Add strategy | impl `Strategy` trait, register in `strategy/registry.rs` |
| Add data source | impl `DataSource` trait in `datasource/` |
| ONNX model update | replace `models/finbert/model.onnx`, auto hot-reload |
| Dashboard route | add to `api/routes/` |

## Brokers

| Kind | Crate | File |
|------|-------|------|
| Tinkoff | `t-invest-sdk` | `broker/tinkoff.rs` |
| Finam | REST | `broker/finam.rs` |
| Mock | in-memory | `broker/mock.rs` |

## ML Pipeline

```
collect.py → dataset.parquet → train.py → models/finbert/ → export_onnx.py → model.onnx
(RSS + Perplexica)   (SFT)                (PyTorch)         (ONNX)
```
