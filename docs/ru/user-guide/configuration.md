# Конфигурация

Файл конфигурации `account.json` определяет все параметры работы бота.

## Структура файла

```json
{
  "type": "trading",
  "creditional": { "token": "..." },
  "accounts": [...],
  "mode": "sandbox",
  "llm_config": {...}
}
```

## Основные параметры

### Корневые параметры

| Параметр | Тип | Обязательный | Описание |
|----------|-----|--------------|----------|
| `type` | string | ✅ | Тип конфигурации ("trading") |
| `creditional.token` | string | ✅ | Tinkoff Invest токен |
| `accounts` | array | ✅ | Массив аккаунтов |
| `mode` | string | ✅ | Режим ("sandbox" или "prod") |
| `llm_config` | object | ❌ | Настройки LLM |

### Account Config

| Параметр | Тип | Описание |
|----------|-----|----------|
| `account_id` | string | ID аккаунта |
| `instruments` | array | Инструменты для торговли |
| `strategy` | object | Настройки стратегии |
| `risk_management` | object | Управление рисками |

### Instrument Config

| Параметр | Тип | Описание |
|----------|-----|----------|
| `figi` | string | FIGI инструмента |
| `ticker` | string | Тикер |
| `name` | string | Название |
| `enabled` | bool | Активен ли |
| `max_position_pct` | f64 | Макс. доля от портфеля |
| `analysis_config` | object | Настройки анализа |

## Примеры

### Минимальная конфигурация

```json
{
  "type": "trading",
  "creditional": { "token": "YOUR_TOKEN" },
  "accounts": [{
    "account_id": "main",
    "instruments": [{
      "figi": "TQBR",
      "ticker": "TTECH",
      "name": "Т-Технологии",
      "enabled": true,
      "max_position_pct": 0.1
    }],
    "strategy": {
      "strategy": "interval",
      "parameters": {
        "interval_size": "1h",
        "days_back_to_consider": 30,
        "quantity_limit": 1000,
        "check_interval": 60
      }
    }
  }],
  "mode": "sandbox"
}
```

### Grid стратегия

```json
{
  "strategy": {
    "strategy": "grid",
    "parameters": {
      "grid_config": {
        "lower_price": 250.0,
        "upper_price": 300.0,
        "grid_levels": 11,
        "order_size": 10,
        "grid_ratio": 0.5
      }
    }
  }
}
```

### Управление рисками

```json
{
  "risk_management": {
    "max_loss_pct": 0.05,
    "take_profit_pct": 0.10,
    "stop_loss_pct": 0.03,
    "max_open_positions": 5,
    "min_balance_reserve": 100000.0
  }
}
```

### LLM конфигурация

```json
{
  "llm_config": {
    "model": "fin-expert",
    "host": "http://localhost",
    "port": 11435,
    "temperature": 0.3,
    "context_window": 4096
  }
}
```

## Переменные окружения

Вместо указания токена в файле можно использовать переменную окружения:

```bash
export API_TOKEN="your_token"
```

В конфиге оставьте пустым:

```json
{
  "creditional": { "token": "" }
}
```
