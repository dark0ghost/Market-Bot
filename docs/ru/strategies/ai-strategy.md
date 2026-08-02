# AI стратегия

AI стратегия использует LLM (Ollama/fin-expert) и rule-based анализ для генерации торговых сигналов. Оборачивает `TradingAgent`, который агрегирует технические, фундаментальные данные и новостной сентимент в структурированное решение.

## Как работает

```
Рыночные данные ─┬─ Тех. анализ (RSI, MACD, Bollinger, Volume)
                 ├─ Фундам. анализ (P/E, ROE, D/E, рост)
                 ├─ Новостной сентимент (FinBERT ONNX)
                 └─ Состояние портфеля ─┐
                                        ▼
                                TradingAgent
                               ┌────────────┐
                               │  LLM или    │
                               │ Rule-based  │
                               │ Decision    │
                               └──────┬─────┘
                                      ▼
                          TradingDecision
                    {action, confidence, entry_price,
                     stop_loss, take_profit, rationale}
```

### Режимы принятия решений

| Режим | Описание | Задержка |
|-------|----------|----------|
| **LLM** (`use_llm=true`) | Собирает промпт из всех сигналов, запрос к Ollama, парсинг JSON | ~1-5с |
| **Rule-based** (`use_llm=false`) | Детерминированные правила без LLM | <10мс |

## Конфигурация

```json
{
  "use_llm": false,
  "use_finbert": false,
  "min_confidence": 0.6,
  "force_regime": null,
  "memory_path": null
}
```

| Поле | Тип | По умолч. | Описание |
|------|-----|-----------|----------|
| `use_llm` | bool | false | Использовать LLM для решений |
| `use_finbert` | bool | false | Включить FinBERT для новостей |
| `min_confidence` | float | 0.6 | Мин. уверенность для сделки (0.0-1.0) |
| `force_regime` | string? | null | Принудительный рыночный режим |
| `memory_path` | string? | null | Путь к JSON-файлу памяти решений |

## Входные данные тех. анализа

- **RSI** - уровни перекупленности/перепроданности
- **MACD** - линия, сигнал, гистограмма
- **Bollinger Bands** - ширина полос, позиция цены
- **Volume** - аномальный объём, коэффициент
- **Support/Resistance** - ключевые уровни
- **Trend** - бычий/медвежий/боковой

## Управление рисками

Встроено в пайплайн решений:

- **Размер позиции** - на основе уверенности, корректировка у сопротивления
- **Stop Loss / Take Profit** - настраиваемые проценты через `RiskManagementConfig`
- **Макс. просадка** - глобальный лимит
- **Макс. открытых позиций** - защита от переэкспозиции

## Память решений

Все решения записываются в `DecisionMemory` (RAM + опционально JSON). Последние решения по тому же тикеру подаются в промпт LLM как few-shot контекст (RAG).

## Ссылки

- `trader-bot/src/strategy/ai.rs` - реализация стратегии
- `trader-bot/src/agent/trading_agent.rs` - TradingAgent (логика решений)
- `trader-bot/src/agent/memory.rs` - DecisionMemory
