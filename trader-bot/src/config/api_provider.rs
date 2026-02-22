use crate::config::TradingConfig;

pub trait ApiProvider {
    fn get_api_token(config: &TradingConfig) -> String;
}

impl ApiProvider for TradingConfig {
    fn get_api_token(config: &TradingConfig) -> String {
        config.creditional.token.clone()
    }
}
