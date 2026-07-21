use anyhow::Result;
use crate::agent::CurrentPosition;
use crate::client::portfolio::{PortfolioInfo, PortfolioService};

pub trait PortfolioProvider {
    async fn get_portfolio(&self) -> Result<PortfolioInfo>;

    async fn get_available_balance(&self) -> Result<f64>;

    async fn get_position(&self, instrument_uid: &str) -> Result<Option<CurrentPosition>>;
}

impl PortfolioProvider for PortfolioService {
    async fn get_portfolio(&self) -> Result<PortfolioInfo> {
        self.get_portfolio().await
    }

    async fn get_available_balance(&self) -> Result<f64> {
        self.get_available_balance().await
    }

    async fn get_position(&self, instrument_uid: &str) -> Result<Option<CurrentPosition>> {
        self.get_position(instrument_uid).await
    }
}
