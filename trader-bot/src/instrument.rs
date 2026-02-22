use serde::{Deserialize, Serialize};

/// Торговый инструмент
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Instrument {
    pub figi: String,
    pub ticker: String,
    pub name: String,
    pub enabled: bool,
}

impl Instrument {
    pub fn new(figi: String, ticker: String, name: String) -> Self {
        Instrument {
            figi,
            ticker,
            name,
            enabled: true,
        }
    }
}
