use anyhow::Result;
use std::collections::HashMap;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::path::Path;
use std::sync::{Arc, Mutex};
use tokenizers::Tokenizer;

use super::session::OrtSessionPool;

const LABELS: [&str; 3] = ["negative", "neutral", "positive"];
const MAX_CACHE_SIZE: usize = 512;

pub struct FinBertInference {
    session: Arc<OrtSessionPool>,
    tokenizer: Tokenizer,
    max_length: usize,
    cache: Mutex<HashMap<u64, NlpResult>>,
}

impl FinBertInference {
    pub fn new(model_dir: &str) -> Result<Self> {
        let onnx_path = Path::new(model_dir).join("model.onnx");
        let tokenizer_path = Path::new(model_dir).join("tokenizer.json");

        let num_threads = std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(4);

        let onnx_str = onnx_path
            .to_str()
            .ok_or_else(|| anyhow::anyhow!("Model path contains invalid UTF-8: {:?}", onnx_path))?;

        let session = Arc::new(OrtSessionPool::new(onnx_str, num_threads)?);

        let tokenizer_str = tokenizer_path.to_str().ok_or_else(|| {
            anyhow::anyhow!(
                "Tokenizer path contains invalid UTF-8: {:?}",
                tokenizer_path
            )
        })?;

        let tokenizer = Tokenizer::from_file(tokenizer_str)
            .map_err(|e| anyhow::anyhow!("Tokenizer load failed: {e}"))?;

        Ok(Self {
            session,
            tokenizer,
            max_length: 128,
            cache: Mutex::new(HashMap::new()),
        })
    }

    pub fn enable_hot_reload(self: Arc<Self>, model_dir: &str) {
        let path = Path::new(model_dir).join("model.onnx");
        if let Some(path_str) = path.to_str() {
            let s = self.session.clone();
            s.spawn_watcher(path_str);
        } else {
            log::warn!(
                "Skipping hot-reload watcher: model path contains invalid UTF-8: {:?}",
                path
            );
        }
    }

    fn tokenize(&self, text: &str) -> Result<(Vec<i64>, Vec<i64>)> {
        let encoding = self
            .tokenizer
            .encode(text, true)
            .map_err(|e| anyhow::anyhow!("Tokenization failed: {e}"))?;

        let ids = encoding.get_ids();
        let mask = encoding.get_attention_mask();
        let len = ids.len().min(self.max_length);

        let mut input_ids = vec![0i64; self.max_length];
        let mut attention_mask = vec![0i64; self.max_length];

        for i in 0..len {
            input_ids[i] = ids[i] as i64;
            attention_mask[i] = mask[i] as i64;
        }

        Ok((input_ids, attention_mask))
    }

    pub fn predict(&self, text: &str) -> Result<NlpResult> {
        let hash = self.hash_text(text);
        if let Some(cached) = self.cache.lock().unwrap().get(&hash) {
            return Ok(cached.clone());
        }

        let (input_ids, attention_mask) = self.tokenize(text)?;
        let logits = self
            .session
            .run(input_ids, attention_mask, self.max_length)?;

        let scores = [logits[[0, 0]], logits[[0, 1]], logits[[0, 2]]];
        let (idx, confidence) = Self::softmax(&scores);
        let label = LABELS[idx].to_string();

        let result = NlpResult {
            label,
            confidence,
            scores,
        };

        let mut cache = self.cache.lock().unwrap();
        if cache.len() >= MAX_CACHE_SIZE
            && let Some(&oldest_key) = cache.keys().next()
        {
            cache.remove(&oldest_key);
        }
        cache.insert(hash, result.clone());

        Ok(result)
    }

    fn hash_text(&self, text: &str) -> u64 {
        let mut hasher = DefaultHasher::new();
        text.hash(&mut hasher);
        hasher.finish()
    }

    fn softmax(scores: &[f32; 3]) -> (usize, f32) {
        let max_val = scores.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
        let exps: Vec<f32> = scores.iter().map(|v| (v - max_val).exp()).collect();
        let sum: f32 = exps.iter().sum();
        let probs: Vec<f32> = exps.iter().map(|e| e / sum).collect();
        let idx = probs
            .iter()
            .cloned()
            .enumerate()
            .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(i, _)| i)
            .unwrap_or(1);
        (idx, probs[idx])
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct NlpResult {
    pub label: String,
    pub confidence: f32,
    pub scores: [f32; 3],
}

impl NlpResult {
    pub fn sentiment_score(&self) -> f32 {
        self.scores[2] - self.scores[0]
    }

    pub fn probability_positive(&self) -> f32 {
        self.scores[2]
    }

    pub fn probability_negative(&self) -> f32 {
        self.scores[0]
    }

    pub fn probability_neutral(&self) -> f32 {
        self.scores[1]
    }
}
