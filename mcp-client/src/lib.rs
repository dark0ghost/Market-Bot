pub mod llm_provider;

pub mod ollama;
pub mod perplexica;

pub use perplexica::{
    FocusMode, ModelConfig, OptimizationMode, PerplexicaProvider, PerplexicaSearcher, SearchSource,
};
