use crate::agent::CurrentPosition;
use anyhow::Result;
use t_invest_sdk::TInvestSdk;
use t_invest_sdk::api::{GetAccountsRequest, PortfolioPosition, PortfolioRequest};

/// Сервис для работы с портфелем и счетами
pub struct PortfolioService {
    sdk: TInvestSdk,
    account_id: String,
}

impl PortfolioService {
    pub fn new(sdk: TInvestSdk, account_id: String) -> Self {
        PortfolioService { sdk, account_id }
    }

    /// Получение списка счетов
    pub async fn get_accounts(&self) -> Result<Vec<AccountInfo>> {
        let request = GetAccountsRequest {
            status: None, // Все счета
        };
        let response = self.sdk.users().get_accounts(request).await?;
        let accounts_response = response.into_inner();

        let accounts = accounts_response
            .accounts
            .into_iter()
            .map(|acc| AccountInfo {
                id: acc.id,
                name: acc.name,
                status: acc.status,
                access_level: acc.access_level,
            })
            .collect();

        Ok(accounts)
    }

    /// Получение текущего портфеля
    pub async fn get_portfolio(&self) -> Result<PortfolioInfo> {
        let request = PortfolioRequest {
            account_id: self.account_id.clone(),
            currency: None,
        };

        let response = self.sdk.operations().get_portfolio(request).await?;
        let portfolio_response = response.into_inner();

        let total_amount = portfolio_response
            .total_amount_shares
            .or(portfolio_response.total_amount_bonds)
            .or(portfolio_response.total_amount_etf)
            .or(portfolio_response.total_amount_currencies)
            .or(portfolio_response.total_amount_futures)
            .and_then(|q| Some(q.units as f64 + q.nano as f64 / 1_000_000_000.0))
            .unwrap_or(0.0);

        let positions = portfolio_response
            .positions
            .into_iter()
            .map(|pos| PositionInfo::from(pos))
            .collect();

        Ok(PortfolioInfo {
            total_amount,
            positions,
            virtual_positions: vec![],
        })
    }

    /// Получение доступного баланса (свободные деньги)
    pub async fn get_available_balance(&self) -> Result<f64> {
        let portfolio = self.get_portfolio().await?;

        // Получаем баланс в рублях - ищем денежные позиции
        let rub_balance = portfolio
            .positions
            .iter()
            .find(|p| p.instrument_type == "bond" || p.instrument_type == "currency")
            .map(|p| p.current_balance)
            .unwrap_or(0.0);

        Ok(rub_balance)
    }

    /// Получение текущей позиции по инструменту
    pub async fn get_position(&self, instrument_uid: &str) -> Result<Option<CurrentPosition>> {
        let portfolio = self.get_portfolio().await?;

        for position in portfolio.positions {
            if position.uid == instrument_uid {
                return Ok(Some(CurrentPosition {
                    quantity: position.quantity as i32,
                    average_price: position.average_position_price,
                    current_value: position.current_balance * position.current_price,
                }));
            }
        }

        Ok(None)
    }

    /// Получение всех открытых позиций
    pub async fn get_all_positions(&self) -> Result<Vec<PositionInfo>> {
        let portfolio = self.get_portfolio().await?;
        Ok(portfolio.positions)
    }
}

/// Информация о счете
#[derive(Debug, Clone)]
pub struct AccountInfo {
    pub id: String,
    pub name: String,
    pub status: i32,
    pub access_level: i32,
}

/// Информация о портфеле
#[derive(Debug, Clone)]
pub struct PortfolioInfo {
    pub total_amount: f64,
    pub positions: Vec<PositionInfo>,
    pub virtual_positions: Vec<PositionInfo>,
}

/// Информация о позиции
#[derive(Debug, Clone)]
pub struct PositionInfo {
    pub uid: String,
    pub instrument_type: String,
    pub quantity: i64,
    pub current_balance: f64,
    pub current_price: f64,
    pub average_position_price: f64,
    pub expected_yield: f64,
}

impl From<PortfolioPosition> for PositionInfo {
    fn from(pos: PortfolioPosition) -> Self {
        let current_balance = pos
            .current_price
            .as_ref()
            .map(|q| q.units as f64 + q.nano as f64 / 1_000_000_000.0)
            .unwrap_or(0.0);

        let current_price = pos
            .current_price
            .as_ref()
            .map(|q| q.units as f64 + q.nano as f64 / 1_000_000_000.0)
            .unwrap_or(0.0);

        let average_price = pos
            .average_position_price
            .as_ref()
            .map(|q| q.units as f64 + q.nano as f64 / 1_000_000_000.0)
            .unwrap_or(0.0);

        let expected_yield = pos
            .expected_yield
            .as_ref()
            .map(|q| q.units as f64 + q.nano as f64 / 1_000_000_000.0)
            .unwrap_or(0.0);

        PositionInfo {
            uid: pos.instrument_uid,
            instrument_type: pos.instrument_type,
            quantity: pos.quantity.map(|q| q.units).unwrap_or(0),
            current_balance,
            current_price,
            average_position_price: average_price,
            expected_yield,
        }
    }
}
