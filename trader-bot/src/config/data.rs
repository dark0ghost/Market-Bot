use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs;

/// Основная конфигурация торгового бота (расширенная)
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TradingConfig {
    #[serde(rename = "type")]
    pub config_type: String,
    pub credential: Credential,
    pub accounts: Vec<AccountConfig>,
    pub mode: WorkingMode,
    pub llm_config: Option<LlmConfig>,
    pub dashboard: Option<DashboardConfig>,

    /// Список дополнительных источников данных
    pub data_sources: Option<Vec<DataSourceConfig>>,

    /// Настройки оптимизатора
    pub optimizer: Option<OptimizerSection>,
}

/// Учетные данные API
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Credential {
    pub token: String,
    /// Дополнительные ключи для других брокеров
    #[serde(default)]
    pub additional_keys: Option<Vec<BrokerCredential>>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct BrokerCredential {
    pub broker: String,
    pub api_key: String,
    pub secret_key: Option<String>,
    pub extra: Option<std::collections::HashMap<String, String>>,
}

/// Конфигурация счета
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AccountConfig {
    pub account_id: String,
    /// Брокер для этого счета (по умолчанию "tinkoff")
    #[serde(default = "default_broker")]
    pub broker: String,
    pub instruments: Vec<InstrumentConfig>,
    pub strategy: StrategyConfig,
    pub risk_management: Option<RiskManagementConfig>,
}

fn default_broker() -> String {
    "tinkoff".to_string()
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
    PairsTrading,
    StatisticalArbitrage,
    #[serde(other)]
    Custom,
}

/// Параметры стратегии
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct StrategyParameters {
    pub interval_size: String,
    pub days_back_to_consider: u32,
    pub quantity_limit: u32,
    pub check_interval: u32,
    #[serde(default)]
    pub grid_config: Option<GridConfig>,
    #[serde(default)]
    pub pairs_config: Option<PairConfig>,
}

/// Конфигурация Grid стратегии
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GridConfig {
    pub lower_price: f64,
    pub upper_price: f64,
    pub grid_levels: u32,
    pub order_size: u32,
    #[serde(default = "default_grid_ratio")]
    pub grid_ratio: f64,
}

fn default_grid_ratio() -> f64 {
    0.5
}

/// Конфигурация парной торговли
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PairConfig {
    pub pair_ticker: String,
    pub pair_figi: String,
    pub entry_zscore: f64,
    pub exit_zscore: f64,
    pub lookback_period: u32,
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

/// Конфигурация дашборда
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DashboardConfig {
    #[serde(default = "default_dashboard_port")]
    pub port: u16,
    #[serde(default = "default_dashboard_enabled")]
    pub enabled: bool,
}

fn default_dashboard_port() -> u16 {
    8080
}

fn default_dashboard_enabled() -> bool {
    true
}

/// Конфигурация внешнего источника данных
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DataSourceConfig {
    pub name: String,
    pub source_type: String,
    pub api_key: Option<String>,
    pub base_url: Option<String>,
}

/// Настройки оптимизатора
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct OptimizerSection {
    pub enabled: bool,
    pub method: String,
    pub metric: String,
    pub max_iterations: u32,
}

impl TradingConfig {
    pub fn load(path: &str) -> Result<Self> {
        let content = fs::read_to_string(path)?;
        let config: TradingConfig = serde_json::from_str(&content)?;
        Ok(config)
    }

    pub fn load_default() -> Result<Self> {
        Self::load("trader-bot/config/account.json")
    }

    pub fn get_enabled_instruments(&self) -> Vec<&InstrumentConfig> {
        self.accounts
            .iter()
            .flat_map(|acc| acc.instruments.iter())
            .filter(|inst| inst.enabled)
            .collect()
    }

    pub fn get_instrument_by_ticker(&self, ticker: &str) -> Option<&InstrumentConfig> {
        self.accounts
            .iter()
            .flat_map(|acc| acc.instruments.iter())
            .find(|inst| inst.ticker == ticker)
    }
}
