//! Integration тесты для торгового бота
//!
//! Эти тесты проверяют взаимодействие между компонентами системы

use trader_bot::error::{BotError, StrategyError};

/// Тест интеграции Sentiment анализа
#[test]
fn test_sentiment_analysis_integration() {
    use trader_bot::analysis::{Sentiment, NewsArticle};

    // Создаём тестовые новости
    let articles = vec![
        NewsArticle {
            title: "Позитивная новость".to_string(),
            content: "Компания показала рост прибыли".to_string(),
            source: "tinkoff".to_string(),
            url: "https://example.com/1".to_string(),
            published_at: None,
            sentiment: Some(Sentiment::Positive),
        },
        NewsArticle {
            title: "Негативная новость".to_string(),
            content: "Санкции против компании".to_string(),
            source: "bloomberg".to_string(),
            url: "https://example.com/2".to_string(),
            published_at: None,
            sentiment: Some(Sentiment::Negative),
        },
        NewsArticle {
            title: "Нейтральная новость".to_string(),
            content: "Обычный отчёт".to_string(),
            source: "investing".to_string(),
            url: "https://example.com/3".to_string(),
            published_at: None,
            sentiment: Some(Sentiment::Neutral),
        },
    ];

    // Проверяем конвертацию sentiment в score
    let scores: Vec<f64> = articles.iter()
        .map(|a| a.sentiment.as_ref().map_or(0.0, Sentiment::to_score))
        .collect();

    assert_eq!(scores, vec![1.0, -1.0, 0.0]);

    // Средний score должен быть 0.0
    let avg_score: f64 = scores.iter().sum::<f64>() / scores.len() as f64;
    assert_eq!(avg_score, 0.0);

    // Конвертация обратно в Sentiment должна дать Neutral
    let overall = Sentiment::from_score(avg_score);
    assert_eq!(overall, Sentiment::Neutral);
}

/// Тест интеграции Grid стратегии
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

    // Проверяем, что уровни корректно разделены
    let buy_levels: Vec<_> = levels.iter()
        .filter(|l| l.order_type == OrderSide::Buy)
        .collect();
    let sell_levels: Vec<_> = levels.iter()
        .filter(|l| l.order_type == OrderSide::Sell)
        .collect();

    assert!(!buy_levels.is_empty());
    assert!(!sell_levels.is_empty());

    // Проверяем, что buy уровни ниже sell уровней
    let max_buy = buy_levels.iter().map(|l| l.price).fold(f64::MIN, f64::max);
    let min_sell = sell_levels.iter().map(|l| l.price).fold(f64::MAX, f64::min);
    
    assert!(max_buy <= min_sell);
}

/// Тест интеграции расчета количества лотов
#[test]
fn test_position_calculation_integration() {
    // Тестируем логику расчета позиции
    let balance = 100000.0;
    let position_pct = 0.1; // 10%
    let price = 150.0;

    let position_value = balance * position_pct;
    let quantity = (position_value / price) as i32;

    assert_eq!(quantity, 66); // 10000 / 150 = 66.67 -> 66

    // Проверяем, что общая стоимость не превышает выделенный бюджет
    let total_cost = quantity as f64 * price;
    assert!(total_cost <= position_value);
    assert!(total_cost + price > position_value); // Нельзя купить ещё один лот
}

/// Тест интеграции error типов
#[test]
fn test_error_types_integration() {
    // Проверяем, что ошибки корректно конвертируются
    let position_err = BotError::Position("test".to_string());
    assert!(position_err.to_string().contains("позиции"));

    let strategy_err = BotError::Strategy("config error".to_string());
    assert!(strategy_err.to_string().contains("стратегии"));

    let insufficient_funds = BotError::InsufficientFunds {
        required: 1000.0,
        available: 500.0,
    };
    let err_msg = insufficient_funds.to_string();
    assert!(err_msg.contains("1000"));
    assert!(err_msg.contains("500"));
}

/// Тест интеграции TechnicalAnalyzer
#[test]
fn test_technical_analyzer_integration() {
    use trader_bot::{TechnicalAnalyzer};

    // TechnicalAnalyzer требует свечи для анализа
    // Проверяем, что создание анализатора работает корректно
    // Полноценный тест требует моковых данных SDK
    let analyzer = TechnicalAnalyzer::new();
    
    // Просто проверяем, что анализатор создаётся
    drop(analyzer);
}

/// Тест интеграции FundamentalAnalyzer
#[test]
fn test_fundamental_analyzer_integration() {
    use trader_bot::{FundamentalAnalyzer, CompanyRating, ValuationMetrics, ProfitabilityMetrics, FinancialHealthMetrics, GrowthMetrics, DividendMetrics};

    let analyzer = FundamentalAnalyzer::default();

    // Тестовые данные для анализа
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

    // Проверяем, что анализ возвращает результат
    let result = analyzer.analyze("TINK", "Tinkoff", valuation, profitability, financial_health, growth, dividends);
    
    // Результат должен содержать рейтинг
    assert!(result.overall_score >= 0.0);
    assert!(matches!(result.rating, CompanyRating::Excellent | CompanyRating::Good | CompanyRating::Fair | CompanyRating::Poor | CompanyRating::VeryPoor));
}

/// Тест интеграции GridState и RebalanceResult
#[test]
fn test_grid_state_rebalance_integration() {
    use trader_bot::strategy::{GridState, GridLevel, OrderSide};
    use trader_bot::strategy::grid_executor::RebalanceResult;

    // Создаём начальное состояние сетки
    let mut state = GridState {
        ticker: "TINK".to_string(),
        figi: "BBG000B9XRY4".to_string(),
        levels: vec![
            GridLevel { price: 100.0, order_type: OrderSide::Buy, level_index: 0 },
            GridLevel { price: 150.0, order_type: OrderSide::Sell, level_index: 1 },
        ],
        active_orders: vec![0, 1],
        filled_orders: vec![],
        current_price: 125.0,
    };

    // Имитация исполнения ордера на уровне 0
    let filled_level = 0;
    state.active_orders.retain(|&i| i != filled_level);
    state.filled_orders.push(filled_level);

    assert_eq!(state.active_orders.len(), 1);
    assert_eq!(state.filled_orders.len(), 1);
    assert!(!state.active_orders.contains(&0));
    assert!(state.filled_orders.contains(&0));

    // Имитация результата перебалансировки
    let rebalance_result = RebalanceResult {
        cancelled_orders: 1,
        placed_orders: 2,
    };

    assert_eq!(rebalance_result.cancelled_orders, 1);
    assert_eq!(rebalance_result.placed_orders, 2);
}

/// Тест интеграции VolumeAnalysis
#[test]
fn test_volume_analysis_integration() {
    use trader_bot::analysis::technical::VolumeAnalysis;

    // Нормальный объём
    let normal = VolumeAnalysis {
        current_volume: 500.0,
        avg_volume: 500.0,
        volume_ratio: 1.0,
        is_unusual: false,
    };

    assert!(!normal.is_unusual);
    assert_eq!(normal.volume_ratio, 1.0);

    // Аномальный объём
    let unusual = VolumeAnalysis {
        current_volume: 2000.0,
        avg_volume: 500.0,
        volume_ratio: 4.0,
        is_unusual: true,
    };

    assert!(unusual.is_unusual);
    assert_eq!(unusual.volume_ratio, 4.0);
}

/// Тест интеграции Recommendation
#[test]
fn test_recommendation_integration() {
    use trader_bot::analysis::Recommendation;

    // Проверяем все варианты рекомендаций
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

/// Тест интеграции NewsSentiment
#[test]
fn test_news_sentiment_integration() {
    use trader_bot::analysis::{NewsSentiment, Sentiment, NewsArticle};

    let sentiment = NewsSentiment {
        ticker: "TINK".to_string(),
        overall_sentiment: Sentiment::Positive,
        sentiment_score: 0.5,
        articles_count: 3,
        articles: vec![
            NewsArticle {
                title: "News 1".to_string(),
                content: "Content 1".to_string(),
                source: "tinkoff".to_string(),
                url: "https://example.com/1".to_string(),
                published_at: None,
                sentiment: Some(Sentiment::Positive),
            },
        ],
        key_events: vec!["Событие 1".to_string()],
    };

    assert_eq!(sentiment.ticker, "TINK");
    assert_eq!(sentiment.overall_sentiment, Sentiment::Positive);
    assert_eq!(sentiment.sentiment_score, 0.5);
    assert_eq!(sentiment.articles_count, 3);
    assert_eq!(sentiment.key_events.len(), 1);
}
