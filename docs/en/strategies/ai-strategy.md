# AI Strategy

The AI Strategy uses LLM (Ollama/fin-expert) and rule-based analysis to produce trading signals. It wraps `TradingAgent` which aggregates technical, fundamental, and news sentiment data into a structured decision.

## How It Works

```
Market Data ─┬─ Technical Analysis (RSI, MACD, Bollinger, Volume)
             ├─ Fundamental Analysis (P/E, ROE, D/E, growth)
             ├─ News Sentiment (FinBERT ONNX)
             └─ Portfolio State  ─┐
                                  ▼
                          TradingAgent
                         ┌──────────┐
                         │  LLM or   │
                         │ Rule-based│
                         │ Decision  │
                         └────┬─────┘
                              ▼
                    TradingDecision
              {action, confidence, entry_price,
               stop_loss, take_profit, rationale}
```

### Decision Modes

| Mode | Description | Latency |
|------|-------------|---------|
| **LLM** (`use_llm=true`) | Builds a prompt from all signals, queries Ollama, parses JSON response | ~1-5s |
| **Rule-based** (`use_llm=false`) | Applies deterministic rules without LLM call | <10ms |

## Configuration

```json
{
  "use_llm": false,
  "use_finbert": false,
  "min_confidence": 0.6,
  "force_regime": null,
  "memory_path": null
}
```

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `use_llm` | bool | false | Use LLM for decisions (vs rule-based) |
| `use_finbert` | bool | false | Enable FinBERT news sentiment |
| `min_confidence` | float | 0.6 | Minimum confidence to act (0.0-1.0) |
| `force_regime` | string? | null | Override market regime detection |
| `memory_path` | string? | null | Path to decision memory JSON file |

## Technical Analysis Inputs

- **RSI** — oversold/overbought thresholds
- **MACD** — line, signal, histogram for crossovers
- **Bollinger Bands** — bandwidth, position within bands
- **Volume** — unusual volume detection, volume ratio
- **Support/Resistance** — key price levels
- **Trend** — bullish/bearish/sideways

## Risk Management

Embedded in the decision pipeline:

- **Position Sizing** — confidence-based, adjusted for proximity to resistance
- **Stop Loss / Take Profit** — configurable percentages via `RiskManagementConfig`
- **Max Drawdown** — global limit
- **Max Open Positions** — prevents overexposure

## Decision Memory

All decisions are recorded to `DecisionMemory` (RAM + optional JSON persistence). Recent decisions for the same ticker are fed as few-shot context in the LLM prompt (RAG).

## Refs

- `trader-bot/src/strategy/ai.rs` — Strategy implementation
- `trader-bot/src/agent/trading_agent.rs` — TradingAgent (decision logic)
- `trader-bot/src/agent/memory.rs` — DecisionMemory
