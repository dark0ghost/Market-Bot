use mcp_client::llm_provider::LLMProvider;
use mcp_client::ollama::{LlmError, LlmMessage, OllamaProvider};

pub fn get_llm_provider() -> impl LLMProvider<LlmMessage, LlmError> {
    let ollama = OllamaProvider::default();

    ollama
}