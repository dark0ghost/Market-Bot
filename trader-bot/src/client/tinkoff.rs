use t_invest_sdk::{Environment, TInvestError, TInvestInterceptor, TInvestSdk};
use t_invest_sdk::api::{FindInstrumentRequest, InstrumentType};
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
        let i_invest_sdk = TInvestSdk::new(token, match working_mode {
            WorkingMode::Prod => Environment::Production,
            WorkingMode::SandBox => Environment::Sandbox
        }).await;

        match i_invest_sdk {
            Ok(sdk) => {
                Ok(TinkoffClient {
                    i_invest_sdk: sdk,
                })
            }
            Err(e) => {
                Err(e)
            }
        }
    }

    pub async fn find_instrument(&self) {
        // self
        //     .i_invest_sdk
        //     .instruments()
        //     .find_instrument(FindInstrumentRequest {
        //     query: "Т-Технологии".to_string(),
        //     instrument_kind: Some(InstrumentType::Share as i32),
        //     api_trade_available_flag: Some(true),
        // })
        //     .await
        //     .into_inner();
    }
}