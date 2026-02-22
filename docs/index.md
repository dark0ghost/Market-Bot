---
hide:
  - navigation
---

# AI Trade Bot Documentation

Добро пожаловать в документацию AI Trade Bot — комплексной торговой системы с искусственным интеллектом для торговли на Московской бирже.

## 🚀 Быстрый старт

### Установка за 5 минут

```bash
# 1. Клонируйте репозиторий
git clone https://gitlab.com/your-username/ai-trade-bot.git
cd ai-trade-bot

# 2. Настройте токен
export API_TOKEN="your_tinkoff_token"

# 3. Запустите Ollama (для LLM-анализа)
docker-compose up -d

# 4. Запустите бота
cargo run -p trader-bot
```

## 📚 Разделы документации

### Для начинающих

- **[Что такое AI Trade Bot?](getting-started/what-is.md)** — Обзор возможностей
- **[Быстрый старт](getting-started/quickstart.md)** — Установка и запуск
- **[Первые шаги](getting-started/first-steps.md)** — Настройка конфигурации

### Для пользователей

- **[Руководство пользователя](user-guide/introduction.md)** — Полное руководство
- **[Конфигурация](user-guide/configuration.md)** — Настройка бота
- **[Торговые стратегии](user-guide/strategies.md)** — Описание стратегий
- **[Управление рисками](user-guide/risk-management.md)** — Настройка рисков

### Для разработчиков

- **[Архитектура](developer-guide/architecture.md)** — Архитектура проекта
- **[API документация](developer-guide/api.md)** — API референс
- **[Вклад в проект](developer-guide/contributing.md)** — Как внести вклад

### Стратегии

- **[Grid Bot](strategies/grid-bot.md)** — Сеточная торговля
- **[Interval Strategy](strategies/interval.md)** — Интервальная торговля
- **[Momentum](strategies/momentum.md)** — Торговля по тренду
- **[Mean Reversion](strategies/mean-reversion.md)** — Возврат к среднему

## 📊 Возможности

| Возможность | Описание | Статус |
|------------|----------|--------|
| AI Анализ | LLM-анализ новостей и принятие решений | ✅ |
| Технический анализ | RSI, MACD, Bollinger Bands | ✅ |
| Фундаментальный анализ | P/E, ROE, D/E, рост | ✅ |
| Grid стратегия | Автоматическая сетка ордеров | ✅ |
| Управление рисками | Stop Loss, Take Profit, лимиты | ✅ |
| Tinkoff API | Интеграция с Московской биржей | ✅ |

## 🆘 Поддержка

- [FAQ](faq.md) — Часто задаваемые вопросы
- [Troubleshooting](troubleshooting.md) — Решение проблем
- [GitHub Issues](https://github.com/your-username/ai-trade-bot/issues) — Сообщить об ошибке

## 📝 Лицензия

MIT License — см. [LICENSE](../LICENSE)

---

**Последнее обновление:** 22 февраля 2026  
**Версия документации:** 0.2.0
