mod config;
mod utils;
mod strategy;
mod client;
mod mcp;
mod instrument;
mod analysis;
mod agent;
mod execution;

use anyhow::{anyhow, Result};
use std::env;
use t_invest_sdk::api::{
    FindInstrumentRequest, InstrumentType,
};
use t_invest_sdk::{Environment, TInvestSdk};

use config::{TradingConfig, StrategyType};
use analysis::{NewsAnalyzer, TechnicalAnalyzer, FundamentalAnalyzer, FundamentalDataService, NewsLLMService, NewsItem, NewsSentiment, Sentiment};
use agent::{TradingAgent, DecisionContext};
use execution::{PositionManager, TradingExecutor};
use client::{MarketDataService, PortfolioService};
use strategy::{GridStrategy, GridExecutor, GridBot, GridBotConfig};
use mcp_client::ollama::OllamaProvider;

#[tokio::main]
async fn main() -> Result<()> {
    // Инициализация логгера
    env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or("info")
    ).init();

    log::info!("Запуск AI Trading Bot...");

    // Загрузка конфигурации
    let config = TradingConfig::load_default()
        .map_err(|e| anyhow!("Failed to load config: {}", e))?;
    
    log::info!("Конфигурация загружена. Режим: {:?}", config.mode);

    // Получение токена API
    let token = if config.creditional.token.is_empty() {
        env::var("API_TOKEN")?
    } else {
        config.creditional.token.clone()
    };

    // Инициализация SDK
    let sdk = TInvestSdk::new(&token, match config.mode {
        config::WorkingMode::Prod => Environment::Production,
        config::WorkingMode::Sandbox => Environment::Sandbox,
    }).await?;
    
    let mut instruments_service_client = sdk.instruments();

    // Инициализация LLM
    let llm_config = config.llm_config.as_ref();
    let ollama_provider = if let Some(llm_cfg) = llm_config {
        OllamaProvider::new(
            llm_cfg.model.clone(),
            llm_cfg.host.clone(),
            llm_cfg.port,
        )
    } else {
        OllamaProvider::default()
    };
    
    let model_name = llm_config.map(|c| c.model.clone()).unwrap_or_else(|| "fin-expert".to_string());
    log::info!("LLM модель: {}", model_name);

    // Инициализация анализаторов
    let news_analyzer = NewsAnalyzer::new(vec![
        "tinkoff".to_string(),
        "investing".to_string(),
    ]);
    let technical_analyzer = TechnicalAnalyzer::new();
    let fundamental_analyzer = FundamentalAnalyzer::default();

    // Инициализация LLM-сервиса для анализа новостей
    let news_llm_service = NewsLLMService::new(ollama_provider.clone());

    // Инициализация сервисов
    let market_data_service = MarketDataService::new(sdk.clone());
    let portfolio_service = PortfolioService::new(sdk.clone(), "main".to_string());
    let fundamental_data_service = FundamentalDataService::new();

    // Получение доступного баланса
    let available_balance = match portfolio_service.get_available_balance().await {
        Ok(balance) => {
            log::info!("Доступный баланс: {:.2}", balance);
            balance
        }
        Err(e) => {
            log::warn!("Ошибка получения баланса: {}", e);
            1_000_000.0 // Запасное значение
        }
    };

    // Инициализация агента и исполнителя
    let trading_agent = TradingAgent::new(ollama_provider, model_name);
    let position_manager = PositionManager::new(sdk.clone(), "main".to_string());
    let trading_executor = TradingExecutor::new(position_manager, available_balance);

    // Проверка на наличие Grid стратегии в конфигурации
    let grid_accounts: Vec<_> = config.accounts.iter()
        .filter(|acc| matches!(acc.strategy.strategy, StrategyType::Grid))
        .collect();
    
    if !grid_accounts.is_empty() {
        log::info!("Найдено {} аккаунтов с Grid стратегией", grid_accounts.len());
        
        // Запуск Grid ботов для каждого аккаунта
        for account in grid_accounts {
            if let Some(grid_config) = &account.strategy.parameters.grid_config {
                for instrument in account.instruments.iter().filter(|i| i.enabled) {
                    log::info!(
                        "Запуск Grid бота для {} ({}): диапазон {:.2}-{:.2}, уровней={}",
                        instrument.ticker,
                        instrument.figi,
                        grid_config.lower_price,
                        grid_config.upper_price,
                        grid_config.grid_levels
                    );
                    
                    let bot_config = GridBotConfig {
                        account_id: account.account_id.clone(),
                        figi: instrument.figi.clone(),
                        ticker: instrument.ticker.clone(),
                        grid_config: grid_config.clone(),
                        check_interval_secs: account.strategy.parameters.check_interval as u64,
                    };
                    
                    let mut grid_bot = GridBot::new(sdk.clone(), bot_config);
                    
                    // Запуск в отдельной задаче
                    tokio::spawn(async move {
                        if let Err(e) = grid_bot.run().await {
                            log::error!("Ошибка Grid бота: {}", e);
                        }
                    });
                }
            }
        }
    }

    // Получение активных инструментов для обычной стратегии
    let enabled_instruments = config.get_enabled_instruments();
    log::info!("Активных инструментов для анализа: {}", enabled_instruments.len());

    // Основной цикл торговли
    for instrument in enabled_instruments {
        if !instrument.enabled {
            continue;
        }

        log::info!("Анализ инструмента: {} ({})", instrument.ticker, instrument.name);

        // Поиск инструмента в API
        let find_instrument_response = instruments_service_client
            .find_instrument(FindInstrumentRequest {
                query: instrument.name.clone(),
                instrument_kind: Some(InstrumentType::Share as i32),
                api_trade_available_flag: Some(true),
            })
            .await?
            .into_inner();

        let found_instrument = find_instrument_response
            .instruments
            .first()
            .ok_or_else(|| anyhow!("Инструмент не найден: {}", instrument.ticker))?;

        log::info!("Найден инструмент: FIGI={}", found_instrument.figi);

        // Получение исторических данных для технического анализа
        // Интервал зависит от стратегии - для внутридневной торговли используем 5-минутные свечи
        let days_for_analysis = config.accounts.first()
            .map(|a| a.strategy.parameters.days_back_to_consider)
            .unwrap_or(30);
        
        log::info!("Загрузка свечей за {} дней...", days_for_analysis);
        
        let candles = match market_data_service.get_5min_candles(&found_instrument.figi, days_for_analysis).await {
            Ok(candles) => {
                log::info!("Загружено {} свечей", candles.len());
                candles
            }
            Err(e) => {
                log::warn!("Ошибка загрузки свечей: {}", e);
                vec![]
            }
        };

        // Получение текущей цены
        let current_price = match market_data_service.get_last_price(&found_instrument.figi).await {
            Ok(price) => {
                log::info!("Текущая цена: {:.2}", price);
                price
            }
            Err(e) => {
                log::warn!("Ошибка получения текущей цены: {}", e);
                0.0
            }
        };

        // Комплексный анализ
        log::info!("Выполнение анализа для {}...", instrument.ticker);

        // 1. Технический анализ
        let tech_analysis = if !candles.is_empty() {
            match technical_analyzer.analyze(&instrument.ticker, &candles) {
                Ok(analysis) => {
                    log::info!(
                        "Технический анализ: тренд={:?}, рекомендация={:?}",
                        analysis.trend,
                        analysis.recommendation
                    );
                    Some(analysis)
                }
                Err(e) => {
                    log::warn!("Ошибка технического анализа: {}", e);
                    None
                }
            }
        } else {
            log::warn!("Нет данных для технического анализа");
            None
        };

        // 2. Анализ новостей с LLM
        let news_sentiment = if instrument.analysis_config.check_news {
            log::info!("Сбор новостей для {}...", instrument.ticker);
            
            // Собираем новости через NewsAnalyzer
            match news_analyzer.analyze(&instrument.ticker, &instrument.name).await {
                Ok(base_sentiment) => {
                    // Если есть новости, используем LLM для углубленного анализа
                    if base_sentiment.articles_count > 0 {
                        log::info!("Найдено {} новостей, анализ через LLM...", base_sentiment.articles_count);
                        
                        // Конвертируем статьи в NewsItem для LLM
                        let news_items: Vec<NewsItem> = base_sentiment.articles
                            .iter()
                            .take(5) // Анализируем только первые 5 новостей
                            .map(|article| NewsItem {
                                title: article.title.clone(),
                                content: article.content.clone(),
                                source: article.source.clone(),
                                url: article.url.clone(),
                            })
                            .collect();
                        
                        // LLM-анализ
                        match news_llm_service.analyze_news_batch(&instrument.ticker, &instrument.name, &news_items).await {
                            Ok(llm_result) => {
                                log::info!(
                                    "LLM-анализ: {:?} (score: {:.2}, confidence: {:.2})",
                                    llm_result.overall_sentiment,
                                    llm_result.sentiment_score,
                                    llm_result.confidence
                                );
                                log::info!("Резюме: {}", llm_result.summary);
                                
                                if !llm_result.key_events.is_empty() {
                                    log::info!("Ключевые события: {}", llm_result.key_events.join(", "));
                                }
                                if !llm_result.risks.is_empty() {
                                    log::warn!("Риски: {}", llm_result.risks.join(", "));
                                }
                                
                                // Конвертация Sentiment из news_llm в news
                                let overall_sentiment = match llm_result.overall_sentiment {
                                    analysis::news_llm::Sentiment::Positive => Sentiment::Positive,
                                    analysis::news_llm::Sentiment::Negative => Sentiment::Negative,
                                    analysis::news_llm::Sentiment::Neutral => Sentiment::Neutral,
                                };
                                
                                // Возвращаем обогащенный результат
                                Some(NewsSentiment {
                                    ticker: instrument.ticker.clone(),
                                    overall_sentiment,
                                    sentiment_score: llm_result.sentiment_score,
                                    articles_count: base_sentiment.articles_count,
                                    articles: base_sentiment.articles,
                                    key_events: llm_result.key_events,
                                })
                            }
                            Err(e) => {
                                log::warn!("Ошибка LLM-анализа: {}", e);
                                Some(base_sentiment)
                            }
                        }
                    } else {
                        log::info!("Новости не найдены");
                        Some(base_sentiment)
                    }
                }
                Err(e) => {
                    log::warn!("Ошибка сбора новостей: {}", e);
                    None
                }
            }
        } else {
            None
        };

        // 3. Фундаментальный анализ
        let fundamental_analysis = if instrument.analysis_config.fundamental_analysis {
            log::info!("Фундаментальный анализ для {}...", instrument.ticker);
            match fundamental_data_service.get_fundamental_data(&instrument.ticker, &instrument.name).await {
                Ok(Some(analysis)) => {
                    log::info!(
                        "Фундаментальный анализ: рейтинг={:?}, score={:.1}",
                        analysis.rating,
                        analysis.overall_score
                    );
                    if !analysis.key_strengths.is_empty() {
                        log::info!("Сильные стороны: {}", analysis.key_strengths.join(", "));
                    }
                    if !analysis.key_risks.is_empty() {
                        log::warn!("Риски: {}", analysis.key_risks.join(", "));
                    }
                    Some(analysis)
                }
                Ok(None) => {
                    log::info!("Фундаментальные данные недоступны для {}", instrument.ticker);
                    None
                }
                Err(e) => {
                    log::warn!("Ошибка фундаментального анализа: {}", e);
                    None
                }
            }
        } else {
            None
        };

        // 4. Принятие решения агентом
        let use_price = if current_price > 0.0 {
            current_price
        } else if let Some(tech) = &tech_analysis {
            tech.current_price
        } else {
            log::warn!("Нет данных о цене для {}", instrument.ticker);
            continue;
        };

        // Получение текущей позиции по инструменту
        let current_position = match portfolio_service.get_position(&found_instrument.figi).await {
            Ok(Some(pos)) => {
                log::info!("Текущая позиция: {} лотов, средняя цена: {:.2}", pos.quantity, pos.average_price);
                Some(pos)
            }
            Ok(None) => None,
            Err(e) => {
                log::warn!("Ошибка получения позиции: {}", e);
                None
            }
        };

        let context = DecisionContext {
            ticker: instrument.ticker.clone(),
            company_name: instrument.name.clone(),
            current_price: use_price,
            news_sentiment: news_sentiment.clone(),
            technical_analysis: tech_analysis.clone(),
            fundamental_analysis: fundamental_analysis.clone(),
            available_balance,
            current_position,
            risk_config: config.accounts.first().and_then(|a| a.risk_management.clone()),
            max_position_pct: instrument.max_position_pct,
        };

        log::info!("Принятие решения агентом...");

        // Решение агентом: LLM или rule-based
        let decision = if config.llm_config.is_some() {
            // Пробуем использовать LLM
            log::info!("Использование LLM для принятия решения...");
            match trading_agent.make_decision(context.clone()).await {
                Ok(dec) => {
                    log::info!("LLM приняла решение");
                    dec
                }
                Err(e) => {
                    log::warn!("Ошибка LLM, используем rule-based: {}", e);
                    trading_agent.make_rule_based_decision(context)?
                }
            }
        } else {
            // Rule-based решение
            trading_agent.make_rule_based_decision(context)?
        };

        log::info!(
            "Решение агента: {:?} (confidence: {:.2}, позиция: {:.1}%)",
            decision.action,
            decision.confidence,
            decision.position_size_pct * 100.0
        );
        log::info!("Обоснование: {}", decision.rationale);

        if !decision.risks.is_empty() {
            log::warn!("Риски: {}", decision.risks.join(", "));
        }

        // 5. Исполнение решения
        if decision.action != agent::Action::Hold && decision.confidence >= 0.6 {
            log::info!("Исполнение решения...");

            match trading_executor.execute_decision(&decision, &found_instrument.figi).await {
                Ok(results) => {
                    for result in results {
                        log::info!(
                            "Заявка размещена: ID={}, статус={:?}",
                            result.order_id,
                            result.status
                        );
                    }
                }
                Err(e) => {
                    log::error!("Ошибка исполнения: {}", e);
                }
            }
        } else {
            log::info!("Решение не исполняется: HOLD или низкая уверенность");
        }

        // Пауза между инструментами
        tokio::time::sleep(tokio::time::Duration::from_secs(
            config.accounts.first()
                .map(|a| a.strategy.parameters.check_interval as u64)
                .unwrap_or(60)
        )).await;
    }

    log::info!("Цикл анализа завершен");

    Ok(())
}
