use std::env;

use crate::config::api_provider::AccountProvider;
use crate::utils::from_env;

pub struct DemoAccount {
    api_token: String,

    account_number: String
}

impl DemoAccount {
    fn from_env() -> DemoAccount {
        DemoAccount {
            api_token: from_env("DEMO_API_TOKEN"),
            account_number: env::var("DEMO_API_TOKEN").unwrap(),
        }
    }
}

impl AccountProvider for DemoAccount {
    fn get_account_number(self) -> String {
        self.account_number
    }
}