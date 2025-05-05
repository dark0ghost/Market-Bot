use mistralrs::ChatCompletionResponse;

pub trait LLMProvider {
    async fn send_message(&self, text: String) -> anyhow::Result<ChatCompletionResponse>;
}