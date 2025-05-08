use serde::{Deserialize};
use std::fs;

#[derive(Debug, Deserialize)]
struct Config {
    #[serde(rename = "type")]
    config_type: String,
    creditional: Credential,
    accounts: Vec<Account>,
    #[serde(rename = "mode")]
    working_mode: WorkingMode
}

#[derive(Debug, Deserialize)]
struct Credential {
    token: String,
}

#[derive(Debug, Deserialize)]
struct Account {
    #[serde(flatten)]
    account_data: std::collections::HashMap<String, Vec<Strategy>>,
}

#[derive(Debug, Deserialize)]
struct Strategy {
    strategy: StrategyType,
    parameters: Parameters,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
enum StrategyType {
    Interval,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum WorkingMode {
    SandBox,
    Prod
}


#[derive(Debug, Deserialize)]
struct Parameters {
    interval_size: String,
    days_back_to_consider: u32,
    quantity_limit: u32,
    check_interval: u32
}