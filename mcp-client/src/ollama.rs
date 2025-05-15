use std::cell::RefCell;

use ollama_rs::coordinator::Coordinator;
use ollama_rs::error::OllamaError;
use ollama_rs::generation::chat::{ChatMessage, ChatMessageResponse};
use ollama_rs::generation::tools::Tool;
use ollama_rs::models::LocalModel;
use ollama_rs::Ollama;

use crate::llm_provider::LLMProvider;

pub type LlmError = OllamaError;
pub type LlmMessage = ChatMessageResponse;


pub struct OllamaProvider {
    ollama: Ollama,
    model: String,
    coordinator: RefCell<Coordinator<Vec<ChatMessage>>>,
}

impl OllamaProvider {
    fn new(model: String, host: String, port: u16) -> Self {
        let ollama = Ollama::new(host, port);
        let coordinator = RefCell::new(Coordinator::new(
            ollama.clone(),
            model.clone(),
            vec![],
        ));

        OllamaProvider {
            ollama,
            model,
            coordinator,
        }
    }

    fn add_tool<T: Tool + 'static>(&mut self, tools: Vec<T>) {
        let mut coordinator = Coordinator::new(
            self.ollama.clone(),
            self.model.clone(),
            vec![],
        );
        for tool in tools {
            coordinator = coordinator.add_tool(tool)
        }

        let cord = self.coordinator.get_mut();
        *cord = coordinator
    }

    async fn get_local_model(self) -> Result<Vec<LocalModel>, OllamaError> {
        self.ollama.list_local_models().await
    }
}

impl Default for OllamaProvider {
    fn default() -> Self {
        let ollama = Ollama::default();
        let model = "qwen3:1.7b".to_string();
        let coordinator = Coordinator::new(
            ollama.clone(),
            model.clone(),
            vec![],
        );
        OllamaProvider {
            ollama,
            model,
            coordinator: RefCell::new(coordinator),
        }
    }
}

impl LLMProvider<LlmMessage, LlmError> for OllamaProvider {
    async fn send_message(self, text: String) -> Result<ChatMessageResponse, OllamaError> {
        let user_message = ChatMessage::user(text.to_owned());
        self
            .coordinator
            .borrow_mut()
            .chat(vec![user_message])
            .await
    }
}

mod test {
    use std::fmt::format;
    use crate::llm_provider::LLMProvider;
    use crate::ollama::OllamaProvider;

    const CPU_TEMPERATURE: &str = "32";

    #[tokio::test]
    async fn test_default() {
        let ollama = OllamaProvider::default();
        let models = ollama.get_local_model().await.unwrap();

        assert_ne!(models.len(), 0);
    }

    #[tokio::test]
    async fn test_working_add_tool() {
        let mut ollama = OllamaProvider::default();

        ollama.add_tool(vec![get_cpu_temperature]);
        let response = ollama
            .send_message("What's the CPU temperature?".to_owned())
            .await
            .expect("Ollama not working");

        assert!(response.message.content.contains(&CPU_TEMPERATURE.to_string()))
    }

    /// Get the CPU temperature in Celsius.
    #[ollama_rs::function]
    async fn get_cpu_temperature() -> Result<&str, Box<dyn std::error::Error + Send + Sync>> {
        Ok(CPU_TEMPERATURE.to_string())
    }
}