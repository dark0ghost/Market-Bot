# Gates Strategy

The Gates strategy is a screening pipeline of pass/fail gates that filter trading setups before execution. Each gate checks a specific condition — if any gate fails, the trade is rejected.

## How It Works

```
Candle Data ────┬──▶ Detectors ──┬──▶ Swings, FVGs, CISD, Displacements
                │                │
Market Regime ──┤                │
Trend ──────────┤                │
News Sentiment ─┤                │
                │                ▼
              GateContext ──▶ run_gates()
                              [Gate1] → [Gate2] → ... → [GateN]
                                │          │               │
                                ▼          ▼               ▼
                             Pass/      Pass/           Pass/
                             Fail       Fail            Fail
                                         │
                              Short-circuit on first Fail
                                         ▼
                                ✅ All Pass → Trade allowed
                                ❌ Any Fail → Trade rejected
```

## Available Gates

| Gate | Constructor | Logic |
|------|-------------|-------|
| **RegimeGate** | `RegimeGate::new(&["trending"])` | Pass if market regime is in allowed list |
| **TrendGate** | `TrendGate::new(&["bullish"])` | Pass if trend is in allowed list |
| **SwingCheckGate** | `SwingCheckGate::new(3)` | Pass if N+ swing highs/lows detected |
| **FvgGate** | `FvgGate::new(1)` | Pass if N+ unconsumed Fair Value Gaps exist |
| **CisdGate** | `CisdGate` | Pass if at least one CISD signal detected |
| **DisplacementGate** | `DisplacementGate::new(2)` | Pass if N+ displacement candles found |
| **SentimentGate** | `SentimentGate::new(0.3)` | Pass if news sentiment score >= threshold |
| **LlmVetoGate** | `LlmVetoGate::new("veto", llm)` | Last gate — LLM can only reject, never force |

## Pipeline Behavior

- Gates run in sequence; first `Fail` short-circuits the pipeline
- `LlmVetoGate` is designed as the final veto — it can only Fail, never Pass
- `run_gates()` returns all results up to and including the first Fail

## Usage Example

```rust
use trader_bot::strategy::gates::*;

let gates: Vec<Box<dyn Gate>> = vec![
    Box::new(RegimeGate::new(&["trending", "ranging"])),
    Box::new(TrendGate::new(&["bullish"])),
    Box::new(FvgGate::new(1)),
    Box::new(CisdGate),
    Box::new(SentimentGate::new(0.0)),
];

let ctx = build_gate_context("SBER", &candles, 285.50, sentiment, regime, trend);
let results = run_gates(&gates, &ctx);

if results.iter().all(|r| r.is_pass()) {
    // Trade setup approved
}
```

## GateContext

```rust
pub struct GateContext {
    pub ticker: String,
    pub candles: Vec<Candle>,
    pub swings: Vec<Swing>,
    pub fvgs: Vec<FairValueGap>,
    pub cisd_signals: Vec<CISDSignal>,
    pub displacements: Vec<Displacement>,
    pub current_price: f64,
    pub sentiment: Option<NewsSentiment>,
    pub market_regime: Option<String>,
    pub trend: Option<String>,
}
```

Built via `build_gate_context()` which runs all detectors automatically.

## Detectors (analysis/detectors/)

| Detector | Output | Purpose |
|----------|--------|---------|
| `detect_swings()` | `Vec<Swing>` | Swing high/low identification |
| `detect_fvg()` | `Vec<FairValueGap>` | Fair Value Gap detection |
| `update_fvg_states()` | — | Mark FVGs as filled/inversed |
| `detect_cisd()` | `Vec<CISDSignal>` | CISD pattern recognition |
| `detect_displacement()` | `Vec<Displacement>` | Displacement candle detection |

## Refs

- `trader-bot/src/strategy/gates/mod.rs` — Gate trait, run_gates, LlmVetoGate
- `trader-bot/src/strategy/gates/gates_impl.rs` — All gate implementations + detectors wiring
- `trader-bot/src/analysis/detectors/` — Pattern detection logic
