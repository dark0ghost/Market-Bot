# Gates Strategy

Gates strategy - это пайплайн фильтрации торговых сигналов через последовательность ворот (gates). Каждые ворота проверяют определённое условие - если хоть одни не пройдены, сделка отклоняется.

## Как работает

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
                                 ✅ Все Pass → Сделка разрешена
                                 ❌ Любой Fail → Сделка отклонена
```

## Доступные gates

| Gate | Конструктор | Логика |
|------|-------------|--------|
| **RegimeGate** | `RegimeGate::new(&["trending"])` | Pass если рынок в разрешённом режиме |
| **TrendGate** | `TrendGate::new(&["bullish"])` | Pass если тренд в разрешённом списке |
| **SwingCheckGate** | `SwingCheckGate::new(3)` | Pass если N+ свингов найдено |
| **FvgGate** | `FvgGate::new(1)` | Pass если N+ незаполненных FVG |
| **CisdGate** | `CisdGate` | Pass если есть хотя бы один CISD сигнал |
| **DisplacementGate** | `DisplacementGate::new(2)` | Pass если N+ свечей смещения |
| **SentimentGate** | `SentimentGate::new(0.3)` | Pass если сентимент новостей >= порога |
| **LlmVetoGate** | `LlmVetoGate::new("veto", llm)` | Финальное вето - LLM может только отклонить |

## Поведение пайплайна

- Gates выполняются последовательно; первый `Fail` прерывает пайплайн
- `LlmVetoGate` - последний, может только Fail (вето), никогда Pass
- `run_gates()` возвращает все результаты до первого Fail включительно

## Пример использования

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
    // Сделка одобрена
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

Создаётся через `build_gate_context()`, которая автоматически запускает все детекторы.

## Детекторы (analysis/detectors/)

| Детектор | Выход | Назначение |
|----------|-------|------------|
| `detect_swings()` | `Vec<Swing>` | Поиск свинг-хаев/лоёв |
| `detect_fvg()` | `Vec<FairValueGap>` | Поиск Fair Value Gap |
| `update_fvg_states()` | - | Пометка FVG как filled/inversed |
| `detect_cisd()` | `Vec<CISDSignal>` | Распознавание CISD паттернов |
| `detect_displacement()` | `Vec<Displacement>` | Детекция свечей смещения |

## Ссылки

- `trader-bot/src/strategy/gates/mod.rs` - Gate trait, run_gates, LlmVetoGate
- `trader-bot/src/strategy/gates/gates_impl.rs` - Все реализации gates + подключение детекторов
- `trader-bot/src/analysis/detectors/` - Логика детекции паттернов
