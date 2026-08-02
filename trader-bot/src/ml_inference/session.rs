use anyhow::{Context, Result};
use ort::session::Session;
use ort::session::builder::{GraphOptimizationLevel, SessionBuilder};
use ort::value::Tensor;
use std::path::Path;
use std::sync::{Arc, Mutex};

pub struct OrtSessionPool {
    session: Arc<Mutex<Session>>,
    model_path: String,
    num_threads: usize,
}

impl OrtSessionPool {
    pub fn new(model_path: &str, num_threads: usize) -> Result<Self> {
        let session = Self::build_session(model_path, num_threads)?;
        Ok(Self {
            session: Arc::new(Mutex::new(session)),
            model_path: model_path.to_string(),
            num_threads,
        })
    }

    fn build_session(model_path: &str, num_threads: usize) -> Result<Session> {
        SessionBuilder::new()
            .context("ORT init failed")?
            .with_optimization_level(GraphOptimizationLevel::Level3)
            .map_err(|e| anyhow::anyhow!("ORT opt level: {e}"))?
            .with_intra_threads(num_threads)
            .map_err(|e| anyhow::anyhow!("ORT threads: {e}"))?
            .commit_from_file(model_path)
            .context("ORT model load failed")
    }

    pub fn reload(&self) -> Result<()> {
        let new = Self::build_session(&self.model_path, self.num_threads)?;
        // Build new session first, then swap atomically.
        // If build_session fails, the old session is preserved.
        let mut session = self.session.lock().unwrap();
        *session = new;
        Ok(())
    }

    pub fn run(
        &self,
        input_ids: Vec<i64>,
        attention_mask: Vec<i64>,
        seq_len: usize,
    ) -> Result<ndarray::Array2<f32>> {
        let shape = vec![1i64, seq_len as i64];
        let in_ids = Tensor::from_array((shape.clone(), input_ids))
            .context("ORT create input_ids tensor")?;
        let attn = Tensor::from_array((shape, attention_mask))
            .context("ORT create attention_mask tensor")?;

        let mut session = self.session.lock().unwrap();
        let outputs = session
            .run(ort::inputs! {
                "input_ids" => in_ids,
                "attention_mask" => attn,
            })
            .context("ORT inference failed")?;

        let (logits_shape, logits_data) = outputs["logits"]
            .try_extract_tensor::<f32>()
            .context("ORT extract logits failed")?;

        let dims: Vec<usize> = logits_shape.iter().map(|d| *d as usize).collect();
        let logits = ndarray::Array2::from_shape_vec((dims[0], dims[1]), logits_data.to_vec())?;

        Ok(logits)
    }

    pub fn spawn_watcher(self: Arc<Self>, path: &str) {
        use notify::{Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
        let path = path.to_string();
        std::thread::spawn(move || {
            let (tx, rx) = std::sync::mpsc::channel();
            let mut watcher = match RecommendedWatcher::new(tx, Config::default()) {
                Ok(w) => w,
                Err(e) => {
                    log::error!("Failed to create file watcher: {e}");
                    return;
                }
            };
            if watcher
                .watch(Path::new(&path), RecursiveMode::NonRecursive)
                .is_err()
            {
                log::error!("Failed to watch model file: {path}");
                return;
            }
            for event in rx {
                if let Ok(Event {
                    kind: EventKind::Modify(_),
                    ..
                }) = event
                {
                    log::info!("Model file changed, reloading: {path}");
                    if let Err(e) = self.reload() {
                        log::error!("Model reload failed: {e}");
                    }
                }
            }
        });
    }
}
