use crate::core::OrderAction;
use crate::execution::position_manager::OrderResult;
use crate::provider::ExecutionProvider;
use anyhow::Result;
use chrono::Utc;

pub struct TwapExecutor<E: ExecutionProvider> {
    executor: E,
    total_quantity: i32,
    num_slices: u32,
    interval_secs: u64,
    direction: OrderAction,
    figi: String,
}

impl<E: ExecutionProvider> TwapExecutor<E> {
    pub fn new(
        executor: E,
        figi: String,
        total_quantity: i32,
        num_slices: u32,
        duration_minutes: u64,
        direction: OrderAction,
    ) -> Self {
        let interval_secs = (duration_minutes * 60) / num_slices as u64;
        TwapExecutor {
            executor,
            total_quantity,
            num_slices,
            interval_secs,
            direction,
            figi,
        }
    }

    pub async fn execute(&self) -> Result<Vec<OrderResult>> {
        let slice_size = self.total_quantity / self.num_slices as i32;
        let mut results = Vec::new();

        for i in 0..self.num_slices {
            let adjusted = if i == self.num_slices - 1 {
                self.total_quantity - slice_size * (self.num_slices as i32 - 1)
            } else {
                slice_size
            };

            if adjusted <= 0 {
                continue;
            }

            let start = Utc::now();
            let result = self
                .executor
                .place_market_order(&self.figi, self.direction.clone(), adjusted)
                .await?;
            results.push(result);

            let elapsed = Utc::now() - start;
            if elapsed.num_seconds() < self.interval_secs as i64 {
                tokio::time::sleep(
                    std::time::Duration::from_secs(self.interval_secs) - elapsed.to_std()?,
                )
                .await;
            }
        }

        Ok(results)
    }
}

pub struct VwapExecutor<E: ExecutionProvider> {
    executor: E,
    total_quantity: i32,
    figi: String,
    direction: OrderAction,
}

impl<E: ExecutionProvider> VwapExecutor<E> {
    pub fn new(executor: E, figi: String, total_quantity: i32, direction: OrderAction) -> Self {
        VwapExecutor {
            executor,
            total_quantity,
            figi,
            direction,
        }
    }

    pub async fn execute(&self, price_schedule: &[f64]) -> Result<Vec<OrderResult>> {
        let slice_size = self.total_quantity / price_schedule.len() as i32;
        let mut results = Vec::new();

        for (i, &price) in price_schedule.iter().enumerate() {
            let adjusted = if i == price_schedule.len() - 1 {
                self.total_quantity - slice_size * (price_schedule.len() as i32 - 1)
            } else {
                slice_size
            };

            if adjusted <= 0 {
                continue;
            }

            let result = self
                .executor
                .place_limit_order(&self.figi, self.direction.clone(), adjusted, price)
                .await?;
            results.push(result);

            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        }

        Ok(results)
    }
}
