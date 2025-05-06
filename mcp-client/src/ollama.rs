use ollama_rs::Ollama;
use crate::llm_provider::LLMProvider;

struct OllamaProvider {
    ollama: Ollama
}

impl OllamaProvider {
    fn new(model: String) -> Self {
        let ollama = Ollama::new("http://localhost".to_string(), 11434);
        OllamaProvider {
             ollama
        }
    }
}

impl Default for OllamaProvider {
    fn default() -> Self {
        OllamaProvider {
            ollama: Ollama::default()
        }
    }
}

impl LLMProvider for OllamaProvider {
    async fn send_message(&self, text: String) -> anyhow::Result<()> {
             Ok(())
    }
}