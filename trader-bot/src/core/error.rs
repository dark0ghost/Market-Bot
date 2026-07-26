use thiserror::Error;

#[derive(Error, Debug)]
pub enum CoreError {
    #[error("Broker error: {0}")]
    Broker(String),

    #[error("DataSource error: {0}")]
    DataSource(String),

    #[error("Strategy error: {0}")]
    Strategy(String),

    #[error("Optimization error: {0}")]
    Optimization(String),

    #[error("Not supported: {0}")]
    NotSupported(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Invalid config: {0}")]
    InvalidConfig(String),
}

pub type CoreResult<T> = Result<T, CoreError>;
