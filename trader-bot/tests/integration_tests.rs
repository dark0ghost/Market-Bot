//! Integration tests for the trading bot
//!
//! These tests verify interaction between system components

use trader_bot::error::BotError;

/// Integration test for Sentiment analysis
#[test]
fn test_sentiment_analysis_integration() {
    use trader_bot::analysis::{NewsArticle, Sentiment};

    let articles = [
        NewsArticle {
            title: "Positive news".to_string(),
            content: "Company showed profit growth".to_string(),
            source: "tinkoff".to_string(),
            url: "https://example.com/1".to_string(),
            published_at: None,
            sentiment: Some(Sentiment::Positive),
        },
        NewsArticle {
            title: "Negative news".to_string(),
            content: "Sanctions against company".to_string(),
            source: "bloomberg".to_string(),
            url: "https://example.com/2".to_string(),
            published_at: None,
            sentiment: Some(Sentiment::Negative),
        },
        NewsArticle {
            title: "Neutral news".to_string(),
            content: "Board meeting scheduled".to_string(),
            source: "interfax".to_string(),
            url: "https://example.com/3".to_string(),
            published_at: None,
            sentiment: Some(Sentiment::Neutral),
        },
    ];

    // Verify sentiment to score conversion
    let scores: Vec<f64> = articles
        .iter()
        .map(|a| a.sentiment.as_ref().map_or(0.0, Sentiment::to_score))
        .collect();

    assert_eq!(scores, vec![1.0, -1.0, 0.0]);

    let avg_score: f64 = scores.iter().sum::<f64>() / scores.len() as f64;
    assert_eq!(avg_score, 0.0);

    let overall = Sentiment::from_score(avg_score);
    assert_eq!(overall, Sentiment::Neutral);
}

/// Integration test for Grid strategy
#[test]
fn test_grid_strategy_integration() {
    use trader_bot::config::GridConfig;
    use trader_bot::strategy::{GridStrategy, OrderSide};

    let config = GridConfig {
        lower_price: 100.0,
        upper_price: 200.0,
        grid_levels: 11,
        order_size: 10,
        grid_ratio: 0.5,
    };

    let strategy = GridStrategy::new(config);
    let levels = strategy.calculate_grid_levels();

    let buy_levels: Vec<_> = levels
        .iter()
        .filter(|l| l.order_type == OrderSide::Buy)
        .collect();
    let sell_levels: Vec<_> = levels
        .iter()
        .filter(|l| l.order_type == OrderSide::Sell)
        .collect();

    assert!(!buy_levels.is_empty());
    assert!(!sell_levels.is_empty());

    let max_buy = buy_levels.iter().map(|l| l.price).fold(f64::MIN, f64::max);
    let min_sell = sell_levels.iter().map(|l| l.price).fold(f64::MAX, f64::min);

    assert!(max_buy <= min_sell);
}

/// Integration test for position calculation
#[test]
fn test_position_calculation_integration() {
    let balance = 100000.0;
    let position_pct = 0.1; // 10%
    let price = 150.0;

    let position_value = balance * position_pct;
    let quantity = (position_value / price) as i32;

    assert_eq!(quantity, 66); // 10000 / 150 = 66.67 -> 66

    let total_cost = quantity as f64 * price;
    assert!(total_cost <= position_value);
    assert!(total_cost + price > position_value);
}

/// Integration test for error types
#[test]
fn test_error_types_integration() {
    let position_err = BotError::Position("test".to_string());
    assert!(position_err.to_string().contains("Position error"));

    let strategy_err = BotError::Strategy("config error".to_string());
    assert!(strategy_err.to_string().contains("Strategy error"));

    let insufficient_funds = BotError::InsufficientFunds {
        required: 1000.0,
        available: 500.0,
    };
    let err_msg = insufficient_funds.to_string();
    assert!(err_msg.contains("1000"));
    assert!(err_msg.contains("500"));
}

/// Integration test for TechnicalAnalyzer
#[test]
fn test_technical_analyzer_integration() {
    use trader_bot::TechnicalAnalyzer;

    let analyzer = TechnicalAnalyzer::new();

    let _ = analyzer;
}

/// Integration test for FundamentalAnalyzer
#[test]
fn test_fundamental_analyzer_integration() {
    use trader_bot::{
        CompanyRating, DividendMetrics, FinancialHealthMetrics, FundamentalAnalyzer, GrowthMetrics,
        ProfitabilityMetrics, ValuationMetrics,
    };

    let analyzer = FundamentalAnalyzer::default();

    let valuation = ValuationMetrics {
        pe_ratio: Some(10.0),
        forward_pe: Some(8.0),
        peg_ratio: Some(1.0),
        price_to_book: Some(1.5),
        price_to_sales: Some(2.0),
        ev_to_ebitda: None,
    };

    let profitability = ProfitabilityMetrics {
        gross_margin: Some(0.30),
        operating_margin: Some(0.25),
        net_margin: Some(0.20),
        roe: Some(0.20),
        roa: Some(0.05),
        roic: None,
    };

    let financial_health = FinancialHealthMetrics {
        current_ratio: Some(1.5),
        quick_ratio: Some(1.2),
        debt_to_equity: Some(0.5),
        debt_to_assets: None,
        interest_coverage: None,
        free_cash_flow: None,
    };

    let growth = GrowthMetrics {
        revenue_growth_yoy: Some(0.15),
        earnings_growth_yoy: Some(0.10),
        revenue_growth_3y: None,
        earnings_growth_3y: None,
        revenue_growth_5y: None,
        earnings_growth_5y: None,
    };

    let dividends = Some(DividendMetrics {
        dividend_yield: Some(0.05),
        payout_ratio: Some(0.3),
        dividend_growth_3y: None,
        consecutive_years: None,
    });

    let result = analyzer.analyze(
        "TINK",
        "Tinkoff",
        valuation,
        profitability,
        financial_health,
        growth,
        dividends,
    );

    assert!(result.overall_score >= 0.0);
    assert!(matches!(
        result.rating,
        CompanyRating::Excellent
            | CompanyRating::Good
            | CompanyRating::Fair
            | CompanyRating::Poor
            | CompanyRating::VeryPoor
    ));
}

/// Integration test for GridState and RebalanceResult
#[test]
fn test_grid_state_rebalance_integration() {
    use trader_bot::strategy::grid_executor::RebalanceResult;
    use trader_bot::strategy::{GridLevel, GridState, OrderSide};
    let mut state = GridState {
        ticker: "TINK".to_string(),
        figi: "BBG000B9XRY4".to_string(),
        levels: vec![
            GridLevel {
                price: 100.0,
                order_type: OrderSide::Buy,
                level_index: 0,
            },
            GridLevel {
                price: 150.0,
                order_type: OrderSide::Sell,
                level_index: 1,
            },
        ],
        active_orders: vec![0, 1],
        filled_orders: vec![],
        current_price: 125.0,
    };

    let filled_level = 0;
    state.active_orders.retain(|&i| i != filled_level);
    state.filled_orders.push(filled_level);

    assert_eq!(state.active_orders.len(), 1);
    assert_eq!(state.filled_orders.len(), 1);
    assert!(!state.active_orders.contains(&0));
    assert!(state.filled_orders.contains(&0));

    let rebalance_result = RebalanceResult {
        cancelled_orders: 1,
        placed_orders: 2,
    };

    assert_eq!(rebalance_result.cancelled_orders, 1);
    assert_eq!(rebalance_result.placed_orders, 2);
}

/// Integration test for VolumeAnalysis
#[test]
fn test_volume_analysis_integration() {
    use trader_bot::analysis::technical::VolumeAnalysis;

    let normal = VolumeAnalysis {
        current_volume: 500.0,
        avg_volume: 500.0,
        volume_ratio: 1.0,
        is_unusual: false,
    };

    assert!(!normal.is_unusual);
    assert_eq!(normal.volume_ratio, 1.0);

    let unusual = VolumeAnalysis {
        current_volume: 2000.0,
        avg_volume: 500.0,
        volume_ratio: 4.0,
        is_unusual: true,
    };

    assert!(unusual.is_unusual);
    assert_eq!(unusual.volume_ratio, 4.0);
}

/// Integration test for Recommendation
#[test]
fn test_recommendation_integration() {
    use trader_bot::analysis::Recommendation;
    let recommendations = vec![
        Recommendation::StrongBuy,
        Recommendation::Buy,
        Recommendation::Hold,
        Recommendation::Sell,
        Recommendation::StrongSell,
    ];

    for rec in &recommendations {
        let debug_str = format!("{:?}", rec);
        assert!(!debug_str.is_empty());
    }
}

/// Integration test for NewsSentiment
#[test]
fn test_news_sentiment_integration() {
    use trader_bot::analysis::{NewsArticle, NewsSentiment, Sentiment};

    let sentiment = NewsSentiment {
        ticker: "TINK".to_string(),
        overall_sentiment: Sentiment::Positive,
        sentiment_score: 0.5,
        articles_count: 3,
        articles: vec![NewsArticle {
            title: "News 1".to_string(),
            content: "Content 1".to_string(),
            source: "tinkoff".to_string(),
            url: "https://example.com/1".to_string(),
            published_at: None,
            sentiment: Some(Sentiment::Positive),
        }],
        key_events: vec!["Event 1".to_string()],
    };

    assert_eq!(sentiment.ticker, "TINK");
    assert_eq!(sentiment.overall_sentiment, Sentiment::Positive);
    assert_eq!(sentiment.sentiment_score, 0.5);
    assert_eq!(sentiment.articles_count, 3);
    assert_eq!(sentiment.key_events.len(), 1);
}
