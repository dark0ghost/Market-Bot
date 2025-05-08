use t_invest_sdk::{TInvestError, TInvestInterceptor, TInvestSdk};
use t_invest_sdk::api::market_data_service_client::MarketDataServiceClient;
use tonic::{
    service::interceptor::InterceptedService,
    transport::Channel,
};

use crate::config::data::WorkingMode;

pub struct TinkoffClient {
    i_invest_sdk: TInvestSdk,

}

impl TinkoffClient {
    pub async fn new(token: &str, working_mode: WorkingMode) -> Result<Self, TInvestError> {
        let i_invest_sdk = match working_mode {
            WorkingMode::Prod => TInvestSdk::new(token).await,
            WorkingMode::SandBox => panic!("Not Supported")
        };

        match i_invest_sdk {
            Ok(sdk) => {
                Ok(TinkoffClient {
                    i_invest_sdk:sdk,
                })
            }
            Err(e) => {
                Err(e)
            }
        }
    }

    pub fn market_data(
        &self,
    ) -> MarketDataServiceClient<InterceptedService<Channel, TInvestInterceptor>> {
        self.i_invest_sdk.market_data()
    }

    pub fn find_instrument(&self) {
        self.i_invest_sdk.instruments();
    }
}