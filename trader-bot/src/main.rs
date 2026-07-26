#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(async_fn_in_trait)]
#![allow(clippy::module_inception)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::upper_case_acronyms)]

mod agent;
mod analysis;
mod api;
mod backtest;
mod broker;
mod client;
mod config;
mod core;
mod datasource;
mod error;
mod execution;
mod instrument;
mod logging;
mod mcp;
mod ml_inference;
mod optimizer;
mod provider;
mod scanner;
mod scheduler;
mod storage;
mod strategy;
mod stream;
mod telemetry;
mod utils;

use anyhow::{Result, anyhow};
use log::Level;
use std::env;
use std::sync::Arc;
use t_invest_sdk::TInvestSdk;
use tokio::sync::Mutex;

use agent::{DecisionContext, OllamaQuery, TradingAgent};
use analysis::{
    FinBertSentimentService, FundamentalDataService, NewsAnalyzer, NewsItem, NewsLlmService,
    NewsSentiment, NewsSentimentAnalyzer, RegimeDetector, TechnicalAnalyzer,
};
use broker::{FinamBroker, MockBroker, TinkoffBroker};
use client::MarketDataService;
use config::{AccountConfig, Credential, SandboxConfig, StrategyType, TradingConfig, WorkingMode};
use core::*;
use datasource::{DataSourceRegistry, TinkoffDataSource};
use execution::PositionTracker;

use crate::mcp::ollama::OllamaProvider;
use strategy::{
    AiStrategy, GridBot, GridBotConfig, GridStrategy, IntervalStrategy, StrategyRegistry,
};

// ─── Broker Initialization ───────────────────────────────────────────

struct AccountBroker {
    broker: Arc<dyn Broker>,
    sdk: Option<TInvestSdk>,
}

async fn init_broker(
    account: &AccountConfig,
    cred: &Credential,
    mode: &WorkingMode,
    sandbox_cfg: Option<&SandboxConfig>,
) -> Result<AccountBroker> {
    let aid = account.account_id.clone().unwrap_or_default();
    match account.broker.as_str() {
        "tinkoff" => {
            let token = if cred.token.is_empty() {
                env::var("API_TOKEN")
                    .map_err(|_| anyhow!("API_TOKEN not set and no token in config"))?
            } else {
                cred.token.clone()
            };
            let is_sandbox = matches!(mode, WorkingMode::Sandbox);
            let (open_account, pay_in_amount) = sandbox_cfg
                .map(|s| (s.open_account, s.pay_in_amount))
                .unwrap_or((false, 0.0));
            let tb = TinkoffBroker::new(
                &token,
                account.account_id.clone(),
                is_sandbox,
                open_account,
                pay_in_amount,
            )
            .await?;
            let sdk = tb.sdk();
            let broker = Arc::new(tb) as Arc<dyn Broker>;
            Ok(AccountBroker {
                broker,
                sdk: Some(sdk),
            })
        }
        "finam" => {
            let finam_cred = cred
                .additional_keys
                .as_ref()
                .and_then(|keys| keys.iter().find(|k| k.broker == "finam"))
                .ok_or_else(|| anyhow!("Finam creds not found for account {}", aid))?;
            let broker = Arc::new(FinamBroker::new(&finam_cred.api_key, aid.clone()).await?)
                as Arc<dyn Broker>;
            Ok(AccountBroker { broker, sdk: None })
        }
        "mock" => {
            let broker = Arc::new(MockBroker::new(aid.clone(), 1_000_000.0)) as Arc<dyn Broker>;
            Ok(AccountBroker { broker, sdk: None })
        }
        other => Err(anyhow!(
            "Unsupported broker '{}' for account {}",
            other,
            aid
        )),
    }
}

// ─── Strategy Creation ───────────────────────────────────────────────

fn create_strategy(account: &AccountConfig, agent: Arc<TradingAgent>) -> Box<dyn Strategy> {
    let aid = account.account_id.clone().unwrap_or_default();
    match &account.strategy.strategy {
        StrategyType::Grid => {
            let cfg =
                account
                    .strategy
                    .parameters
                    .grid_config
                    .clone()
                    .unwrap_or(config::GridConfig {
                        lower_price: 100.0,
                        upper_price: 200.0,
                        grid_levels: 11,
                        order_size: 10,
                        grid_ratio: 0.5,
                    });
            Box::new(GridStrategy::new(cfg))
        }
        StrategyType::Ai => {
            let ai_cfg = account
                .strategy
                .parameters
                .ai_config
                .clone()
                .unwrap_or_default();
            Box::new(AiStrategy::new(aid, ai_cfg, agent))
        }
        _ => Box::new(IntervalStrategy),
    }
}

// ─── Main ────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() -> Result<()> {
    // ── 0. Logger ─────────────────────────────────────────────────────
    let log_webhook = env::var("LOG_WEBHOOK").ok();
    let log_file = env::var("LOG_FILE").ok();
    let log_level = env::var("LOG_LEVEL").unwrap_or_else(|_| "info".to_string());
    {
        let mut builder = logging::LoggerBuilder::new().console();
        if let Some(path) = &log_file {
            builder = builder.file(path)?;
        }
        if let Some(url) = &log_webhook {
            let threshold = match log_level.as_str() {
                "error" => Level::Error,
                "warn" => Level::Warn,
                "debug" => Level::Debug,
                _ => Level::Info,
            };
            builder = builder.network(url, threshold);
        }
        builder.init()?;
    }

    log::info!("╔══════════════════════════════════════════╗");
    log::info!(
        "║     Market Bot v{}                  ║",
        env!("CARGO_PKG_VERSION")
    );
    log::info!("╚══════════════════════════════════════════╝");

    // ── 1. Config ─────────────────────────────────────────────────────
    let config =
        TradingConfig::load_default().map_err(|e| anyhow!("Failed to load config: {}", e))?;
    log::info!(
        "Loaded config, mode: {:?}, accounts: {}",
        config.mode,
        config.accounts.len()
    );

    // ── 2. LLM ────────────────────────────────────────────────────────
    let llm_config = config.llm_config.as_ref();
    let ollama = if let Some(c) = llm_config {
        OllamaProvider::new(c.model.clone(), c.host.clone(), c.port)
    } else {
        OllamaProvider::default()
    };
    let model_name = llm_config
        .map(|c| c.model.clone())
        .unwrap_or_else(|| "fin-expert".to_string());
    log::info!("LLM: {}", model_name);

    // ── 3. Analysis services ──────────────────────────────────────────
    let news_analyzer = NewsAnalyzer::new(vec!["tinkoff".to_string(), "investing".to_string()]);
    let technical_analyzer = TechnicalAnalyzer::new();
    let fundamental_data = FundamentalDataService::new();
    let mut regime_detector = RegimeDetector::new(14, 14);

    let use_finbert = config
        .accounts
        .first()
        .and_then(|a| a.strategy.parameters.ai_config.as_ref())
        .map(|c| c.use_finbert)
        .unwrap_or(false);
    let news_analyzer_service: Arc<dyn NewsSentimentAnalyzer> = if use_finbert {
        log::info!("News sentiment: FinBERT");
        match FinBertSentimentService::new("models/finbert") {
            Ok(fb) => Arc::new(fb),
            Err(e) => {
                log::warn!("Failed to init FinBERT ({}), falling back to Ollama", e);
                Arc::new(NewsLlmService::new(ollama.clone()))
            }
        }
    } else {
        log::info!("News sentiment: Ollama LLM");
        Arc::new(NewsLlmService::new(ollama.clone()))
    };

    // ── 4. Init per-account brokers ───────────────────────────────────
    let mut account_brokers: Vec<(&AccountConfig, AccountBroker)> = Vec::new();
    for account in &config.accounts {
        let aid = account.account_id.as_deref().unwrap_or("");
        match init_broker(
            account,
            &config.credential,
            &config.mode,
            config.sandbox.as_ref(),
        )
        .await
        {
            Ok(ab) => {
                log::info!(
                    "Broker for {}: {} (sdk: {})",
                    aid,
                    account.broker,
                    ab.sdk.is_some()
                );
                account_brokers.push((account, ab));
            }
            Err(e) => log::error!("Failed to init broker for {}: {}", aid, e),
        }
    }

    // ── 5. Data sources ───────────────────────────────────────────────
    let mut data_sources = DataSourceRegistry::new();
    for (_, ab) in &account_brokers {
        if let Some(ref sdk) = ab.sdk {
            data_sources.register(Arc::new(TinkoffDataSource::new(sdk.clone())));
        }
    }
    log::info!("Data sources: {:?}", data_sources.list_names());

    // ── 6. Strategies ─────────────────────────────────────────────────
    let mut strategy_registry = StrategyRegistry::new();
    for (account, _) in &account_brokers {
        let memory_path = account
            .strategy
            .parameters
            .ai_config
            .as_ref()
            .and_then(|a| a.memory_path.clone())
            .map(std::path::PathBuf::from);
        let agent = Arc::new(
            TradingAgent::new(
                Box::new(OllamaQuery::new(ollama.clone())),
                model_name.clone(),
                memory_path,
            )
            .unwrap(),
        );
        let strategy = create_strategy(account, agent);
        let aid = account.account_id.as_deref().unwrap_or("");
        log::info!("Strategy for {}: {}", aid, strategy.name());
        strategy_registry.register(strategy);
    }
    log::info!("Strategies: {:?}", strategy_registry.list_names());

    // ── 7. Dashboard ──────────────────────────────────────────────────
    if let Some(dash) = &config.dashboard
        && dash.enabled
    {
        let brokers: Vec<Arc<dyn Broker>> = account_brokers
            .iter()
            .map(|(_, ab)| ab.broker.clone())
            .collect();
        let state = Arc::new(Mutex::new(api::AppState {
            brokers,
            data_sources,
            strategies: strategy_registry,
        }));
        let port = dash.port;
        tokio::spawn(async move {
            if let Err(e) = api::start_dashboard(state, port).await {
                log::error!("Dashboard error: {}", e);
            }
        });
        log::info!("Dashboard on port {}", port);
    }

    // ── 8. Per-account execution ──────────────────────────────────────
    for (account, account_broker) in &account_brokers {
        match account.strategy.strategy {
            StrategyType::Grid => run_grid_account(account, account_broker).await,
            StrategyType::Ai => {
                run_ai_account(
                    account,
                    account_broker,
                    &technical_analyzer,
                    &news_analyzer,
                    news_analyzer_service.as_ref(),
                    &fundamental_data,
                    &mut regime_detector,
                    &ollama,
                    &model_name,
                )
                .await
            }
            _ => {
                run_standard_account(
                    account,
                    account_broker,
                    &technical_analyzer,
                    &news_analyzer,
                    news_analyzer_service.as_ref(),
                    &fundamental_data,
                    &mut regime_detector,
                    &ollama,
                    &model_name,
                )
                .await
            }
        }?;
    }

    log::info!("All accounts processed. Waiting for background tasks...");
    tokio::signal::ctrl_c().await.ok();
    log::info!("Shutdown.");
    Ok(())
}

// ─── Grid Account ────────────────────────────────────────────────────

async fn run_grid_account(account: &AccountConfig, ab: &AccountBroker) -> Result<()> {
    let aid = account.account_id.as_deref().unwrap_or("");
    let sdk = match &ab.sdk {
        Some(sdk) => sdk.clone(),
        None => {
            log::warn!(
                "Grid account {} has no Tinkoff SDK — GridBot requires it",
                aid
            );
            return Ok(());
        }
    };
    let grid_cfg = match &account.strategy.parameters.grid_config {
        Some(c) => c.clone(),
        None => return Ok(()),
    };
    let account_id = aid.to_string();
    let check_interval = account.strategy.parameters.check_interval;

    for instrument in account.instruments.iter().filter(|i| i.enabled) {
        let sdk = sdk.clone();
        let bot_config = GridBotConfig {
            account_id: account_id.clone(),
            figi: instrument.figi.clone(),
            ticker: instrument.ticker.clone(),
            grid_config: grid_cfg.clone(),
            check_interval_secs: check_interval as u64,
        };
        let ticker2 = instrument.ticker.clone();
        let figi2 = instrument.figi.clone();
        let ticker_name = instrument.ticker.clone();
        tokio::spawn(async move {
            let mut bot = GridBot::new(sdk, bot_config);
            if let Err(e) = bot.run().await {
                log::error!("GridBot {} error: {}", ticker2, e);
            }
        });
        log::info!("GridBot started for {} ({})", ticker_name, figi2);
    }
    Ok(())
}

// ─── AI / Standard Account Pipeline ──────────────────────────────────

#[allow(clippy::too_many_arguments)]
async fn run_ai_account(
    account: &AccountConfig,
    ab: &AccountBroker,
    technical_analyzer: &TechnicalAnalyzer,
    news_analyzer: &NewsAnalyzer,
    news_sentiment: &dyn NewsSentimentAnalyzer,
    fundamental_data: &FundamentalDataService,
    regime_detector: &mut RegimeDetector,
    ollama: &OllamaProvider,
    model_name: &str,
) -> Result<()> {
    let aid = account.account_id.as_deref().unwrap_or("");
    let balance = ab.broker.balance().await.unwrap_or(0.0);
    log::info!("[{}] Balance: {:.2}", aid, balance);

    let memory_path = account
        .strategy
        .parameters
        .ai_config
        .as_ref()
        .and_then(|a| a.memory_path.clone())
        .map(std::path::PathBuf::from);
    let agent = Arc::new(
        TradingAgent::new(
            Box::new(OllamaQuery::new(ollama.clone())),
            model_name.to_string(),
            memory_path,
        )
        .unwrap(),
    );
    let mut tracker = PositionTracker::new(Some(agent.memory.clone()));

    for instrument in account.instruments.iter().filter(|i| i.enabled) {
        log::info!("[{}] Analyzing {}", aid, instrument.ticker);

        // 1. Wick-aware exit check before making new decisions
        let candles = get_candles_for_analysis(account, ab, instrument).await;
        for candle in &candles {
            let high = client::market_data::extract_price(&candle.high)?;
            let low = client::market_data::extract_price(&candle.low)?;
            let close = client::market_data::extract_price(&candle.close)?;
            let closed = tracker.check_candle(&instrument.ticker, high, low, close);
            for pos in &closed {
                log::info!(
                    "[{}] Exit via {:?} at {:.2}",
                    instrument.ticker,
                    pos.reason,
                    pos.exit_price
                );
            }
        }

        let current_price = ab
            .broker
            .last_price(&instrument.ticker)
            .await
            .unwrap_or(0.0);

        // Skip analysis if we already hold a position (let it run to SL/TP)
        if tracker.has_position(&instrument.ticker) {
            log::info!("[{}] Position open, skipping analysis", instrument.ticker);
            continue;
        }

        let tech = if !candles.is_empty() {
            technical_analyzer
                .analyze(&instrument.ticker, &candles)
                .ok()
        } else {
            None
        };

        let news = get_news_for_instrument(instrument, news_analyzer, news_sentiment).await;
        let fund = if instrument.analysis_config.fundamental_analysis {
            fundamental_data
                .get_fundamental_data(&instrument.ticker, &instrument.name)
                .await
                .ok()
                .flatten()
        } else {
            None
        };

        if current_price > 0.0 {
            regime_detector.add_price(current_price);
        }
        let regime = regime_detector.detect();
        log::info!(
            "[{}] Price: {:.2}, Regime: {:?}",
            instrument.ticker,
            current_price,
            regime
        );

        let position = ab.broker.position(&instrument.ticker).await.ok().flatten();
        let pos_for_ctx = position.map(|p| agent::CurrentPosition {
            quantity: p.quantity,
            average_price: p.average_price,
            current_value: p.current_price * p.quantity as f64,
        });

        let ctx = DecisionContext {
            ticker: instrument.ticker.clone(),
            company_name: instrument.name.clone(),
            current_price,
            news_sentiment: news,
            technical_analysis: tech,
            fundamental_analysis: fund,
            available_balance: balance,
            current_position: pos_for_ctx,
            risk_config: account.risk_management.clone(),
            max_position_pct: instrument.max_position_pct,
            market_regime: regime,
            candles: vec![],
        };

        let decision = if llm_config_enabled(account) {
            agent.make_decision(ctx.clone()).await.unwrap_or_else(|e| {
                log::warn!("LLM error, fallback to rule: {}", e);
                agent
                    .make_rule_based_decision(ctx.clone())
                    .unwrap_or_else(|_| agent::TradingDecision {
                        ticker: instrument.ticker.clone(),
                        action: agent::Action::Hold,
                        confidence: 0.0,
                        entry_price: None,
                        position_size_pct: 0.0,
                        stop_loss: None,
                        take_profit: None,
                        rationale: "fallback".into(),
                        risks: vec![],
                        time_horizon: agent::TimeHorizon::Medium,
                        current_position: None,
                        current_price,
                    })
            })
        } else {
            agent.make_rule_based_decision(ctx)?
        };

        log::info!(
            "[{}] {:?} (conf: {:.2}, size: {:.1}%)",
            instrument.ticker,
            decision.action,
            decision.confidence,
            decision.position_size_pct * 100.0
        );

        if decision.action != agent::Action::Hold && decision.confidence >= 0.6 {
            execute_via_broker(&ab.broker, &decision).await?;
            tracker.open(
                &instrument.ticker,
                decision.current_price,
                decision.position_size_pct * 100.0,
                decision.action.clone(),
                decision.stop_loss,
                decision.take_profit,
            );
        }
    }
    Ok(())
}

/// Same pipeline as AI but without LLM (always rule-based) — used for non-AI strategies
async fn run_standard_account(
    account: &AccountConfig,
    ab: &AccountBroker,
    technical_analyzer: &TechnicalAnalyzer,
    news_analyzer: &NewsAnalyzer,
    news_sentiment: &dyn NewsSentimentAnalyzer,
    fundamental_data: &FundamentalDataService,
    regime_detector: &mut RegimeDetector,
    ollama: &OllamaProvider,
    model_name: &str,
) -> Result<()> {
    run_ai_account(
        account,
        ab,
        technical_analyzer,
        news_analyzer,
        news_sentiment,
        fundamental_data,
        regime_detector,
        ollama,
        model_name,
    )
    .await
}

// ─── Helpers ─────────────────────────────────────────────────────────

async fn get_candles_for_analysis(
    account: &AccountConfig,
    ab: &AccountBroker,
    instrument: &config::InstrumentConfig,
) -> Vec<t_invest_sdk::api::HistoricCandle> {
    if let Some(ref sdk) = ab.sdk {
        let days = account.strategy.parameters.days_back_to_consider;
        match MarketDataService::new(sdk.clone())
            .get_5min_candles(&instrument.figi, days)
            .await
        {
            Ok(c) => {
                log::info!("Loaded {} candles for {}", c.len(), instrument.ticker);
                c
            }
            Err(e) => {
                log::warn!("Candle error: {}", e);
                vec![]
            }
        }
    } else {
        log::warn!(
            "No SDK for candle data — skipping TA for {}",
            instrument.ticker
        );
        vec![]
    }
}

async fn get_news_for_instrument(
    instrument: &config::InstrumentConfig,
    analyzer: &NewsAnalyzer,
    news_sentiment: &dyn NewsSentimentAnalyzer,
) -> Option<NewsSentiment> {
    if !instrument.analysis_config.check_news {
        return None;
    }
    let base = analyzer
        .analyze(&instrument.ticker, &instrument.name)
        .await
        .ok()?;
    if base.articles_count == 0 {
        return Some(base);
    }
    let items: Vec<NewsItem> = base
        .articles
        .iter()
        .take(5)
        .map(|a| NewsItem {
            title: a.title.clone(),
            content: a.content.clone(),
            source: a.source.clone(),
            url: a.url.clone(),
        })
        .collect();

    match news_sentiment
        .analyze_news_batch(&instrument.ticker, &instrument.name, &items)
        .await
    {
        Ok(enh) => Some(NewsSentiment {
            ticker: instrument.ticker.clone(),
            overall_sentiment: enh.overall_sentiment,
            sentiment_score: enh.sentiment_score,
            articles_count: base.articles_count,
            articles: base.articles,
            key_events: enh.key_events,
        }),
        Err(e) => {
            log::warn!("News sentiment error: {}", e);
            Some(base)
        }
    }
}

async fn execute_via_broker(
    broker: &Arc<dyn Broker>,
    decision: &agent::TradingDecision,
) -> Result<()> {
    let action = match decision.action {
        agent::Action::Buy => OrderAction::Buy,
        agent::Action::Sell => OrderAction::Sell,
        agent::Action::Hold => return Ok(()),
    };
    let price = decision.entry_price.unwrap_or(decision.current_price);
    let qty = (decision.position_size_pct * 100.0) as i32;
    if qty <= 0 {
        return Ok(());
    }
    let request = OrderRequest {
        instrument: decision.ticker.clone(),
        action,
        order_type: OrderType::Limit,
        quantity: qty,
        price: Some(price),
        account_id: broker.account_id().to_string(),
        client_order_id: None,
    };
    match broker.place_order(request).await {
        Ok(resp) => log::info!("Order placed: {:?}", resp),
        Err(e) => log::error!("Order error: {}", e),
    }
    Ok(())
}

fn llm_config_enabled(account: &AccountConfig) -> bool {
    account
        .strategy
        .parameters
        .ai_config
        .as_ref()
        .map(|c| c.use_llm)
        .unwrap_or(false)
        || account.strategy.strategy == StrategyType::Ai
}
