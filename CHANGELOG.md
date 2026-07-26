# Changelog

Все изменения в проекте Market Bot.

## [0.3.0] — 2026-07-21

### Добавлено

#### FinBERT SFT (Supervised Fine-Tuning)
- **`training/finbert_sft/`** — Python-пайплайн для дообучения FinBERT
  - `dataset.py` — загрузка Financial PhraseBank (3 класса sentiment)
  - `model.py` — загрузка ProsusAI/finbert с classification head
  - `train.py` — SFT: HuggingFace Trainer, FP16, early stopping, eval по F1
  - `evaluate.py` — classification report, confusion matrix
  - `export_onnx.py` — torch.onnx.export → model.onnx с dynamic axes
  - `config.yaml` — все гиперпараметры обучения

- **`trader-bot/src/ml_inference/`** — ONNX-инференс в Rust
  - `session.rs` — `OrtSessionPool` с hot-reload через `notify`
  - `nlp.rs` — FinBERT inference: tokenization → ONNX → softmax → sentiment

- **`models/finbert/`** — директория для ONNX-артефактов (gitignored)

#### Новые зависимости
- `ort`, `ndarray`, `tokenizers` — ONNX Runtime для Rust
- `arc-swap` — lock-free чтение модели (hot-reload)
- `notify` — отслеживание изменений model.onnx

### Изменено

#### Архитектура проекта (3-слойная)
- **Layer 1**: Real-Time Trading (Rust, Tokio, ORT)
- **Layer 2**: Near-Real-Time Context (Perplexica → Redis)
- **Layer 3**: Offline Training (Python, PyTorch → ONNX)

#### Документация
- **README.md** — обновлена архитектура, добавлен раздел FinBERT SFT
- **docs/OVERVIEW.md** — обновлена структура проекта

---

## [0.2.0] — 2026-02-22

### Добавлено

#### Grid Trading Bot
- **GridStrategy**: Расчет уровней сетки с равномерным шагом
  - Автоматическое разделение на buy/sell уровни
  - Поддержка custom grid ratio (соотношение buy/sell)
  - Методы для перебалансировки сетки

- **GridExecutor**: Исполнитель ордеров для Grid стратегии
  - Инициализация сетки ордеров
  - Размещение лимитных ордеров на уровнях
  - Перебалансировка при изменении цены на 2%+
  - Обработка исполнения ордеров (размещение противоположного)
  - Корректная остановка с отменой ордеров

- **GridBot**: Основной цикл Grid бота
  - Автоматический запуск из конфигурации
  - Периодическая перебалансировка
  - Мониторинг цены через MarketDataService
  - Запуск в отдельных tokio задачах

#### Конфигурация
- Новый тип стратегии: `StrategyType::Grid`
- `GridConfig`: Конфигурация Grid стратегии
  - `lower_price` / `upper_price` — диапазон цен
  - `grid_levels` — количество уровней
  - `order_size` — размер ордера в лотах
  - `grid_ratio` — соотношение buy/sell (0.5 = 50/50)

- Пример конфигурации для SBER Grid бота в `account.json`

#### Интеграция
- Автоматический запуск Grid ботов для аккаунтов с `strategy: "grid"`
- Параллельная работа с обычными AI-стратегиями
- Поддержка нескольких Grid ботов одновременно

#### Документация
- **GRID_BOT.md**: Полная документация по Grid боту
  - Принцип работы с диаграммой
  - Примеры конфигурации
  - Параметры GridConfig
  - Рекомендации по выбору диапазона
  - Управление рисками
  - Troubleshooting

- **README.md**: Обновленная документация проекта
  - Оглавление
  - Быстрый старт
  - Примеры использования
  - API и интеграции

### Изменено

#### Структура проекта
- `trader-bot/src/strategy/grid.rs` — Grid стратегия
- `trader-bot/src/strategy/grid_executor.rs` — Исполнитель
- `trader-bot/src/strategy/grid_bot.rs` — Основной цикл
- `trader-bot/src/config/data.rs` — Конфигурация

#### Зависимости
- Обновлены импорты в `trader-bot/src/main.rs`
- Добавлен `MarketDataService::sdk_clone()` для Grid бота

### Исправлено

- Ошибки времени жизни в `GridStrategy::get_level_by_index()`
- Проблемы с заимствованием в `GridExecutor::on_order_filled()`
- Экспорты модулей в `strategy/mod.rs`

---

## [0.1.5] — 2026-02-22

### Добавлено

#### NewsLLMService
- LLM-анализ тональности новостей
- Пакетный анализ с выделением ключевых событий
- Определение рисков и возможностей
- Интеграция с Ollama (fin-expert)

#### Улучшения анализа
- Конвертация Sentiment между модулями
- Обогащенный результат анализа новостей
- Логирование резюме от LLM

### Изменено

#### mcp-client
- Добавлен `Clone` для `OllamaProvider`
- Сохранение host/port в структуре

#### main.rs
- Двухуровневый анализ новостей (сбор + LLM)
-Fallback на rule-based при ошибке LLM

---

## [0.1.4] — 2026-02-21

### Добавлено

#### MarketDataService
- `get_historical_candles()` — получение свечей из API
- `get_5min_candles()` — 5-минутные свечи
- `get_last_price()` — текущая цена
- Конвертация HistoricCandle → Candle

#### PortfolioService
- `get_accounts()` — список счетов
- `get_portfolio()` — текущий портфель
- `get_available_balance()` — доступный баланс
- `get_position()` — позиция по инструменту

#### FundamentalDataService
- Загрузка фундаментальных данных
- Пример данных для Т-Технологии
- Отраслевые средние значения

### Изменено

#### main.rs
- Интеграция MarketDataService для данных
- Интеграция PortfolioService для баланса
- Интеграция FundamentalDataService для анализа
- Реальное исполнение ордеров с расчетом лотов

#### TradingExecutor
- Расчет количества лотов от баланса
- Логирование исполнения решений

---

## [0.1.3] — 2026-02-20

### Добавлено

#### Технический анализ
- RSI (Relative Strength Index)
- MACD (Moving Average Convergence Divergence)
- Bollinger Bands
- Уровни поддержки и сопротивления
- Анализ объема

#### NewsAnalyzer
- Сбор новостей из источников
- Анализ тональности
- Выделение ключевых событий

#### FundamentalAnalyzer
- Valuation_metrics (P/E, PEG, P/B)
- Profitability metrics (ROE, маржинальность)
- Financial health (D/E, Current Ratio)
- Growth metrics (рост выручки/прибыли)
- Company Rating (Excellent/Good/Fair/Poor/VeryPoor)

### Изменено

#### TradingAgent
- `make_decision()` — LLM-решение
- `make_rule_based_decision()` — rule-based решение
- Расчет позиции с учетом рисков

---

## [0.1.2] — 2026-02-19

### Добавлено

#### MCP Client
- `LLMProvider` trait
- `OllamaProvider` implementation
- Интеграция с ollama-rs

#### Конфигурация
- `account.json` с примерами инструментов
- Поддержка LLM config
- Risk management config

### Изменено

#### Структура проекта
- Workspace с mcp-client и trader-bot
- Модульная архитектура

---

## [0.1.1] — 2026-02-18

### Добавлено

- Базовая структура проекта
- Tinkoff Invest SDK интеграция
- Market data streaming
- Ollama Docker container

---

## [0.1.0] — 2026-02-17

### Добавлено

- Initial проект
- Rust workspace
- Docker compose для Ollama
