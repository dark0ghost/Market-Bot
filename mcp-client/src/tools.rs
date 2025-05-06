/// Stock global market share data.
///
/// * stocks - The city for which to get the weather.
#[ollama_rs::function]
async fn get_stocks_data(stocks: String) -> Result<String, Box<dyn std::error::Error + Sync + Send>> {
    let url = format!("https://wttr.in/{stocks}?format=%C+%t");
    let response = reqwest::get(&url).await?.text().await?;
    Ok(response)
}


mod test {
    use ollama_rs::coordinator::Coordinator;
    use ollama_rs::generation::chat::ChatMessage;
    use ollama_rs::Ollama;
    use crate::tools;

    #[tokio::test]
    async fn test_global_market_not_crypto() {
        let ollama = Ollama::default();
        let history = vec![];
        let mut coordinator = Coordinator::new(ollama, "qwen3:1.7b".to_string(), history)
            .add_tool(tools::get_stocks_data);

        let user_message = ChatMessage::user("What's carrency ETH?".into());
        let resp = coordinator.chat(vec![user_message]).await.unwrap();

        const EXCEPTED: &str = "The function's parameters require a city, not a cryptocurrency. ";

        assert!(resp.message.content.contains(EXCEPTED))
    }
}