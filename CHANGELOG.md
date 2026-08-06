# Changelog

Все изменения в проекте Market Bot.

## [Unreleased] - 2026-08-07

### Добавлено

#### Торговый календарь MOEX
- **`trader-bot/src/strategy/trading_calendar.rs`** - `TradingCalendar`:
  проверка торговых часов MOEX (10:00–18:45 МСК), выходных и праздников.
  Гейт перед каждым ордером в `run_ai_account` и в цикле `GridBot` -
  бот больше не шлёт ордера 24/7.

#### Broker-side Stop-Loss / Take-Profit
- `StopOrderRequest` / `StopOrderKind` в `core::types`, метод
  `place_stop_order` в `Broker` trait (дефолт - error для брокеров без поддержки).
- Реализация для Tinkoff (`stop_orders().post_stop_order`) и
  `PositionManager::place_stop_order`. SL/TP теперь ставятся на стороне
  брокера и переживают рестарт бота (раньше только логировались, `// Stop loss via separate order`).
- `TradingExecutor::execute_decision` и `execute_via_broker` в `main.rs`
  выставляют SL/TP через брокер, с in-memory fallback в трекере при ошибке.

#### Crash recovery для Grid
- `GridExecutor` хранит mapping `level_index → order_id`.
  `cancel_order_by_level` и `stop_grid` теперь реально отменяют ордера
  в брокере (раньше TODO/no-op → orphan orders).

#### Конфигурация и документация
- `trader-bot/config/account.example.json` - шаблон конфига (раньше собирали из README).
- `models/finbert/MODEL_CARD.md` - карточка ONNX-модели.
- `training/requirements.lock.txt` - pinned-compatible версии Python-зависимостей.

#### BrokerType enum
- `config/data.rs` - `BrokerType` (`Tinkoff`/`Finam`/`Mock`) вместо magic-строк в
  `AccountConfig.broker` и `BrokerCredential.broker`; `#[serde(rename_all = "lowercase")]`
  сохраняет обратную совместимость с существующими конфигами, добавлен `Display`.
- `main.rs::init_broker` переведён на `match account.broker` (неизвестный брокер
  теперь ошибка десериализации, а не рантайм-ошибка).

#### Внешний календарь праздников
- `TradingCalendar` помимо повторяющихся (месяц, день) держит exact-даты
  (`NaiveDate`); файл `YYYY-MM-DD` (по строке, `#` - комментарии) подключается
  через env `MOEX_HOLIDAYS_FILE`, при отсутствии файла - fallback на встроенный
  набор. Подключено в `main.rs` и `GridBot`.

#### Decimal для денег
- Broker-facing money-поля (`OrderRequest.price`, `StopOrderRequest.stop_price`,
  `OrderResponse.*price`, `PositionView`, `PortfolioView`) переведены с `f64` на
  `rust_decimal::Decimal`; хелперы `f64_to_decimal`/`decimal_to_f64`/`decimal_from_str`.
  Рыночные цены (OHLCV, стакан) остались f64 - они кормят индикаторы.
- `rust_decimal` включён с фичей `serde`; `FinamBroker` парсит деньги сразу в
  Decimal (`parse_decimal_money`) без f64-промежутка.

#### FinamDataSource подключение
- `AccountBroker.datasource: Option<Arc<dyn DataSource>>`; `init_broker` создаёт
  `TinkoffDataSource`/`FinamDataSource` на каждый аккаунт; датасорсы регистрируются
  из аккаунтов (раньше только Tinkoff).

### Изменено

#### Качество и корректность
- **Rate limiter Tinkoff** - `try_acquire` теперь корректно обновляет
  `last_refill` при каждом успешном acquire (баг: обновлялся только при пустом
  бакете → лимит не работал как token-bucket).
- **`get_orders` (Tinkoff, PositionManager)** - маппинг реального направления
  и статуса исполнения из ответа брокера вместо хардкода `Buy`/`New`.
- **`win_rate` метрика** - теперь обновляется в `record_execution`
  (раньше всегда `0.0` в `/metrics`).
- **`analyze_batch` FinBERT** - конкурентный `spawn_blocking` + `try_join_all`
  вместо последовательного цикла (async runtime больше не блокируется).
- **Дубли order placement** - общий `build_post_order_request` в `PositionManager`.
- **Дубли key-event extraction** - общий модуль `analysis::key_events`
  (заменил копии в `news.rs` и `finbert.rs`).

#### Python-пайплайн
- `collect.py` - `ET.fromstring` обёрнут в try/except (malformed RSS больше не
  валит весь сбор); rate-limiting (`inter_feed_delay_sec`) между RSS-фидами.
- `export_onnx.py` - сохраняет tokenizer рядом с ONNX + `MANIFEST.json` с SHA-256
  чексуммами (раньше tokenizer не экспортировался).
- `train.py` - метрики расширены: Cohen's kappa, ECE (калибровка), per-class F1.

#### Инфра
- `.gitignore` - убран `trader-bot/Cargo.lock` (бинарник должен коммитить lock).

#### main.rs → библиотека
- `main.rs` переведён с `mod ...` на `use trader_bot::...` (lib.rs) - убрано
  ~250 dead-code warnings.
- Удалены мёртвые модули: `scanner/`, `strategy/strategy.rs`, `config/api_provider.rs`.
- Удалены неиспользуемые поля/структуры: `z_exit` (StatArbPredictor),
  `model_name` (TradingAgent), `client` (FundamentalDataService),
  `trades_file`/`signals_file` (Journal), `max_drawdown_pct`/`var_confidence`
  (RiskAgent), `FinamAsset`/`FinamAssetsResponse`, `calculate_spread` (pairs).

#### CI
- `.gitlab-ci.yml` - `RUST_VERSION` 1.85 → 1.88 (код использует let-chains).

#### AGENTS.md
- Синхронизирован с реальной структурой: убраны несуществующие `mcp-client/`,
  `skills/trade-api/`, `CalibrationAgent`/`MemoryAgent`, мнимый time-series ONNX;
  дерево и структуры агентов приведены к коду.

### Исправлено
- Опечатка `creditional` → `credential` в `README.md` и `GRID_BOT.md`
  (задокументированный конфиг не парсился serde).
- `PositionTracker` - `mem.add(rec.clone())` возвращал future без `.await`
  (решение молча не записывалось в память); добавлен синхронный
  `DecisionMemory::add_sync`.
- `TradingAgent::record_decision` - `MutexGuard` удерживался через `await`
  (clippy `await_holding_lock`); сериализация JSON вынесена из-под лока.
- `use_finbert` - проверял только первый аккаунт (`.first()`) → `.iter().any()`.
- Дублирующиеся ветки `StrategyType::Ai` и `_` в `create_strategy`.
- Clippy: `too_many_arguments` (fundamental), collapsible `if` (nlp, main),
  `saturating_sub` (tinkoff), импорт `OrderStatus` в tests (grid_executor).

---

## [0.3.0] - 2026-07-21

### Добавлено

#### FinBERT SFT (Supervised Fine-Tuning)
- **`training/finbert_sft/`** - Python-пайплайн для дообучения FinBERT
  - `dataset.py` - загрузка Financial PhraseBank (3 класса sentiment)
  - `model.py` - загрузка ProsusAI/finbert с classification head
  - `train.py` - SFT: HuggingFace Trainer, FP16, early stopping, eval по F1
  - `evaluate.py` - classification report, confusion matrix
  - `export_onnx.py` - torch.onnx.export → model.onnx с dynamic axes
  - `config.yaml` - все гиперпараметры обучения

- **`trader-bot/src/ml_inference/`** - ONNX-инференс в Rust
  - `session.rs` - `OrtSessionPool` с hot-reload через `notify`
  - `nlp.rs` - FinBERT inference: tokenization → ONNX → softmax → sentiment

- **`models/finbert/`** - директория для ONNX-артефактов (gitignored)

#### Новые зависимости
- `ort`, `ndarray`, `tokenizers` - ONNX Runtime для Rust
- `arc-swap` - lock-free чтение модели (hot-reload)
- `notify` - отслеживание изменений model.onnx

### Изменено

#### Архитектура проекта (3-слойная)
- **Layer 1**: Real-Time Trading (Rust, Tokio, ORT)
- **Layer 2**: Near-Real-Time Context (Perplexica → Redis)
- **Layer 3**: Offline Training (Python, PyTorch → ONNX)

#### Документация
- **README.md** - обновлена архитектура, добавлен раздел FinBERT SFT
- **docs/OVERVIEW.md** - обновлена структура проекта

---

## [0.2.0] - 2026-02-22

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
  - `lower_price` / `upper_price` - диапазон цен
  - `grid_levels` - количество уровней
  - `order_size` - размер ордера в лотах
  - `grid_ratio` - соотношение buy/sell (0.5 = 50/50)

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
- `trader-bot/src/strategy/grid.rs` - Grid стратегия
- `trader-bot/src/strategy/grid_executor.rs` - Исполнитель
- `trader-bot/src/strategy/grid_bot.rs` - Основной цикл
- `trader-bot/src/config/data.rs` - Конфигурация

#### Зависимости
- Обновлены импорты в `trader-bot/src/main.rs`
- Добавлен `MarketDataService::sdk_clone()` для Grid бота

### Исправлено

- Ошибки времени жизни в `GridStrategy::get_level_by_index()`
- Проблемы с заимствованием в `GridExecutor::on_order_filled()`
- Экспорты модулей в `strategy/mod.rs`

---

## [0.1.5] - 2026-02-22

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

## [0.1.4] - 2026-02-21

### Добавлено

#### MarketDataService
- `get_historical_candles()` - получение свечей из API
- `get_5min_candles()` - 5-минутные свечи
- `get_last_price()` - текущая цена
- Конвертация HistoricCandle → Candle

#### PortfolioService
- `get_accounts()` - список счетов
- `get_portfolio()` - текущий портфель
- `get_available_balance()` - доступный баланс
- `get_position()` - позиция по инструменту

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

## [0.1.3] - 2026-02-20

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
- `make_decision()` - LLM-решение
- `make_rule_based_decision()` - rule-based решение
- Расчет позиции с учетом рисков

---

## [0.1.2] - 2026-02-19

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

## [0.1.1] - 2026-02-18

### Добавлено

- Базовая структура проекта
- Tinkoff Invest SDK интеграция
- Market data streaming
- Ollama Docker container

---

## [0.1.0] - 2026-02-17

### Добавлено

- Initial проект
- Rust workspace
- Docker compose для Ollama
