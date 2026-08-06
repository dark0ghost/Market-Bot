use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;

/// Main trading bot configuration (extended)
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TradingConfig {
    #[serde(rename = "type")]
    pub config_type: String,
    pub credential: Credential,
    pub accounts: Vec<AccountConfig>,
    pub mode: WorkingMode,
    pub llm_config: Option<LlmConfig>,
    pub dashboard: Option<DashboardConfig>,

    /// List of additional data sources
    pub data_sources: Option<Vec<DataSourceConfig>>,

    /// Optimizer settings
    pub optimizer: Option<OptimizerSection>,

    /// Sandbox settings (only used when mode = sandbox)
    pub sandbox: Option<SandboxConfig>,
}

/// API credentials
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Credential {
    pub token: String,
    /// Additional keys for other brokers
    #[serde(default)]
    pub additional_keys: Option<Vec<BrokerCredential>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum BrokerType {
    #[default]
    Tinkoff,
    Finam,
    Mock,
}

impl BrokerType {
    pub const fn as_str(&self) -> &'static str {
        match self {
            BrokerType::Tinkoff => "tinkoff",
            BrokerType::Finam => "finam",
            BrokerType::Mock => "mock",
        }
    }
}

impl std::fmt::Display for BrokerType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct BrokerCredential {
    pub broker: BrokerType,
    pub api_key: String,
    pub secret_key: Option<String>,
    pub extra: Option<std::collections::HashMap<String, String>>,
}

/// Account configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AccountConfig {
    #[serde(default)]
    pub account_id: Option<String>,
    /// Broker for this account (default "tinkoff")
    #[serde(default = "default_broker")]
    pub broker: BrokerType,
    pub instruments: Vec<InstrumentConfig>,
    pub strategy: StrategyConfig,
    pub risk_management: Option<RiskManagementConfig>,
}

const fn default_broker() -> BrokerType {
    BrokerType::Tinkoff
}

/// Instrument configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct InstrumentConfig {
    pub figi: String,
    pub ticker: String,
    pub name: String,
    pub enabled: bool,
    pub max_position_pct: f64,
    pub analysis_config: AnalysisConfig,
}

/// Instrument analysis configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AnalysisConfig {
    pub check_news: bool,
    pub technical_analysis: bool,
    pub fundamental_analysis: bool,
    pub news_sources: Vec<String>,
    pub technical_indicators: Vec<String>,
    pub fundamental_metrics: Vec<String>,
}

/// Strategy configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct StrategyConfig {
    pub strategy: StrategyType,
    pub parameters: StrategyParameters,
}

/// Strategy type
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum StrategyType {
    Interval,
    Momentum,
    MeanReversion,
    Grid,
    Ai,
    PairsTrading,
    StatisticalArbitrage,
    #[serde(other)]
    Custom,
}

/// AI strategy configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AiConfig {
    /// Use LLM for decisions (if false, rule-based is used)
    #[serde(default = "default_ai_use_llm")]
    pub use_llm: bool,
    /// Use FinBERT for news sentiment (if false, Ollama LLM is used)
    #[serde(default = "default_ai_use_finbert")]
    pub use_finbert: bool,
    /// Minimum confidence threshold to execute trades
    #[serde(default = "default_ai_min_confidence")]
    pub min_confidence: f64,
    /// Override market regime detection
    #[serde(default)]
    pub force_regime: Option<String>,
    /// Path to decision memory JSON file (dual persistence: RAM + flash)
    #[serde(default)]
    pub memory_path: Option<String>,
}

fn default_ai_use_llm() -> bool {
    true
}

fn default_ai_use_finbert() -> bool {
    false
}

fn default_ai_min_confidence() -> f64 {
    0.6
}

impl Default for AiConfig {
    fn default() -> Self {
        AiConfig {
            use_llm: default_ai_use_llm(),
            use_finbert: default_ai_use_finbert(),
            min_confidence: default_ai_min_confidence(),
            force_regime: None,
            memory_path: None,
        }
    }
}

/// Strategy parameters
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
    #[serde(default)]
    pub ai_config: Option<AiConfig>,
}

/// Grid strategy configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GridConfig {
    pub lower_price: f64,
    pub upper_price: f64,
    pub grid_levels: u32,
    pub order_size: u32,
    #[serde(default = "default_grid_ratio")]
    pub grid_ratio: f64,
}

const fn default_grid_ratio() -> f64 {
    0.5
}

/// Pairs trading configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PairConfig {
    pub pair_ticker: String,
    pub pair_figi: String,
    pub entry_zscore: f64,
    pub exit_zscore: f64,
    pub lookback_period: u32,
}

/// Risk management
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RiskManagementConfig {
    pub max_loss_pct: f64,
    pub take_profit_pct: f64,
    pub stop_loss_pct: f64,
    pub max_open_positions: u32,
    pub min_balance_reserve: f64,
}

/// Working mode
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum WorkingMode {
    Sandbox,
    Prod,
}

/// Sandbox configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SandboxConfig {
    /// Auto-create sandbox account if account_id is not set
    #[serde(default = "default_sandbox_open_account")]
    pub open_account: bool,
    /// Amount to deposit into sandbox account (RUB)
    #[serde(default = "default_sandbox_pay_in")]
    pub pay_in_amount: f64,
}

fn default_sandbox_open_account() -> bool {
    true
}
fn default_sandbox_pay_in() -> f64 {
    30_000_000.0
}

/// LLM configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LlmConfig {
    pub model: String,
    pub host: String,
    pub port: u16,
    pub temperature: f32,
    pub context_window: u32,
}

/// Dashboard configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DashboardConfig {
    #[serde(default = "default_dashboard_port")]
    pub port: u16,
    #[serde(default = "default_dashboard_enabled")]
    pub enabled: bool,
}

const fn default_dashboard_port() -> u16 {
    8080
}

const fn default_dashboard_enabled() -> bool {
    true
}

/// External data source configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DataSourceConfig {
    pub name: String,
    pub source_type: String,
    pub api_key: Option<String>,
    pub base_url: Option<String>,
}

/// Optimizer settings
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
        let path = resolve_config_path();
        Self::load(&path).with_context(|| format!("failed to load trading config from '{path}'"))
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

/// Scan `std::env::args()` for a `--config <path>` or `--config=<path>` flag.
///
/// Returns the first value found, or `None` if the flag is absent.
fn config_arg_from_args<I>(args: I) -> Option<String>
where
    I: IntoIterator<Item = String>,
{
    let mut iter = args.into_iter();
    while let Some(arg) = iter.next() {
        if let Some(value) = arg.strip_prefix("--config=") {
            return Some(value.to_string());
        }
        if arg == "--config" {
            return iter.next();
        }
    }
    None
}

/// Pure path-selection logic used by [`resolve_config_path`].
///
/// Resolution order (first match wins):
/// 1. `cli_arg` (from `--config`), if present.
/// 2. `env_var` (from `CONFIG_PATH`), if present.
/// 3. First candidate that satisfies `exists`.
/// 4. Fallback: the last candidate (so the error message is sensible), or an
///    empty string if there are no candidates.
fn pick_config_path(
    cli_arg: Option<String>,
    env_var: Option<String>,
    candidates: &[&str],
    exists: impl Fn(&str) -> bool,
) -> String {
    if let Some(cli) = cli_arg {
        return cli;
    }
    if let Some(env) = env_var {
        return env;
    }
    if let Some(found) = candidates.iter().find(|c| exists(c)) {
        return (*found).to_string();
    }
    candidates
        .last()
        .map(|c| (*c).to_string())
        .unwrap_or_default()
}

/// Resolve the trading config path in a working-directory-robust way.
///
/// Precedence: `--config`/`--config=` CLI arg, then the `CONFIG_PATH` env var,
/// then the first existing candidate relative path.
pub fn resolve_config_path() -> String {
    let cli_arg = config_arg_from_args(std::env::args());
    let env_var = std::env::var("CONFIG_PATH").ok();
    let candidates = ["trader-bot/config/account.json", "config/account.json"];
    pick_config_path(cli_arg, env_var, &candidates, |p| {
        std::path::Path::new(p).is_file()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cli_arg_wins_over_env_and_candidates() {
        let path = pick_config_path(
            Some("cli.json".to_string()),
            Some("env.json".to_string()),
            &["a.json", "b.json"],
            |_| true,
        );
        assert_eq!(path, "cli.json");
    }

    #[test]
    fn env_wins_over_candidates() {
        let path = pick_config_path(
            None,
            Some("env.json".to_string()),
            &["a.json", "b.json"],
            |_| true,
        );
        assert_eq!(path, "env.json");
    }

    #[test]
    fn first_existing_candidate_is_chosen() {
        let path = pick_config_path(None, None, &["a.json", "b.json"], |p| p == "b.json");
        assert_eq!(path, "b.json");
    }

    #[test]
    fn falls_back_to_last_candidate_when_none_exist() {
        let path = pick_config_path(None, None, &["a.json", "b.json"], |_| false);
        assert_eq!(path, "b.json");
    }

    #[test]
    fn parses_config_flag_space_separated() {
        let args = vec![
            "trader-bot".to_string(),
            "--config".to_string(),
            "custom.json".to_string(),
        ];
        assert_eq!(config_arg_from_args(args), Some("custom.json".to_string()));
    }

    #[test]
    fn parses_config_flag_equals_separated() {
        let args = vec!["trader-bot".to_string(), "--config=custom.json".to_string()];
        assert_eq!(config_arg_from_args(args), Some("custom.json".to_string()));
    }

    #[test]
    fn no_config_flag_returns_none() {
        let args = vec!["trader-bot".to_string(), "--other".to_string()];
        assert_eq!(config_arg_from_args(args), None);
    }

    #[test]
    fn broker_type_parses_lowercase_strings() {
        for (json, expected) in [
            ("\"tinkoff\"", BrokerType::Tinkoff),
            ("\"finam\"", BrokerType::Finam),
            ("\"mock\"", BrokerType::Mock),
        ] {
            let parsed: BrokerType = serde_json::from_str(json).unwrap();
            assert_eq!(parsed, expected);
        }
        // Unknown broker string must error rather than silently map.
        assert!(serde_json::from_str::<BrokerType>("\"alor\"").is_err());
    }

    #[test]
    fn broker_type_serializes_lowercase() {
        assert_eq!(
            serde_json::to_string(&BrokerType::Finam).unwrap(),
            "\"finam\""
        );
    }
}
