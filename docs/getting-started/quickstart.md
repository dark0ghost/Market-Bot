# Быстрый старт

Это руководство поможет вам запустить AI Trade Bot за 5 минут.

## Шаг 1: Установка зависимостей

### Rust

```bash
# Linux/macOS
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Проверка установки
rustc --version
```

### Docker (опционально, для Ollama)

```bash
# Ubuntu/Debian
curl -fsSL https://get.docker.com -o get-docker.sh
sudo sh get-docker.sh

# Проверка
docker --version
```

## Шаг 2: Клонирование репозитория

```bash
git clone https://gitlab.com/your-username/ai-trade-bot.git
cd ai-trade-bot
```

## Шаг 3: Получение Tinkoff Invest токена

1. Войдите в [Tinkoff Invest](https://www.tbank.ru/invest/settings/)
2. Перейдите в настройки
3. Нажмите "Выпустить токен"
4. Скопируйте токен

## Шаг 4: Настройка конфигурации

### Вариант A: Переменная окружения

```bash
export API_TOKEN="your_token_here"
```

### Вариант B: Файл конфигурации

Отредактируйте `trader-bot/config/account.json`:

```json
{
  "creditional": {
    "token": "your_token_here"
  },
  "mode": "sandbox"
}
```

## Шаг 5: Запуск Ollama (опционально)

Для LLM-анализа новостей:

```bash
docker-compose up -d
```

Проверка:

```bash
curl http://localhost:11435/api/tags
```

## Шаг 6: Запуск бота

### Тестовый режим (рекомендуется)

```bash
cargo run -p trader-bot
```

### С логированием

```bash
RUST_LOG=info cargo run -p trader-bot
```

### Подробные логи

```bash
RUST_LOG=debug cargo run -p trader-bot
```

## Проверка работы

Вы должны увидеть логи:

```
[INFO] Запуск AI Trading Bot...
[INFO] Конфигурация загружена. Режим: Sandbox
[INFO] LLM модель: fin-expert
[INFO] Активных инструментов: 2
[INFO] Анализ инструмента: Т-Технологии (TTECH)
[INFO] Найден инструмент: FIGI=TQBR
[INFO] Загрузка свечей за 30 дней...
[INFO] Загружено 8640 свечей
```

## Первые сделки

Бот начнет анализировать рынок и принимать решения:

```
[INFO] Технический анализ: тренд=Bullish, рекомендация=Buy
[INFO] Новостной фон: Positive (score: 0.65)
[INFO] Решение агента: Buy (confidence: 0.75, позиция: 5.0%)
[INFO] Размещение BUY заявки: 10 лотов по цене 275.50
[INFO] Заявка размещена: ID=12345, статус=New
```

## Остановка бота

Нажмите `Ctrl+C` для остановки.

## Следующие шаги

- **[Конфигурация](../user-guide/configuration.md)** — Детальная настройка
- **[Стратегии](../strategies/grid-bot.md)** — Выбор стратегии
- **[Управление рисками](../user-guide/risk-management.md)** — Настройка рисков

## Частые проблемы

### Ошибка: "Connection refused"

```bash
# Проверьте подключение к интернету
ping tinkoff.ru

# Проверьте токен
echo $API_TOKEN
```

### Ошибка: "Instrument not found"

Проверьте FIGI в конфигурации:

```json
{
  "instruments": [{
    "figi": "TQBR",
    "ticker": "TTECH",
    "enabled": true
  }]
}
```

### Ошибка: "Недостаточно средств"

Уменьшите размер позиции или пополните счет:

```json
{
  "risk_management": {
    "max_position_pct": 0.05
  }
}
```
