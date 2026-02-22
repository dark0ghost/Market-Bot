use serde::{Deserialize, Serialize};
use std::fs;
use anyhow::Result;

/// Основная конфигурация торгового бота
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TradingConfig {
    #[serde(rename = "type")]
    pub config_type: String,
    pub credential: Credential,
    pub accounts: Vec<AccountConfig>,
    pub mode: WorkingMode,
    pub llm_config: Option<LlmConfig>,
}

/// Учетные данные API
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Credential {
    pub token: String,
}

/// Конфигурация счета
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AccountConfig {
    pub account_id: String,
    pub instruments: Vec<InstrumentConfig>,
    pub strategy: StrategyConfig,
    pub risk_management: Option<RiskManagementConfig>,
}

/// Конфигурация инструмента
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct InstrumentConfig {
    pub figi: String,
    pub ticker: String,
    pub name: String,
    pub enabled: bool,
    pub max_position_pct: f64,
    pub analysis_config: AnalysisConfig,
}

/// Конфигурация анализа инструмента
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AnalysisConfig {
    pub check_news: bool,
    pub technical_analysis: bool,
    pub fundamental_analysis: bool,
    pub news_sources: Vec<String>,
    pub technical_indicators: Vec<String>,
    pub fundamental_metrics: Vec<String>,
}

/// Конфигурация стратегии
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct StrategyConfig {
    pub strategy: StrategyType,
    pub parameters: StrategyParameters,
}

/// Тип стратегии
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum StrategyType {
    Interval,
    Momentum,
    MeanReversion,
    Grid,
}

/// Параметры стратегии
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct StrategyParameters {
    pub interval_size: String,
    pub days_back_to_consider: u32,
    pub quantity_limit: u32,
    pub check_interval: u32,
    /// Параметры для Grid стратегии
    #[serde(default)]
    pub grid_config: Option<GridConfig>,
}

/// Конфигурация Grid стратегии
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GridConfig {
    /// Нижняя граница диапазона (цена)
    pub lower_price: f64,
    /// Верхняя граница диапазона (цена)
    pub upper_price: f64,
    /// Количество уровней сетки
    pub grid_levels: u32,
    /// Размер ордера в лотах для каждого уровня
    pub order_size: u32,
    /// Процент сетки для каждой стороны (0.5 = 50% на покупку, 50% на продажу)
    #[serde(default = "default_grid_ratio")]
    pub grid_ratio: f64,
}

fn default_grid_ratio() -> f64 {
    0.5
}

/// Управление рисками
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RiskManagementConfig {
    pub max_loss_pct: f64,
    pub take_profit_pct: f64,
    pub stop_loss_pct: f64,
    pub max_open_positions: u32,
    pub min_balance_reserve: f64,
}

/// Режим работы
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum WorkingMode {
    Sandbox,
    Prod,
}

/// Конфигурация LLM
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LlmConfig {
    pub model: String,
    pub host: String,
    pub port: u16,
    pub temperature: f32,
    pub context_window: u32,
}

impl TradingConfig {
    /// Загрузить конфигурацию из файла
    pub fn load(path: &str) -> Result<Self> {
        let content = fs::read_to_string(path)?;
        let config: TradingConfig = serde_json::from_str(&content)?;
        Ok(config)
    }

    /// Загрузить конфигурацию из пути по умолчанию
    pub fn load_default() -> Result<Self> {
        Self::load("trader-bot/config/account.json")
    }

    /// Получить активные инструменты
    pub fn get_enabled_instruments(&self) -> Vec<&InstrumentConfig> {
        self.accounts
            .iter()
            .flat_map(|acc| acc.instruments.iter())
            .filter(|inst| inst.enabled)
            .collect()
    }

    /// Получить инструмент по тикеру
    pub fn get_instrument_by_ticker(&self, ticker: &str) -> Option<&InstrumentConfig> {
        self.accounts
            .iter()
            .flat_map(|acc| acc.instruments.iter())
            .find(|inst| inst.ticker == ticker)
    }
}
