
pub trait LLMProvider<T, E> {

    #[warn(async_fn_in_trait)]
    async fn send_message(self, text: String) -> Result<T, E>;
}