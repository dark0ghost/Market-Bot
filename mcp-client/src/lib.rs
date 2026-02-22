pub mod llm_provider;

pub mod ollama;
pub mod perplexica;

pub use perplexica::{ModelConfig, PerplexicaProvider, PerplexicaSearcher, SearchSource, OptimizationMode, FocusMode};