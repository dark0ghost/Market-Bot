use ollama_rs::error::OllamaError;
use ollama_rs::generation::chat::{ChatMessage, ChatMessageResponse};
use ollama_rs::generation::chat::request::ChatMessageRequest;
use ollama_rs::models::LocalModel;
use ollama_rs::Ollama;
use crate::llm_provider::LLMProvider;


pub type LlmError = OllamaError;
pub type LlmMessage = ChatMessageResponse;


pub struct OllamaProvider {
    ollama: Ollama,
    model: String,
}

impl OllamaProvider {
    fn new(model: String, host: String, port: u16) -> Self {
        let ollama = Ollama::new(host, port);
        OllamaProvider {
            ollama,
            model,
        }
    }

    async fn get_local_model(self) -> Result<Vec<LocalModel>, OllamaError> {
        self.ollama.list_local_models().await
    }
}

impl Default for OllamaProvider {
    fn default() -> Self {
        OllamaProvider {
            ollama: Ollama::default(),
            model: "qwen3:1.7b".to_string(),
        }
    }
}

impl LLMProvider<LlmMessage, LlmError> for OllamaProvider {
    async fn send_message(self, text: String) -> Result<ChatMessageResponse, OllamaError> {
        let user_message = ChatMessage::user(text.to_owned());
        self.ollama.send_chat_messages(ChatMessageRequest::new(self.model, vec![user_message])).await
    }
}

mod test {
    use crate::ollama::OllamaProvider;

    #[tokio::test]
    async fn test_default() {
        let ollama = OllamaProvider::default();
        let models = ollama.get_local_model().await.unwrap();

        assert_ne!(models.len(), 0);
    }
}