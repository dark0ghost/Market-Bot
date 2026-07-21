mod config;
mod utils;
mod provider;
mod strategy;
mod client;
mod mcp;
mod instrument;
mod analysis;
mod agent;
mod execution;
mod error;
mod scanner;
mod backtest;
mod stream;
mod storage;
mod telemetry;
mod scheduler;

// ─── New Architecture ────────────────────────────────────────────────
mod core;
mod broker;
mod datasource;
mod optimizer;
mod api;

use anyhow::{anyhow, Result};
use std::env;
use std::sync::Arc;
use tokio::sync::Mutex;
use t_invest_sdk::api::{
    FindInstrumentRequest, InstrumentType,
};
use t_invest_sdk::{Environment, TInvestSdk};

use config::{TradingConfig, StrategyType};
use analysis::{NewsAnalyzer, TechnicalAnalyzer, FundamentalAnalyzer, FundamentalDataService, NewsLLMService, NewsItem, NewsSentiment, Sentiment, MarketRegime, RegimeDetector};
use agent::{TradingAgent, DecisionContext};
use execution::{PositionManager, TradingExecutor};
use client::{MarketDataService, PortfolioService};
use strategy::{GridStrategy, GridExecutor, GridBot, GridBotConfig};

use mcp_client::ollama::OllamaProvider;

// New imports
use core::*;
use broker::TinkoffBroker;
use datasource::{TinkoffDataSource, DataSourceRegistry};
use strategy::registry::StrategyRegistry;

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or("info")
    ).init();

    log::info!("╔══════════════════════════════════════════╗");
    log::info!("║     AI Trade Bot v{} (refactored)      ║", env!("CARGO_PKG_VERSION"));
    log::info!("╚══════════════════════════════════════════╝");

    // ── 1. Load Configuration ────────────────────────────────────────
    let config = TradingConfig::load_default()
        .map_err(|e| anyhow!("Failed to load config: {}", e))?;

    log::info!("Configuration loaded. Mode: {:?}", config.mode);

    let token = if config.credential.token.is_empty() {
        env::var("API_TOKEN")?
    } else {
        config.credential.token.clone()
    };

    let env_mode = match config.mode {
        config::WorkingMode::Prod => Environment::Production,
        config::WorkingMode::Sandbox => Environment::Sandbox,
    };

    // ── 2. Initialize SDK & Broker ────────────────────────────────────
    let sdk = TInvestSdk::new(&token, env_mode).await?;

    // Wrap Tinkoff SDK in our Broker trait
    let tinkoff_broker = Arc::new(TinkoffBroker::from_sdk(sdk.clone(), "main".to_string()));

    // ── 3. Initialize Data Sources ────────────────────────────────────
    let mut data_source_registry = DataSourceRegistry::new();
    let tinkoff_data = Arc::new(TinkoffDataSource::new(sdk.clone()));
    data_source_registry.register(tinkoff_data);
    log::info!("Data sources: {:?}", data_source_registry.list_names());

    // ── 4. Initialize LLM ─────────────────────────────────────────────
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
    log::info!("LLM model: {}", model_name);

    // ── 5. Initialize Analysis Services ───────────────────────────────
    let news_analyzer = NewsAnalyzer::new(vec![
        "tinkoff".to_string(),
        "investing".to_string(),
    ]);
    let technical_analyzer = TechnicalAnalyzer::new();
    let fundamental_analyzer = FundamentalAnalyzer::default();
    let news_llm_service = NewsLLMService::new(ollama_provider.clone());
    let fundamental_data_service = FundamentalDataService::new();
    let mut regime_detector = RegimeDetector::new(14, 14);

    // ── 6. Initialize Trading Services ────────────────────────────────
    let market_data_service = MarketDataService::new(sdk.clone());
    let portfolio_service = PortfolioService::new(sdk.clone(), "main".to_string());

    // ── 7. Initialize Strategy Registry ───────────────────────────────
    let mut strategy_registry = StrategyRegistry::new();

    // Register Grid strategy for each account with grid config
    let grid_accounts: Vec<_> = config.accounts.iter()
        .filter(|acc| matches!(acc.strategy.strategy, StrategyType::Grid))
        .collect();

    for account in &grid_accounts {
        if let Some(grid_cfg) = &account.strategy.parameters.grid_config {
            let grid_strategy = crate::strategy::grid::GridStrategy::new(grid_cfg.clone());
            strategy_registry.register(Box::new(grid_strategy));
            log::info!("Grid strategy registered for account {}", account.account_id);
        }
    }
    log::info!("Strategies registered: {:?}", strategy_registry.list_names());

    // ── 8. Get Balance ────────────────────────────────────────────────
    let available_balance = match portfolio_service.get_available_balance().await {
        Ok(balance) => {
            log::info!("Available balance: {:.2}", balance);
            balance
        }
        Err(e) => {
            log::error!("Balance fetch error: {}", e);
            std::env::var("DEFAULT_BALANCE")
                .ok()
                .and_then(|s| s.parse::<f64>().ok())
                .unwrap_or_else(|| {
                    log::error!("DEFAULT_BALANCE not set. Exiting.");
                    std::process::exit(1);
                })
        }
    };

    // ── 9. Start Dashboard if enabled ──────────────────────────────────
    if let Some(dashboard_cfg) = &config.dashboard {
        if dashboard_cfg.enabled {
            let api_state = Arc::new(Mutex::new(api::AppState {
                brokers: vec![tinkoff_broker.clone() as Arc<dyn Broker>],
                data_sources: data_source_registry,
                strategies: strategy_registry,
            }));
            let port = dashboard_cfg.port;
            tokio::spawn(async move {
                if let Err(e) = api::start_dashboard(api_state, port).await {
                    log::error!("Dashboard error: {}", e);
                }
            });
            log::info!("Dashboard started on port {}", port);
        }
    }

    // Re-borrow strategy_registry for later use
    // (it was moved into api_state, so we re-create for main loop)
    let mut strategy_registry = StrategyRegistry::new();
    for account in &grid_accounts {
        if let Some(grid_cfg) = &account.strategy.parameters.grid_config {
            let grid_strategy = crate::strategy::grid::GridStrategy::new(grid_cfg.clone());
            strategy_registry.register(Box::new(grid_strategy));
        }
    }

    // ── 10. Run Optimizer if enabled ───────────────────────────────────
    if let Some(opt_cfg) = &config.optimizer {
        if opt_cfg.enabled {
            log::info!("Running optimizer...");
            let opt_method = if opt_cfg.method == "random_search" {
                OptimizationMethod::RandomSearch
            } else {
                OptimizationMethod::GridSearch
            };
            let opt_metric = match opt_cfg.metric.as_str() {
                "total_return" => OptimizationMetric::TotalReturn,
                "win_rate" => OptimizationMetric::WinRate,
                "calmar_ratio" => OptimizationMetric::CalmarRatio,
                "profit_factor" => OptimizationMetric::ProfitFactor,
                _ => OptimizationMetric::SharpeRatio,
            };

            // Example: optimize a simple SMA crossover strategy
            let opt_config = OptimizerConfig {
                param_ranges: vec![
                    ParamRange { name: "fast_period".to_string(), min: 5.0, max: 50.0, step: 5.0 },
                    ParamRange { name: "slow_period".to_string(), min: 20.0, max: 200.0, step: 10.0 },
                ],
                metric: opt_metric,
                method: opt_method,
                max_iterations: opt_cfg.max_iterations,
            };

            // Get some candles for optimization
            if let Some(instrument) = config.get_enabled_instruments().first() {
                if let Ok(candles) = market_data_service.get_5min_candles(&instrument.figi, 30).await {
                    let core_candles: Vec<Candle> = candles.iter().filter_map(|c| {
                        let time = c.time.as_ref().and_then(|t| chrono::DateTime::from_timestamp(t.seconds, t.nanos as u32))?;
                        Some(Candle {
                            open: crate::client::market_data::extract_price(&c.open).ok()?,
                            high: crate::client::market_data::extract_price(&c.high).ok()?,
                            low: crate::client::market_data::extract_price(&c.low).ok()?,
                            close: crate::client::market_data::extract_price(&c.close).ok()?,
                            volume: c.volume as f64,
                            time,
                            ticker: instrument.ticker.clone(),
                        })
                    }).collect();

                    let bt_config = backtest::BacktestConfig {
                        initial_balance: 100000.0,
                        commission_pct: 0.001,
                        slippage_pct: 0.001,
                        max_positions: 1,
                        max_position_pct: 0.25,
                    };

                    // SMA crossover strategy function
                    let strategy_fn: optimizer::StrategyFn = Arc::new(|prices, _volumes, params| {
                        let fast = params.get("fast_period").copied().unwrap_or(10.0) as usize;
                        let slow = params.get("slow_period").copied().unwrap_or(30.0) as usize;
                        if prices.len() < slow { return 0.0; }

                        let fast_sma: f64 = prices[prices.len()-fast..].iter().sum::<f64>() / fast as f64;
                        let slow_sma: f64 = prices[prices.len()-slow..].iter().sum::<f64>() / slow as f64;

                        if fast_sma > slow_sma { 1.0 } else if fast_sma < slow_sma { -1.0 } else { 0.0 }
                    });

                    let optimizer = optimizer::Optimizer::new(opt_config, strategy_fn, bt_config)
                        .with_data(core_candles);

                    match optimizer.optimize().await {
                        Ok(report) => {
                            log::info!("Optimization complete in {}ms", report.total_time_ms);
                            log::info!("Best params: {:?}", report.best_params);
                            log::info!("Best score ({:?}): {:.4}", config.optimizer.as_ref().unwrap().metric, report.best_score);
                            log::info!("Total trials: {}", report.trials.len());
                        }
                        Err(e) => log::error!("Optimization failed: {}", e),
                    }
                }
            }
        }
    }

    // ── 11. Start Grid Bots ────────────────────────────────────────────
    for account in grid_accounts {
        if let Some(grid_config) = &account.strategy.parameters.grid_config {
            for instrument in account.instruments.iter().filter(|i| i.enabled) {
                log::info!(
                    "Starting Grid bot for {} ({})", instrument.ticker, instrument.figi
                );

                let bot_config = GridBotConfig {
                    account_id: account.account_id.clone(),
                    figi: instrument.figi.clone(),
                    ticker: instrument.ticker.clone(),
                    grid_config: grid_config.clone(),
                    check_interval_secs: account.strategy.parameters.check_interval as u64,
                };

                let mut grid_bot = GridBot::new(sdk.clone(), bot_config);
                tokio::spawn(async move {
                    if let Err(e) = grid_bot.run().await {
                        log::error!("Grid bot error: {}", e);
                    }
                });
            }
        }
    }

    // ── 12. Main Trading Loop ─────────────────────────────────────────
    let enabled_instruments = config.get_enabled_instruments();
    log::info!("Active instruments for analysis: {}", enabled_instruments.len());

    for instrument in enabled_instruments {
        if !instrument.enabled { continue; }
        log::info!("Analyzing: {} ({})", instrument.ticker, instrument.name);

        let mut instruments_service_client = sdk.instruments();
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
            .ok_or_else(|| anyhow!("Instrument not found: {}", instrument.ticker))?;

        let days_for_analysis = config.accounts.first()
            .map(|a| a.strategy.parameters.days_back_to_consider)
            .unwrap_or(30);

        let candles = match market_data_service.get_5min_candles(&found_instrument.figi, days_for_analysis).await {
            Ok(c) => { log::info!("Loaded {} candles", c.len()); c }
            Err(e) => { log::warn!("Candle load error: {}", e); vec![] }
        };

        let current_price = match market_data_service.get_last_price(&found_instrument.figi).await {
            Ok(p) => { log::info!("Current price: {:.2}", p); p }
            Err(e) => { log::warn!("Price fetch error: {}", e); 0.0 }
        };

        // Technical Analysis
        let tech_analysis = if !candles.is_empty() {
            match technical_analyzer.analyze(&instrument.ticker, &candles) {
                Ok(a) => { log::info!("Technical: trend={:?}, recommendation={:?}", a.trend, a.recommendation); Some(a) }
                Err(e) => { log::warn!("TA error: {}", e); None }
            }
        } else { None };

        // News Analysis
        let news_sentiment = if instrument.analysis_config.check_news {
            match news_analyzer.analyze(&instrument.ticker, &instrument.name).await {
                Ok(base) => {
                    if base.articles_count > 0 {
                        let news_items: Vec<NewsItem> = base.articles.iter().take(5).map(|a| NewsItem {
                            title: a.title.clone(),
                            content: a.content.clone(),
                            source: a.source.clone(),
                            url: a.url.clone(),
                        }).collect();

                        match news_llm_service.analyze_news_batch(&instrument.ticker, &instrument.name, &news_items).await {
                            Ok(llm) => Some(NewsSentiment {
                                ticker: instrument.ticker.clone(),
                                overall_sentiment: llm.overall_sentiment,
                                sentiment_score: llm.sentiment_score,
                                articles_count: base.articles_count,
                                articles: base.articles,
                                key_events: llm.key_events,
                            }),
                            Err(e) => { log::warn!("LLM news error: {}", e); Some(base) }
                        }
                    } else { Some(base) }
                }
                Err(e) => { log::warn!("News fetch error: {}", e); None }
            }
        } else { None };

        // Fundamental Analysis
        let fundamental_analysis = if instrument.analysis_config.fundamental_analysis {
            match fundamental_data_service.get_fundamental_data(&instrument.ticker, &instrument.name).await {
                Ok(Some(a)) => { log::info!("Fundamental: rating={:?}, score={:.1}", a.rating, a.overall_score); Some(a) }
                Ok(None) => { log::info!("No fundamental data for {}", instrument.ticker); None }
                Err(e) => { log::warn!("Fundamental error: {}", e); None }
            }
        } else { None };

        // Market Regime
        if current_price > 0.0 { regime_detector.add_price(current_price); }
        let market_regime = regime_detector.detect();
        log::info!("Market regime: {:?}", market_regime);

        // Agent Decision
        let use_price = if current_price > 0.0 { current_price }
            else if let Some(tech) = &tech_analysis { tech.current_price }
            else { log::warn!("No price data for {}", instrument.ticker); continue; };

        let current_position = match portfolio_service.get_position(&found_instrument.figi).await {
            Ok(Some(pos)) => { log::info!("Position: {} lots, avg: {:.2}", pos.quantity, pos.average_price); Some(pos) }
            Ok(None) => None,
            Err(e) => { log::warn!("Position error: {}", e); None }
        };

        let trading_agent = TradingAgent::new(ollama_provider.clone(), model_name.clone());
        let position_manager = PositionManager::new(sdk.clone(), "main".to_string());
        let trading_executor = TradingExecutor::new(position_manager, available_balance);

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
            market_regime,
            candles: vec![],
        };

        let decision = if config.llm_config.is_some() {
            match trading_agent.make_decision(context.clone()).await {
                Ok(d) => d,
                Err(e) => { log::warn!("LLM error, using rule-based: {}", e);
                    trading_agent.make_rule_based_decision(context)? }
            }
        } else {
            trading_agent.make_rule_based_decision(context)?
        };

        log::info!("Agent decision: {:?} (conf: {:.2}, pos: {:.1}%)",
            decision.action, decision.confidence, decision.position_size_pct * 100.0);
        log::info!("Rationale: {}", decision.rationale);

        if decision.action != agent::Action::Hold && decision.confidence >= 0.6 {
            match trading_executor.execute_decision(&decision, &found_instrument.figi).await {
                Ok(results) => {
                    for r in results {
                        log::info!("Order placed: ID={}, status={:?}", r.order_id, r.status);
                    }
                }
                Err(e) => log::error!("Execution error: {}", e),
            }
        } else {
            log::info!("Decision not executed: HOLD or low confidence");
        }

        tokio::time::sleep(tokio::time::Duration::from_secs(
            config.accounts.first()
                .map(|a| a.strategy.parameters.check_interval as u64)
                .unwrap_or(60)
        )).await;
    }

    log::info!("Analysis cycle completed. Waiting for background tasks...");
    tokio::signal::ctrl_c().await.ok();
    log::info!("Shutting down.");
    Ok(())
}
