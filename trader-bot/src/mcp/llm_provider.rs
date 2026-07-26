#[allow(async_fn_in_trait)]
pub trait LlmProvider<T, E> {
    async fn send_message(&self, text: String) -> Result<T, E>;
}
