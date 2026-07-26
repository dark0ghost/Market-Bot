use anyhow::Result;
use std::path::Path;
use std::sync::Arc;
use tokenizers::Tokenizer;

use super::session::OrtSessionPool;

const LABELS: [&str; 3] = ["negative", "neutral", "positive"];

pub struct FinBertInference {
    session: Arc<OrtSessionPool>,
    tokenizer: Tokenizer,
    max_length: usize,
}

impl FinBertInference {
    pub fn new(model_dir: &str) -> Result<Self> {
        let onnx_path = Path::new(model_dir).join("model.onnx");
        let tokenizer_path = Path::new(model_dir).join("tokenizer.json");

        let num_threads = std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(4);

        let session = Arc::new(OrtSessionPool::new(
            onnx_path.to_str().unwrap(),
            num_threads,
        )?);

        let tokenizer = Tokenizer::from_file(tokenizer_path.to_str().unwrap())
            .map_err(|e| anyhow::anyhow!("Tokenizer load failed: {e}"))?;

        Ok(Self {
            session,
            tokenizer,
            max_length: 128,
        })
    }

    pub fn enable_hot_reload(self: Arc<Self>, model_dir: &str) {
        let path = Path::new(model_dir).join("model.onnx");
        let s = self.session.clone();
        s.spawn_watcher(path.to_str().unwrap());
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
        let (input_ids, attention_mask) = self.tokenize(text)?;
        let logits = self
            .session
            .run(input_ids, attention_mask, self.max_length)?;

        let scores = [logits[[0, 0]], logits[[0, 1]], logits[[0, 2]]];
        let (idx, confidence) = Self::softmax(&scores);
        let label = LABELS[idx].to_string();

        Ok(NlpResult {
            label,
            confidence,
            scores,
        })
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
            .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
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
        match self.label.as_str() {
            "positive" => self.confidence,
            "negative" => -self.confidence,
            _ => 0.0,
        }
    }
}
