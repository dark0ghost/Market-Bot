# AI Trade Bot — Project Context

## Project Overview

This is a multi-component **AI-powered trading system** that combines:
- **High-Frequency Trading (HFT)** with neural network predictions (Python/TensorFlow)
- **LLM-based market analysis** using Ollama with custom financial expert models
- **Real-time market data streaming** from Tinkoff Invest API
- **MCP (Model Context Protocol)** integration for LLM tool interactions

### Architecture

```
ai-trade-bot/
├── trader-bot/          # Main Rust trading bot (workspace member)
├── mcp-client/          # MCP client library for LLM integration (workspace member)
├── hft-ai-trader/       # Python HFT system with LSTM models
└── ollama-mcp/          # Docker container for Ollama with custom financial model
```

## Components

### 1. trader-bot (Rust)
**Purpose:** Core trading bot that connects to Tinkoff Invest API and executes trading strategies.

**Key Dependencies:**
- `t-invest-sdk` — Tinkoff Invest API client
- `ollama-rs` — LLM integration for market analysis
- `tokio` — Async runtime
- `flume` — Channel-based communication
- `mcp-client` — Internal MCP client library

**Structure:**
```
trader-bot/src/
├── main.rs              # Entry point, market data streaming
├── config/              # Configuration management
├── client/              # Tinkoff API client wrapper
├── mcp/                 # MCP/LLM provider integration
├── strategy/            # Trading strategies (Interval, etc.)
├── instrument.rs        # Instrument data structures
└── utils/               # Utilities
```

### 2. mcp-client (Rust)
**Purpose:** Library for LLM provider integration with tool support.

**Key Dependencies:**
- `ollama-rs` — Ollama API client with streaming and macros
- `reqwest` — HTTP client
- `tokio-stream` — Stream utilities

**Structure:**
```
mcp-client/src/
├── lib.rs               # Module exports
├── llm_provider.rs      # LLMProvider trait
└── ollama.rs            # OllamaProvider implementation
```

**LLMProvider Trait:**
```rust
pub trait LLMProvider<T, E> {
    async fn send_message(self, text: String) -> Result<T, E>;
}
```

### 3. hft-ai-trader (Python)
**Purpose:** High-Frequency Trading system with LSTM neural network for price prediction.

**Tech Stack:**
- TensorFlow 2.19+ / TFLite Runtime
- NumPy, Pandas
- Poetry for dependency management

**Model:**
- LSTM-based architecture with 60-tick window
- Quantized TFLite model for fast inference
- Predicts price movements based on price + volume data

### 4. ollama-mcp (Docker)
**Purpose:** Containerized Ollama instance with custom financial analyst model.

**Configuration:**
- Base model: `qwen3:1.7b`
- Custom system prompt for trading signals (LONG/SHORT)
- GPU support (NVIDIA)
- Exposed port: 11435 → 11434

**Custom Model (`fin-expert`):**
- Temperature: 0.3
- Context window: 4096
- Output format: Signal, Levels (Entry/SL/TP), Rationale, Risks

## Building and Running

### Prerequisites
- Rust (edition 2024)
- Python 3.11+
- Docker with NVIDIA GPU support
- Tinkoff Invest API token

### Rust Workspace

```bash
# Build all workspace members
cargo build

# Build in release mode
cargo build --release

# Run trader-bot
cargo run -p trader-bot

# Run tests
cargo test
```

### Python HFT Trader

```bash
cd hft-ai-trader

# Install dependencies
poetry install

# Run training
poetry run python src/hft.py
```

### Docker (Ollama)

```bash
# Build and run Ollama container
docker-compose up --build

# The model fin-expert will be auto-created on startup
```

### Environment Variables

| Variable | Description | Required |
|----------|-------------|----------|
| `API_TOKEN` | Tinkoff Invest API token | Yes |

## Configuration

Trading bot uses a config file (location in `.idea/` or external) with structure:

```toml
type = "..."
[creditional]
token = "..."

[[accounts]]
# Account-specific strategies

mode = "sandbox" | "prod"
```

## Development Conventions

### Rust
- **Edition:** 2024
- **Error Handling:** `anyhow::Result` for application code
- **Async:** Tokio runtime with `#[tokio::main]`
- **Naming:** `snake_case` for variables/functions, `PascalCase` for types
- **Module Structure:** `mod.rs` for module roots, flat structure within directories

### Python
- **Version:** 3.11+
- **Package Manager:** Poetry
- **ML Framework:** TensorFlow with TFLite optimization

### Project Structure
- Workspace members in root `Cargo.toml`
- Shared dependencies in `[workspace.dependencies]`
- License: MIT (see `LICENSE`)

## Key Integration Points

1. **Tinkoff API → Trader Bot:** Real-time candle data streaming via `MarketDataStream`
2. **Trader Bot → Ollama:** LLM analysis for trading signals via `OllamaProvider`
3. **LLM Tools:** DDGSearcher, Scraper, Calculator for market research
4. **HFT Model:** TFLite model for microsecond-level predictions (integration in progress)

## Current Status

- ✅ Workspace structure established
- ✅ Market data streaming implemented
- ✅ Ollama integration with custom financial model
- ✅ LLM tool system (search, scrape, calculate)
- 🔄 HFT model training script complete
- 🔄 Strategy implementations in progress
