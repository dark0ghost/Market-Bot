use anyhow::Result;
use std::sync::Arc;
use tokio_cron_scheduler::{Job, JobScheduler};

pub struct CronScheduler {
    scheduler: JobScheduler,
    tasks: Vec<String>,
}

impl CronScheduler {
    pub async fn new() -> Result<Self> {
        let scheduler = JobScheduler::new().await?;
        Ok(CronScheduler {
            scheduler,
            tasks: Vec::new(),
        })
    }

    pub async fn add_job<F, Fut>(&mut self, cron_expr: &str, name: &str, task: F) -> Result<()>
    where
        F: Fn() -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = Result<()>> + Send,
    {
        let task = Arc::new(task);
        let cron_expr = cron_expr.to_string();
        let job = Job::new_async(cron_expr, move |_uuid, _lock| {
            let task = task.clone();
            Box::pin(async move {
                if let Err(e) = task().await {
                    log::error!("Cron task failed: {}", e);
                }
            })
        })?;
        self.scheduler.add(job).await?;
        self.tasks.push(name.to_string());
        Ok(())
    }

    pub async fn add_interval_job<F, Fut>(
        &mut self,
        interval_secs: u64,
        name: &str,
        task: F,
    ) -> Result<()>
    where
        F: Fn() -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = Result<()>> + Send,
    {
        let task = Arc::new(task);
        let cron_expr = format!("*/{} * * * * *", interval_secs);
        let name_owned = name.to_string();
        self.tasks.push(name_owned.clone());
        let job = Job::new_async(cron_expr, move |_uuid, _lock| {
            let task = task.clone();
            let name = name_owned.clone();
            Box::pin(async move {
                if let Err(e) = task().await {
                    log::error!("Interval task '{}' failed: {}", name, e);
                }
            })
        })?;
        self.scheduler.add(job).await?;
        Ok(())
    }

    pub async fn start(&self) -> Result<()> {
        self.scheduler.start().await?;
        Ok(())
    }

    pub const fn task_count(&self) -> usize {
        self.tasks.len()
    }
}
