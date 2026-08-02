---
hide:
  - navigation
---

# Market Bot Documentation

Добро пожаловать в документацию Market Bot - комплексной торговой системы с поддержкой множества стратегий (AI, Grid, Gates) и брокеров для Московской биржи.

## 🚀 Быстрый старт

### Установка за 5 минут

```bash
# 1. Клонируйте репозиторий
git clone https://github.com/dark0ghost/ai-trader-bot.git
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

- **[Что такое Market Bot?](getting-started/what-is.md)** - Обзор возможностей
- **[Быстрый старт](getting-started/quickstart.md)** - Установка и запуск

### Для пользователей

- **[Конфигурация](user-guide/configuration.md)** - Настройка бота

### Для разработчиков

- **[API документация](developer-guide/api.md)** - API референс

### Стратегии

- **[Grid Bot](strategies/grid-bot.md)** - Сеточная торговля

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

- [GitHub Issues](https://github.com/dark0ghost/ai-trader-bot/issues) - Сообщить об ошибке

## 📝 Лицензия

MIT License - см. [LICENSE](../LICENSE)

---

**Последнее обновление:** 22 февраля 2026  
**Версия документации:** 0.2.0
