# Perplexica Integration

Интеграция с **Perplexica** — AI-ориентированной поисковой системой для поиска информации о компаниях, новостей и аналитики.

## Обзор

[Perplexica](https://github.com/ItzCrazyKns/Perplexica) — это open-source AI-поисковик, который работает на вашем оборудовании и предоставляет точные ответы с цитированием источников.

## Требования

- Запущенный экземпляр Perplexica (порт 3000 по умолчанию)
- Настроенные провайдеры LLM и embedding-моделей в Perplexica

## Быстрый старт

### 1. Запуск Perplexica

```bash
docker run -d -p 3000:3000 -v perplexica-data:/home/perplexica/data --name perplexica itzcrazykns1337/perplexica:latest
```

### 2. Использование в коде

```rust
use mcp_client::perplexica::{PerplexicaProvider, PerplexicaSearcher, ModelConfig};

// Конфигурация моделей
let chat_model = ModelConfig {
    provider_id: "your-provider-uuid".to_string(),
    key: "gpt-4o-mini".to_string(),
};

let embedding_model = ModelConfig {
    provider_id: "your-provider-uuid".to_string(),
    key: "text-embedding-3-large".to_string(),
};

// Создание провайдера
let provider = PerplexicaProvider::new(chat_model.clone(), embedding_model.clone());

// Поиск информации о компании
let result = provider.search_company("AAPL", "Apple Inc.").await?;
println!("Ответ: {}", result.answer);
println!("Источники: {:?}", result.sources);
```

## API

### PerplexicaProvider

Основной класс для работы с Perplexica API.

#### Создание

```rust
// С URL по умолчанию (http://localhost:3000)
let provider = PerplexicaProvider::new(chat_model, embedding_model);

// С кастомным URL
let provider = PerplexicaProvider::with_url(
    "http://custom-host:3000",
    chat_model,
    embedding_model,
);
```

#### Методы

##### `search(query: &str) -> Result<SearchResult>`

Базовый поисковый запрос.

```rust
let result = provider.search("ИИ в трейдинге").await?;
```

##### `search_company(ticker: &str, company_name: &str) -> Result<SearchResult>`

Поиск информации о компании: финансовые показатели, новости, аналитика.

```rust
let result = provider.search_company("TCSG", "Тинькофф").await?;
```

##### `search_news(ticker: &str) -> Result<SearchResult>`

Поиск последних новостей по тикеру.

```rust
let result = provider.search_news("AAPL").await?;
```

##### `search_analyst_ratings(ticker: &str, company_name: &str) -> Result<SearchResult>`

Поиск аналитических рейтингов и целевых цен.

```rust
let result = provider.search_analyst_ratings("MSFT", "Microsoft").await?;
```

##### `search_with_options(...) -> Result<SearchResult>`

Расширенный поиск с настройками.

```rust
use mcp_client::perplexica::{SearchSource, OptimizationMode};

let result = provider.search_with_options(
    "query",
    Some(vec![SearchSource::Web, SearchSource::Academic]),
    Some("Специальные инструкции"),
    None, // history
    Some(OptimizationMode::Quality),
).await?;
```

### PerplexicaSearcher

Удобный интерфейс для поиска с форматированным выводом.

```rust
let searcher = PerplexicaSearcher::new(chat_model, embedding_model);

// Поиск информации о компании
let info = searcher.search_company_info("AAPL".into(), "Apple".into()).await?;
println!("{}", info);

// Поиск новостей
let news = searcher.search_latest_news("TCSG".into()).await?;
println!("{}", news);

// Поиск аналитики
let ratings = searcher.search_analyst_ratings("MSFT".into(), "Microsoft".into()).await?;
println!("{}", ratings);

// Общий поиск
let results = searcher.search("ИИ в финансах".into()).await?;
println!("{}", results);
```

## Структуры данных

### ModelConfig

```rust
pub struct ModelConfig {
    pub provider_id: String,  // UUID провайдера из /api/providers
    pub key: String,          // Ключ модели (например, "gpt-4o-mini")
}
```

### SearchResult

```rust
pub struct SearchResult {
    pub answer: String,           // AI-ответ
    pub sources: Vec<SourceInfo>, // Список источников
}

pub struct SourceInfo {
    pub title: String,    // Заголовок
    pub url: String,      // URL
    pub snippet: String,  // Сниппет контента
}
```

### SearchSource

Источники поиска:

- `SearchSource::Web` — веб-страницы
- `SearchSource::Academic` — академические источники
- `SearchSource::Discussions` — обсуждения и форумы

### OptimizationMode

Режим оптимизации:

- `OptimizationMode::Speed` — скорость
- `OptimizationMode::Balanced` — баланс (по умолчанию)
- `OptimizationMode::Quality` — качество

## Интеграция с LLM Tools

PerplexicaSearcher может использоваться как инструмент для LLM:

```rust
use mcp_client::perplexica::{PerplexicaSearcher, ModelConfig};
use ollama_rs::coordinator::Coordinator;

let searcher = PerplexicaSearcher::new(chat_model, embedding_model);

// LLM может вызывать методы searcher для поиска информации
let company_info = searcher.search_company_info("AAPL".into(), "Apple".into()).await?;

// Передача информации в LLM для анализа
let llm_response = coordinator.chat(vec![
    ChatMessage::user(format!("Проанализируй компанию: {}", company_info))
]).await?;
```

## Примеры использования

### Анализ компании перед сделкой

```rust
async fn analyze_company(ticker: &str, name: &str) -> Result<()> {
    let searcher = PerplexicaSearcher::new(chat_model, embedding_model);
    
    // Получаем информацию о компании
    let info = searcher.search_company_info(ticker.into(), name.into()).await?;
    
    // Получаем последние новости
    let news = searcher.search_latest_news(ticker.into()).await?;
    
    // Получаем аналитические рейтинги
    let ratings = searcher.search_analyst_ratings(ticker.into(), name.into()).await?;
    
    println!("=== {}\n{}\n{}\n{}", ticker, info, news, ratings);
    Ok(())
}
```

### Мониторинг новостей по портфелю

```rust
async fn monitor_portfolio(portfolio: &[(&str, &str)]) -> Result<()> {
    let searcher = PerplexicaSearcher::new(chat_model, embedding_model);
    
    for (ticker, name) in portfolio {
        let news = searcher.search_latest_news(ticker.to_string()).await?;
        println!("{} ({}): {}\n", name, ticker, news);
    }
    
    Ok(())
}
```

## Конфигурация Perplexica

### Получение списка провайдеров

```bash
curl http://localhost:3000/api/providers
```

Ответ:
```json
{
  "providers": [
    {
      "id": "550e8400-e29b-41d4-a716-446655440000",
      "name": "OpenAI",
      "chatModels": [
        { "name": "GPT 4 Omni Mini", "key": "gpt-4o-mini" }
      ],
      "embeddingModels": [
        { "name": "Text Embedding 3 Large", "key": "text-embedding-3-large" }
      ]
    }
  ]
}
```

### Пример запроса к API

```bash
curl -X POST http://localhost:3000/api/search \
  -H "Content-Type: application/json" \
  -d '{
    "chatModel": {
      "providerId": "550e8400-e29b-41d4-a716-446655440000",
      "key": "gpt-4o-mini"
    },
    "embeddingModel": {
      "providerId": "550e8400-e29b-41d4-a716-446655440000",
      "key": "text-embedding-3-large"
    },
    "sources": ["web"],
    "query": "Apple Inc. stock analysis",
    "optimizationMode": "balanced",
    "stream": false
  }'
```

## Тесты

```bash
cargo test -p mcp-client
```

## Ссылки

- [GitHub репозиторий Perplexica](https://github.com/ItzCrazyKns/Perplexica)
- [Документация API](https://github.com/ItzCrazyKns/Perplexica/tree/master/docs/API)
