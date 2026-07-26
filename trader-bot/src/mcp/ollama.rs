use std::sync::Arc;

use ollama_rs::Ollama;
use ollama_rs::coordinator::Coordinator;
use ollama_rs::error::OllamaError;
use ollama_rs::generation::chat::{ChatMessage, ChatMessageResponse};
use ollama_rs::generation::parameters::FormatType;
use tokio::sync::Mutex;

use crate::mcp::llm_provider::LlmProvider;

pub type LlmError = OllamaError;
pub type LlmMessage = ChatMessageResponse;

pub struct OllamaProvider {
    model: String,
    host: String,
    port: u16,
    coordinator: Arc<Mutex<Coordinator<Vec<ChatMessage>>>>,
}

impl Clone for OllamaProvider {
    fn clone(&self) -> Self {
        let ollama = Ollama::new(self.host.clone(), self.port);
        let coordinator = Arc::new(Mutex::new(
            Coordinator::new(ollama, self.model.clone(), vec![])
                .format(FormatType::Json),
        ));
        OllamaProvider {
            model: self.model.clone(),
            host: self.host.clone(),
            port: self.port,
            coordinator,
        }
    }
}

impl OllamaProvider {
    pub fn new(model: String, host: String, port: u16) -> Self {
        let ollama = Ollama::new(host.clone(), port);
        let coordinator = Arc::new(Mutex::new(
            Coordinator::new(ollama, model.clone(), vec![])
                .format(FormatType::Json),
        ));
        OllamaProvider {
            model,
            host,
            port,
            coordinator,
        }
    }

    #[cfg(test)]
    fn add_tool<T: ollama_rs::generation::tools::Tool + 'static>(&mut self, tools: Vec<T>) {
        let ollama = Ollama::new(self.host.clone(), self.port);
        let mut coordinator = Coordinator::new(ollama, self.model.clone(), vec![]);
        for tool in tools {
            coordinator = coordinator.add_tool(tool)
        }
        let cord = self.coordinator.clone();
        tokio::spawn(async move {
            let mut guard = cord.lock().await;
            *guard = coordinator;
        });
    }

    #[cfg(test)]
    async fn get_local_model(self) -> Result<Vec<ollama_rs::models::LocalModel>, OllamaError> {
        let ollama = Ollama::new(self.host.clone(), self.port);
        ollama.list_local_models().await
    }
}

impl Default for OllamaProvider {
    fn default() -> Self {
        let model = std::env::var("OLLAMA_MODEL_NAME").unwrap_or_else(|_| "fin-expert".to_string());
        let host = "http://localhost".to_string();
        let port = 11434u16;
        Self::new(model, host, port)
    }
}

impl LlmProvider<LlmMessage, LlmError> for OllamaProvider {
    async fn send_message(&self, text: String) -> Result<ChatMessageResponse, OllamaError> {
        let user_message = ChatMessage::user(text);
        let mut coordinator = self.coordinator.lock().await;
        coordinator.chat(vec![user_message]).await
    }
}

#[cfg(test)]
mod test {
    use crate::mcp::llm_provider::LlmProvider;
    use crate::mcp::ollama::OllamaProvider;

    const CPU_TEMPERATURE: &str = "32";

    #[ignore]
    #[tokio::test]
    async fn test_default() {
        let ollama = OllamaProvider::default();
        let models = ollama.get_local_model().await.unwrap();
        assert_ne!(models.len(), 0);
    }

    #[ignore]
    #[tokio::test]
    async fn test_working_add_tool() {
        let mut ollama = OllamaProvider::default();
        ollama.add_tool(vec![get_cpu_temperature]);
        let response = ollama
            .send_message("What's the CPU temperature?".to_owned())
            .await
            .expect("Ollama not working");
        assert!(response.message.content.contains(CPU_TEMPERATURE))
    }

    /// Get the CPU temperature in Celsius.
    #[ollama_rs::function]
    async fn get_cpu_temperature() -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        Ok(CPU_TEMPERATURE.to_string())
    }
}
