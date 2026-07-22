# 📚 Документация AI Trade Bot — Созданные файлы

## Обзор

Создана полноценная документация с интеграцией GitLab Pages для автоматического деплоя.

## 📁 Структура файлов

```
ai-trade-bot/
├── core/                           # #![no_std]-compatible core types
├── data-ingestion/                 # WS + REST data fetching
├── perplexica-client/              # Perplexica API polling + cache
├── ml-inference/                   # ONNX model inference (FinBERT, etc.)
├── decision-engine/                # Signal fusion + risk management
├── execution/                      # Order management
├── trader-bot/                     # Binary — wiring, config, main loop
│   ├── src/
│   │   ├── main.rs                 # Точка входа
│   │   ├── core/                   # Broker-agnostic types & traits
│   │   ├── broker/                 # Broker impls (Tinkoff, Mock, Finam)
│   │   ├── datasource/             # Data sources (Tinkoff, Finam)
│   │   ├── ml_inference/           # ONNX inference (FinBERT NLP, TS)
│   │   ├── strategy/               # Trading strategies
│   │   ├── execution/              # Order execution
│   │   ├── client/                 # API clients (market data, portfolio)
│   │   ├── config/                 # Config loading
│   │   └── api/                    # Dashboard (Axum)
│   └── config/                     # Config files
├── training/                       # ML training pipeline
│   └── finbert_sft/                # FinBERT SFT (PyTorch → ONNX)
├── models/                         # ONNX model artifacts
│   └── finbert/                    # FinBERT model.onnx
├── mcp-client/                     # MCP client for LLM (Ollama)
├── ollama-mcp/                     # Docker container with Ollama
├── docs/                           # Документация MkDocs
│   ├── index.md                    # Главная страница
│   ├── README.md                   # Инструкция по работе с docs
│   ├── DEPLOYMENT.md               # Руководство по деплою
│   ├── requirements.txt            # Python зависимости
│   │   ...
├── mkdocs.yml                      # Конфигурация MkDocs
├── .gitlab-ci.yml                  # CI/CD для GitLab Pages
├── README.md                       # Основная документация проекта
├── CHANGELOG.md                    # История изменений
└── GRID_BOT.md                     # Документация Grid бота
```

## 📄 Описание файлов

### Основные файлы документации

| Файл | Описание | Статус |
|------|----------|--------|
| `docs/index.md` | Главная страница документации | ✅ |
| `docs/README.md` | Инструкция по работе с docs/ | ✅ |
| `docs/DEPLOYMENT.md` | Полное руководство по деплою | ✅ |
| `README.md` | Основная документация проекта | ✅ |
| `GRID_BOT.md` | Документация Grid бота | ✅ |
| `CHANGELOG.md` | История изменений | ✅ |

### Разделы документации

#### Getting Started
- `what-is.md` — Обзор возможностей
- `quickstart.md` — Установка и запуск

#### User Guide
- `configuration.md` — Настройка конфигурации

#### Strategies
- `grid-bot.md` — Grid стратегия (полная)

#### Developer Guide
- `api.md` — API референс

### Конфигурационные файлы

| Файл | Назначение | Статус |
|------|------------|--------|
| `mkdocs.yml` | Конфигурация MkDocs Material | ✅ |
| `.gitlab-ci.yml` | CI/CD пайплайн для деплоя | ✅ |
| `docs/requirements.txt` | Python зависимости | ✅ |
| `docs/stylesheets/extra.css` | Кастомные стили | ✅ |

## 🚀 Быстрый старт документации

### 1. Локальная разработка

```bash
# Перейдите в директорию проекта
cd ai-trade-bot

# Установите зависимости
pip install -r docs/requirements.txt

# Запустите локальный сервер
mkdocs serve

# Откройте в браузере
# http://127.0.0.1:8000/
```

### 2. Сборка статической версии

```bash
# Сборка
mkdocs build

# Проверка
ls site/
```

### 3. Деплой на GitLab Pages

```bash
# Пуш в репозиторий
git add .
git commit -m "Add documentation"
git push origin main

# Документация автоматически задеплоится через CI/CD
# URL: https://dark0ghost.gitlab.io/ai-trader-bot/
```

## ⚙️ Настройка GitLab Pages

### Автоматический деплой

После пуша в ветку `main`:
1. GitLab запускает пайплайн
2. Job `pages` собирает документацию
3. Документация публикуется на GitLab Pages

### URL документации

```
https://dark0ghost.gitlab.io/ai-trader-bot/
```

### Версионирование (опционально)

```bash
# Установка mike
pip install mike

# Деплой версии
mike deploy 0.2.0
mike deploy --push 0.2.0
```

## 📊 Статистика документации

- **Всего файлов:** 10 Markdown файлов
- **Разделов:** 4 (Getting Started, User Guide, Strategies, Developer Guide)
- **Конфигурационных файлов:** 4
- **Языки:** Русский

## 🎨 Тема и оформление

Используется **Material for MkDocs** с функциями:
- ✅ Светлая/темная тема
- ✅ Адаптивный дизайн
- ✅ Поиск по документации
- ✅ Навигация с вкладками
- ✅ Диаграммы Mermaid
- ✅ Подсветка синтаксиса
- ✅ Версионирование (mike)

## 📝 Следующие шаги

### Рекомендуется добавить:

1. **Дополнительные руководства:**
   - `user-guide/strategies.md` — Обзор всех стратегий
   - `user-guide/risk-management.md` — Управление рисками
   - `user-guide/monitoring.md` — Мониторинг и логи

2. **Разработчикам:**
   - `developer-guide/architecture.md` — Архитектура проекта
   - `developer-guide/contributing.md` — Как внести вклад
   - `developer-guide/changelog.md` — Ссылка на CHANGELOG.md

3. **Поддержка:**
   - `faq.md` — Часто задаваемые вопросы
   - `troubleshooting.md` — Решение проблем

4. **Ассеты:**
   - `assets/logo.svg` — Логотип проекта
   - `assets/favicon.ico` — Favicon для сайта

## 🔧 Проверка перед деплоем

```bash
# 1. Установка зависимостей
pip install -r docs/requirements.txt

# 2. Локальный предпросмотр
mkdocs serve

# 3. Строгая сборка
mkdocs build --strict

# 4. Проверка ссылок
mkdocs build --clean 2>&1 | grep -i "broken\|error"
```

## 📞 Поддержка

- [GitLab Pages Documentation](https://docs.gitlab.com/ee/user/project/pages/)
- [MkDocs Material](https://squidfunk.github.io/mkdocs-material/)
- [mike Versioning](https://github.com/jimporter/mike)

## ✅ Чеклист готовности

- [x] Структура `docs/` создана
- [x] `mkdocs.yml` настроен
- [x] `.gitlab-ci.yml` создан
- [x] Основные разделы документации написаны
- [x] Инструкции по деплою созданы
- [x] Python зависимости указаны
- [x] Кастомные стили добавлены
- [ ] Ассеты (логотип, favicon) — **требуется добавить**
- [ ] Дополнительные страницы — **опционально**

---

**Дата создания:** 22 февраля 2026  
**Версия документации:** 0.2.0  
**Статус:** Готова к деплою ✅
