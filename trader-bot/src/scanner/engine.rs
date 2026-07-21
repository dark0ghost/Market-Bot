use tokio::sync::Semaphore;
use std::sync::Arc;
use t_invest_sdk::TInvestSdk;
use t_invest_sdk::api::CandleInterval;
use crate::client::{MarketDataService, PortfolioService};
use crate::agent::{TradingAgent, DecisionContext, TradingDecision};
use crate::analysis::{*, MarketRegime};
use mcp_client::ollama::OllamaProvider;

pub struct ScanResult {
    pub ticker: String,
    pub decision: Option<TradingDecision>,
    pub error: Option<String>,
}

pub struct SignalScanner {
    sdk: TInvestSdk,
    market_data: MarketDataService,
    portfolio: PortfolioService,
    agent: TradingAgent,
    technical_analyzer: TechnicalAnalyzer,
    news_analyzer: NewsAnalyzer,
    fundamental_data: FundamentalDataService,
    news_llm: Option<NewsLLMService>,
    concurrency_limit: Arc<Semaphore>,
}

impl SignalScanner {
    pub fn new(
        sdk: TInvestSdk,
        portfolio: PortfolioService,
        agent: TradingAgent,
        llm_provider: Option<OllamaProvider>,
        max_concurrent: usize,
    ) -> Self {
        SignalScanner {
            market_data: MarketDataService::new(sdk.clone()),
            portfolio,
            agent,
            technical_analyzer: TechnicalAnalyzer::new(),
            news_analyzer: NewsAnalyzer::new(vec!["tinkoff".into(), "investing".into()]),
            fundamental_data: FundamentalDataService::new(),
            news_llm: llm_provider.map(NewsLLMService::new),
            sdk,
            concurrency_limit: Arc::new(Semaphore::new(max_concurrent)),
        }
    }

    pub async fn scan_instrument(
        &self,
        ticker: &str,
        figi: &str,
        name: &str,
        available_balance: f64,
        max_position_pct: f64,
        days_back: u32,
    ) -> ScanResult {
        let _permit = self.concurrency_limit.acquire().await;

        let candles = match self.market_data.get_5min_candles(figi, days_back).await {
            Ok(c) => c,
            Err(e) => return ScanResult {
                ticker: ticker.to_string(),
                decision: None,
                error: Some(format!("candles: {}", e)),
            },
        };

        let current_price = candles.last()
            .and_then(|c| c.close.as_ref())
            .map(|q| q.units as f64 + q.nano as f64 / 1_000_000_000.0)
            .unwrap_or(0.0);

        let tech = self.technical_analyzer.analyze(ticker, &candles).ok();

        let news = if let Some(ref llm) = self.news_llm {
            match self.news_analyzer.analyze(ticker, name).await {
                Ok(sentiment) => {
                    let items: Vec<NewsItem> = sentiment.articles.iter().take(5).map(|a| NewsItem {
                        title: a.title.clone(),
                        content: a.content.clone(),
                        source: a.source.clone(),
                        url: a.url.clone(),
                    }).collect();

                    match llm.analyze_news_batch(ticker, name, &items).await {
                        Ok(llm_result) => Some(NewsSentiment {
                            ticker: ticker.to_string(),
                            overall_sentiment: llm_result.overall_sentiment,
                            sentiment_score: llm_result.sentiment_score,
                            articles_count: sentiment.articles_count,
                            articles: sentiment.articles,
                            key_events: llm_result.key_events,
                        }),
                        Err(_) => Some(sentiment),
                    }
                }
                Err(_) => None,
            }
        } else {
            None
        };

        let fundamental = self.fundamental_data
            .get_fundamental_data(ticker, name).await
            .ok()
            .flatten();

        let position = self.portfolio.get_position(figi).await.ok().flatten();

        let context = DecisionContext {
            ticker: ticker.to_string(),
            company_name: name.to_string(),
            current_price,
            news_sentiment: news,
            technical_analysis: tech,
            fundamental_analysis: fundamental,
            available_balance,
            current_position: position,
            risk_config: None,
            max_position_pct,
            market_regime: MarketRegime::Quiet,
            candles: vec![],
        };

        let decision = self.agent.make_rule_based_decision(context).ok();

        ScanResult {
            ticker: ticker.to_string(),
            decision,
            error: None,
        }
    }

    pub async fn scan_many(
        &self,
        instruments: Vec<(String, String, String)>,
        available_balance: f64,
        max_position_pct: f64,
        days_back: u32,
    ) -> Vec<ScanResult> {
        let mut handles = Vec::new();

        for (ticker, figi, name) in instruments {
            let self_ref: &SignalScanner = &*self;
            let ticker_ = ticker.clone();
            let figi_ = figi.clone();
            let name_ = name.clone();

            handles.push(async move {
                self_ref.scan_instrument(
                    &ticker_, &figi_, &name_,
                    available_balance, max_position_pct, days_back
                ).await
            });
        }

        futures::future::join_all(handles).await
    }
}
