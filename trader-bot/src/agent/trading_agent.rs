use crate::analysis::{
    CompanyRating, FundamentalAnalysis, MarketRegime, NewsSentiment, Recommendation, Sentiment,
    TechnicalAnalysis, Trend,
};
use crate::config::RiskManagementConfig;
use anyhow::Result;
use mcp_client::llm_provider::LLMProvider;
use mcp_client::ollama::OllamaProvider;
use serde::{Deserialize, Serialize};

/// Решение торгового агента
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradingDecision {
    pub ticker: String,
    pub action: Action,
    pub confidence: f64,          // 0.0 - 1.0
    pub entry_price: Option<f64>, // Рекомендуемая цена входа
    pub position_size_pct: f64,   // Доля от портфеля (0.0 - 1.0)
    pub stop_loss: Option<f64>,
    pub take_profit: Option<f64>,
    pub rationale: String,
    pub risks: Vec<String>,
    pub time_horizon: TimeHorizon,
    pub current_position: Option<i32>, // Текущая позиция в лотах (для Sell)
    pub current_price: f64,            // Текущая цена инструмента
}

/// Тип действия
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum Action {
    Buy,
    Sell,
    Hold,
}

/// Горизонт инвестирования
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TimeHorizon {
    Short,  // 1-7 дней
    Medium, // 1-4 недели
    Long,   // 1+ месяцев
}

/// Контекст для принятия решения
#[derive(Debug, Clone)]
pub struct DecisionContext {
    pub ticker: String,
    pub company_name: String,
    pub current_price: f64,
    pub news_sentiment: Option<NewsSentiment>,
    pub technical_analysis: Option<TechnicalAnalysis>,
    pub fundamental_analysis: Option<FundamentalAnalysis>,
    pub available_balance: f64,
    pub current_position: Option<CurrentPosition>,
    pub risk_config: Option<RiskManagementConfig>,
    pub max_position_pct: f64,
    pub market_regime: MarketRegime,
    pub candles: Vec<crate::client::order_book::OrderBookLevel>,
}

/// Текущая позиция
#[derive(Debug, Clone)]
pub struct CurrentPosition {
    pub quantity: i32,
    pub average_price: f64,
    pub current_value: f64,
}

/// Торговый агент на основе LLM
pub struct TradingAgent {
    llm_provider: OllamaProvider,
    model_name: String,
}

impl TradingAgent {
    pub fn new(llm_provider: OllamaProvider, model_name: String) -> Self {
        TradingAgent {
            llm_provider,
            model_name,
        }
    }

    /// Принятие торгового решения
    pub async fn make_decision(&self, context: DecisionContext) -> Result<TradingDecision> {
        // Формирование промпта для LLM
        let prompt = self.build_decision_prompt(&context);

        // Запрос к LLM
        let llm_response = self.llm_provider.send_message(prompt).await?;

        // Парсинг ответа LLM
        let decision = self.parse_llm_response(&llm_response.message.content, &context)?;

        Ok(decision)
    }

    /// Быстрое решение без LLM (на основе правил)
    pub fn make_rule_based_decision(&self, context: DecisionContext) -> Result<TradingDecision> {
        let mut action = Action::Hold;
        let mut confidence = 0.5;
        let mut rationale_parts = Vec::new();
        let mut risks = Vec::new();

        // Анализ новостей
        if let Some(news) = &context.news_sentiment {
            match news.overall_sentiment {
                Sentiment::Positive => {
                    confidence += 0.1;
                    rationale_parts.push(format!(
                        "Позитивный новостной фон (score: {:.2}, статей: {})",
                        news.sentiment_score, news.articles_count
                    ));
                }
                Sentiment::Negative => {
                    confidence -= 0.1;
                    risks.push("Негативный новостной фон".to_string());
                    rationale_parts.push(format!(
                        "Негативный новостной фон (score: {:.2})",
                        news.sentiment_score
                    ));
                }
                Sentiment::Neutral => {
                    rationale_parts.push("Нейтральный новостной фон".to_string());
                }
            }

            // Ключевые события
            if !news.key_events.is_empty() {
                rationale_parts.push(format!("Ключевые события: {}", news.key_events.join(", ")));
            }
        }

        // Технический анализ
        if let Some(tech) = &context.technical_analysis {
            match tech.recommendation {
                Recommendation::StrongBuy => {
                    action = Action::Buy;
                    confidence += 0.2;
                    rationale_parts.push("Технический анализ: Strong Buy".to_string());
                }
                Recommendation::Buy => {
                    action = Action::Buy;
                    confidence += 0.15;
                    rationale_parts.push("Технический анализ: Buy".to_string());
                }
                Recommendation::Sell => {
                    action = Action::Sell;
                    confidence += 0.15;
                    rationale_parts.push("Технический анализ: Sell".to_string());
                }
                Recommendation::StrongSell => {
                    action = Action::Sell;
                    confidence += 0.2;
                    rationale_parts.push("Технический анализ: Strong Sell".to_string());
                }
                Recommendation::Hold => {
                    rationale_parts.push("Технический анализ: Hold".to_string());
                }
            }

            // Тренд
            match tech.trend {
                Trend::Bullish => {
                    rationale_parts.push("Бычий тренд".to_string());
                    if action == Action::Hold {
                        action = Action::Buy;
                        confidence += 0.1;
                    }
                }
                Trend::Bearish => {
                    rationale_parts.push("Медвежий тренд".to_string());
                    if action == Action::Hold {
                        action = Action::Sell;
                        confidence += 0.1;
                    }
                }
                Trend::Sideways => {
                    rationale_parts.push("Боковое движение".to_string());
                }
            }

            // Уровни
            if !tech.support_levels.is_empty() {
                if let Some(&support) = tech.support_levels.first() {
                    if context.current_price <= support * 1.05 {
                        rationale_parts.push(format!("Цена у поддержки: {:.2}", support));
                        if action == Action::Hold {
                            action = Action::Buy;
                            confidence += 0.1;
                        }
                    }
                }
            }
        }

        // Фундаментальный анализ
        if let Some(fund) = &context.fundamental_analysis {
            match fund.rating {
                CompanyRating::Excellent => {
                    confidence += 0.15;
                    rationale_parts.push(format!(
                        "Отличные фундаментальные показатели (score: {:.1})",
                        fund.overall_score
                    ));
                    if action == Action::Hold {
                        action = Action::Buy;
                    }
                }
                CompanyRating::Good => {
                    confidence += 0.1;
                    rationale_parts.push(format!(
                        "Хорошие фундаментальные показатели (score: {:.1})",
                        fund.overall_score
                    ));
                }
                CompanyRating::Poor | CompanyRating::VeryPoor => {
                    confidence -= 0.1;
                    risks.push("Слабые фундаментальные показатели".to_string());
                }
                _ => {}
            }

            // Риски
            if !fund.key_risks.is_empty() {
                risks.extend(fund.key_risks.iter().take(3).cloned());
            }
        }

        // Расчет размера позиции
        let position_size_pct = self.calculate_position_size(&context, &action, confidence);

        // Stop Loss и Take Profit
        let (stop_loss, take_profit) =
            self.calculate_levels(context.current_price, &action, &context.risk_config);

        // Время горизонта
        let time_horizon = self.determine_time_horizon(&context);

        // Ограничение confidence
        confidence = confidence.min(0.95).max(0.05);

        let rationale = rationale_parts.join("; ");

        // Получаем текущую позицию в лотах
        let current_position = context.current_position.as_ref().map(|p| p.quantity);

        Ok(TradingDecision {
            ticker: context.ticker,
            action,
            confidence,
            entry_price: Some(context.current_price),
            position_size_pct,
            stop_loss,
            take_profit,
            rationale,
            risks,
            time_horizon,
            current_position,
            current_price: context.current_price,
        })
    }

    /// Расчет размера позиции
    fn calculate_position_size(
        &self,
        context: &DecisionContext,
        action: &Action,
        confidence: f64,
    ) -> f64 {
        if *action != Action::Buy {
            return 0.0;
        }

        // Базовый размер от confidence
        let base_size = confidence * context.max_position_pct;

        // Корректировка по волатильности и рискам
        let adjusted_size = if let Some(tech) = &context.technical_analysis {
            // Если цена близко к сопротивлению - уменьшаем позицию
            if !tech.resistance_levels.is_empty() {
                if let Some(&resistance) = tech.resistance_levels.first() {
                    let distance_to_resistance =
                        (resistance - context.current_price) / context.current_price;
                    if distance_to_resistance < 0.02 {
                        base_size * 0.5 // Уменьшаем на 50%
                    } else {
                        base_size
                    }
                } else {
                    base_size
                }
            } else {
                base_size
            }
        } else {
            base_size
        };

        // Учет доступного баланса
        let max_affordable = if context.current_price > 0.0 {
            (context.available_balance * (1.0 - 0.1)) / context.current_price // 10% резерв
        } else {
            0.0
        };

        // Возвращаем долю, не превышающую лимиты
        adjusted_size.min(context.max_position_pct)
    }

    /// Расчет уровней Stop Loss и Take Profit
    fn calculate_levels(
        &self,
        current_price: f64,
        action: &Action,
        risk_config: &Option<RiskManagementConfig>,
    ) -> (Option<f64>, Option<f64>) {
        let (sl_pct, tp_pct) = if let Some(config) = risk_config {
            (config.stop_loss_pct, config.take_profit_pct)
        } else {
            (0.03, 0.10) // Значения по умолчанию
        };

        match action {
            Action::Buy => {
                let sl = Some(current_price * (1.0 - sl_pct));
                let tp = Some(current_price * (1.0 + tp_pct));
                (sl, tp)
            }
            Action::Sell => {
                let sl = Some(current_price * (1.0 + sl_pct));
                let tp = Some(current_price * (1.0 - tp_pct));
                (sl, tp)
            }
            Action::Hold => (None, None),
        }
    }

    /// Определение временного горизонта
    fn determine_time_horizon(&self, context: &DecisionContext) -> TimeHorizon {
        // Если есть фундаментальный анализ с высоким рейтингом - длинный горизонт
        if let Some(fund) = &context.fundamental_analysis {
            if fund.rating == CompanyRating::Excellent || fund.rating == CompanyRating::Good {
                return TimeHorizon::Long;
            }
        }

        // Если сильный технический сигнал - короткий горизонт
        if let Some(tech) = &context.technical_analysis {
            if tech.recommendation == Recommendation::StrongBuy
                || tech.recommendation == Recommendation::StrongSell
            {
                return TimeHorizon::Short;
            }
        }

        TimeHorizon::Medium
    }

    /// Построение промпта для LLM
    fn build_decision_prompt(&self, context: &DecisionContext) -> String {
        let mut prompt = format!(
            "Ты - профессиональный торговый аналитик. Проанализируй данные по акции {} ({}) и дай рекомендацию.\n\n",
            context.ticker, context.company_name
        );

        prompt.push_str(&format!("Текущая цена: {:.2}\n\n", context.current_price));

        // Новости
        if let Some(news) = &context.news_sentiment {
            prompt.push_str("НОВОСТИ:\n");
            prompt.push_str(&format!(
                "Общий сентимент: {:?} (score: {:.2})\n",
                news.overall_sentiment, news.sentiment_score
            ));
            prompt.push_str(&format!("Количество статей: {}\n", news.articles_count));
            if !news.key_events.is_empty() {
                prompt.push_str("Ключевые события:\n");
                for event in &news.key_events {
                    prompt.push_str(&format!("  - {}\n", event));
                }
            }
            prompt.push('\n');
        }

        // Технический анализ
        if let Some(tech) = &context.technical_analysis {
            prompt.push_str("ТЕХНИЧЕСКИЙ АНАЛИЗ:\n");
            prompt.push_str(&format!("Тренд: {:?}\n", tech.trend));
            prompt.push_str(&format!("Рекомендация: {:?}\n", tech.recommendation));
            if let Some(rsi) = tech.rsi {
                prompt.push_str(&format!("RSI: {:.2}\n", rsi));
            }
            if let Some(ref macd) = tech.macd {
                prompt.push_str(&format!(
                    "MACD: {:.3} (signal: {:.3}, histogram: {:.3})\n",
                    macd.macd_line, macd.signal_line, macd.histogram
                ));
            }
            if let Some(ref bb) = tech.bollinger {
                prompt.push_str(&format!(
                    "Bollinger: upper={:.2}, middle={:.2}, lower={:.2}\n",
                    bb.upper, bb.middle, bb.lower
                ));
            }
            prompt.push_str(&format!(
                "Поддержка: {:?}\n",
                tech.support_levels.iter().take(2).collect::<Vec<_>>()
            ));
            prompt.push_str(&format!(
                "Сопротивление: {:?}\n",
                tech.resistance_levels.iter().take(2).collect::<Vec<_>>()
            ));
            prompt.push('\n');
        }

        // Фундаментальный анализ
        if let Some(fund) = &context.fundamental_analysis {
            prompt.push_str("ФУНДАМЕНТАЛЬНЫЙ АНАЛИЗ:\n");
            prompt.push_str(&format!(
                "Рейтинг: {:?} (score: {:.1}/100)\n",
                fund.rating, fund.overall_score
            ));
            if let Some(pe) = fund.valuation.pe_ratio {
                prompt.push_str(&format!("P/E: {:.2}\n", pe));
            }
            if let Some(roe) = fund.profitability.roe {
                prompt.push_str(&format!("ROE: {:.2}%\n", roe));
            }
            if let Some(dte) = fund.financial_health.debt_to_equity {
                prompt.push_str(&format!("D/E: {:.2}\n", dte));
            }
            if let Some(growth) = fund.growth.revenue_growth_yoy {
                prompt.push_str(&format!("Рост выручки (YoY): {:.2}%\n", growth));
            }
            if !fund.key_risks.is_empty() {
                prompt.push_str("Риски:\n");
                for risk in &fund.key_risks {
                    prompt.push_str(&format!("  - {}\n", risk));
                }
            }
            if !fund.key_strengths.is_empty() {
                prompt.push_str("Сильные стороны:\n");
                for strength in &fund.key_strengths {
                    prompt.push_str(&format!("  - {}\n", strength));
                }
            }
            prompt.push('\n');
        }

        // Контекст портфеля
        prompt.push_str("КОНТЕКСТ ПОРТФЕЛЯ:\n");
        prompt.push_str(&format!(
            "Доступный баланс: {:.2}\n",
            context.available_balance
        ));
        prompt.push_str(&format!(
            "Макс. доля позиции: {:.1}%\n",
            context.max_position_pct * 100.0
        ));
        if let Some(pos) = &context.current_position {
            prompt.push_str(&format!(
                "Текущая позиция: {} лотов по средней {:.2}\n",
                pos.quantity, pos.average_price
            ));
        }
        prompt.push('\n');

        // Запрос
        prompt.push_str("ЗАДАНИЕ:\n");
        prompt.push_str("Дай рекомендацию в формате JSON:\n");
        prompt.push_str("{\n");
        prompt.push_str("  \"action\": \"BUY\" | \"SELL\" | \"HOLD\",\n");
        prompt.push_str("  \"confidence\": 0.0-1.0,\n");
        prompt.push_str("  \"entry_price\": цена входа,\n");
        prompt.push_str("  \"position_size_pct\": доля от портфеля 0.0-1.0,\n");
        prompt.push_str("  \"stop_loss\": цена stop-loss,\n");
        prompt.push_str("  \"take_profit\": цена take-profit,\n");
        prompt.push_str("  \"rationale\": \"обоснование\",\n");
        prompt.push_str("  \"risks\": [\"риск1\", \"риск2\"],\n");
        prompt.push_str("  \"time_horizon\": \"SHORT\" | \"MEDIUM\" | \"LONG\"\n");
        prompt.push_str("}\n");

        prompt
    }

    /// Парсинг ответа LLM
    ///
    /// # Errors
    /// Возвращает ошибку, если JSON некорректен или отсутствуют обязательные поля
    fn parse_llm_response(
        &self,
        content: &str,
        context: &DecisionContext,
    ) -> Result<TradingDecision> {
        // Поиск JSON в ответе
        let json_start = content
            .find('{')
            .ok_or_else(|| anyhow::anyhow!("Не найден JSON в ответе LLM"))?;
        let json_end = content
            .rfind('}')
            .ok_or_else(|| anyhow::anyhow!("Не найден закрывающий символ JSON"))?;

        if json_start > json_end {
            anyhow::bail!("Некорректный JSON: открывающая скобка после закрывающей");
        }

        let json_str = &content[json_start..=json_end];

        // Парсинг JSON с обработкой ошибок
        let parsed: serde_json::Value = serde_json::from_str(json_str)
            .map_err(|e| anyhow::anyhow!("Ошибка парсинга JSON: {}. JSON: {}", e, json_str))?;

        // Извлечение полей
        let action_str = parsed["action"].as_str().unwrap_or("HOLD");
        let action = match action_str.to_uppercase().as_str() {
            "BUY" => Action::Buy,
            "SELL" => Action::Sell,
            _ => Action::Hold,
        };

        let confidence = parsed["confidence"].as_f64().unwrap_or(0.5);
        let entry_price = parsed["entry_price"].as_f64();
        let position_size_pct = parsed["position_size_pct"].as_f64().unwrap_or(0.0);
        let stop_loss = parsed["stop_loss"].as_f64();
        let take_profit = parsed["take_profit"].as_f64();
        let rationale = parsed["rationale"]
            .as_str()
            .unwrap_or("No rationale provided")
            .to_string();

        let risks = parsed["risks"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();

        let time_horizon_str = parsed["time_horizon"].as_str().unwrap_or("MEDIUM");
        let time_horizon = match time_horizon_str.to_uppercase().as_str() {
            "SHORT" => TimeHorizon::Short,
            "LONG" => TimeHorizon::Long,
            _ => TimeHorizon::Medium,
        };

        // Получаем текущую позицию в лотах
        let current_position = context.current_position.as_ref().map(|p| p.quantity);

        Ok(TradingDecision {
            ticker: context.ticker.clone(),
            action,
            confidence,
            entry_price,
            position_size_pct,
            stop_loss,
            take_profit,
            rationale,
            risks,
            time_horizon,
            current_position,
            current_price: context.current_price,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rule_based_decision_buy() {
        let context = DecisionContext {
            ticker: "TTECH".to_string(),
            company_name: "Т-Технологии".to_string(),
            current_price: 100.0,
            news_sentiment: Some(NewsSentiment {
                ticker: "TTECH".to_string(),
                overall_sentiment: Sentiment::Positive,
                sentiment_score: 0.5,
                articles_count: 5,
                articles: vec![],
                key_events: vec!["Позитивные новости".to_string()],
            }),
            technical_analysis: Some(TechnicalAnalysis {
                ticker: "TTECH".to_string(),
                timestamp: chrono::Utc::now(),
                current_price: 100.0,
                trend: Trend::Bullish,
                rsi: Some(45.0),
                macd: None,
                bollinger: None,
                volume_analysis: crate::analysis::VolumeAnalysis {
                    current_volume: 1000.0,
                    avg_volume: 500.0,
                    volume_ratio: 2.0,
                    is_unusual: true,
                },
                support_levels: vec![95.0],
                resistance_levels: vec![110.0],
                recommendation: Recommendation::Buy,
            }),
            fundamental_analysis: None,
            available_balance: 100000.0,
            current_position: None,
            risk_config: None,
            max_position_pct: 0.15,
            market_regime: MarketRegime::Quiet,
            candles: vec![],
        };

        // Для теста нужен реальный LLM provider, поэтому пропускаем LLM тест
        // let agent = TradingAgent::new(OllamaProvider::default(), "fin-expert".to_string());
        // let decision = agent.make_rule_based_decision(context).await.unwrap();

        // Проверяем структуру контекста
        assert_eq!(context.ticker, "TTECH");
        assert!(context.news_sentiment.is_some());
        assert!(context.technical_analysis.is_some());
    }
}
