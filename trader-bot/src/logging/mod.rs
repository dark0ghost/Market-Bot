use anyhow::Result;
use log::Level;
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::sync::Mutex;

// ─── Trait ────────────────────────────────────────────────────────────

/// Polymorphic log destination
pub trait LogDestination: Send + Sync {
    fn name(&self) -> &str;
    fn write(&self, level: Level, target: &str, message: &str);
}

// ─── Console ──────────────────────────────────────────────────────────

/// Logs to stdout with coloured levels
pub struct ConsoleDestination;

impl LogDestination for ConsoleDestination {
    fn name(&self) -> &str {
        "console"
    }

    fn write(&self, level: Level, target: &str, message: &str) {
        let now = chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f");
        let level_str = match level {
            Level::Error => "ERROR",
            Level::Warn => " WARN",
            Level::Info => " INFO",
            Level::Debug => "DEBUG",
            Level::Trace => "TRACE",
        };
        println!("{} {} [{}] {}", now, level_str, target, message);
    }
}

// ─── File ─────────────────────────────────────────────────────────────

/// Logs to a file (append mode, auto-create)
pub struct FileDestination {
    file: Mutex<File>,
    path: String,
}

impl FileDestination {
    pub fn new(path: &str) -> Result<Self> {
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)?;
        Ok(FileDestination {
            file: Mutex::new(file),
            path: path.to_string(),
        })
    }
}

impl LogDestination for FileDestination {
    fn name(&self) -> &str {
        &self.path
    }

    fn write(&self, level: Level, target: &str, message: &str) {
        let now = chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f");
        if let Ok(mut f) = self.file.lock() {
            let _ = writeln!(f, "{} {:5} [{}] {}", now, level, target, message);
        }
    }
}

// ─── Network (HTTP) ───────────────────────────────────────────────────

/// Sends logs above a threshold via HTTP POST (JSON body)
pub struct NetworkDestination {
    url: String,
    client: reqwest::Client,
    level_threshold: Level,
}

impl NetworkDestination {
    pub fn new(url: &str, level_threshold: Level) -> Self {
        NetworkDestination {
            url: url.to_string(),
            client: reqwest::Client::new(),
            level_threshold,
        }
    }
}

impl LogDestination for NetworkDestination {
    fn name(&self) -> &str {
        &self.url
    }

    fn write(&self, level: Level, target: &str, message: &str) {
        if level < self.level_threshold {
            return;
        }
        let body = serde_json::json!({
            "level": format!("{}", level),
            "target": target,
            "message": message,
            "timestamp": chrono::Utc::now().to_rfc3339(),
        });
        let url = self.url.clone();
        let client = self.client.clone();
        tokio::spawn(async move {
            let _ = client.post(&url).json(&body).send().await;
        });
    }
}

// ─── Router ───────────────────────────────────────────────────────────

/// Dispatches each log record to all registered destinations
pub struct LogRouter {
    destinations: Vec<Box<dyn LogDestination>>,
}

impl LogRouter {
    pub fn new() -> Self {
        LogRouter {
            destinations: Vec::new(),
        }
    }

    pub fn add(&mut self, destination: Box<dyn LogDestination>) {
        let name = destination.name().to_string();
        log::info!("Log destination added: {}", name);
        self.destinations.push(destination);
    }

    pub fn remove(&mut self, name: &str) {
        self.destinations.retain(|d| d.name() != name);
    }

    pub fn count(&self) -> usize {
        self.destinations.len()
    }
}

impl log::Log for LogRouter {
    fn enabled(&self, _metadata: &log::Metadata) -> bool {
        true
    }

    fn log(&self, record: &log::Record) {
        for dest in &self.destinations {
            dest.write(record.level(), record.target(), &record.args().to_string());
        }
    }

    fn flush(&self) {}
}

// ─── Builder ──────────────────────────────────────────────────────────

/// Convenience builder for setting up the global logger
pub struct LoggerBuilder {
    router: LogRouter,
}

impl LoggerBuilder {
    pub fn new() -> Self {
        LoggerBuilder {
            router: LogRouter::new(),
        }
    }

    pub fn console(mut self) -> Self {
        self.router.add(Box::new(ConsoleDestination));
        self
    }

    pub fn file(mut self, path: &str) -> Result<Self> {
        let dest = FileDestination::new(path)?;
        self.router.add(Box::new(dest));
        Ok(self)
    }

    pub fn network(mut self, url: &str, threshold: Level) -> Self {
        self.router.add(Box::new(NetworkDestination::new(url, threshold)));
        self
    }

    /// Register as the global logger (panics if already set)
    pub fn init(self) -> Result<()> {
        let max_level = log::LevelFilter::Debug;
        log::set_boxed_logger(Box::new(self.router))
            .map(|()| log::set_max_level(max_level))?;
        Ok(())
    }
}

impl Default for LoggerBuilder {
    fn default() -> Self {
        Self::new()
    }
}
