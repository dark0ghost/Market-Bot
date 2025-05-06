// https://github.com/pepperoni21/ollama-rs?tab=readme-ov-file#usage
use ollama_rs::{
    generation::completion::{
        GenerationContext, GenerationResponseStream, request::GenerationRequest,
    },
    Ollama,
};
use ollama_rs::coordinator::Coordinator;
use ollama_rs::generation::chat::ChatMessage;
use tokio::io::{AsyncWriteExt, stdout};
use tokio_stream::StreamExt;
use std::path::PathBuf;
use anyhow::Result;


mod llm_provider;
mod ollama;
mod tools;


#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Sync + Send>> {
    let ollama = Ollama::default();
    let history = vec![];
    let mut coordinator = Coordinator::new(ollama, "qwen3:1.7b".to_string(), history);

    let user_messages = vec![
        "What's the weather in Berlin?",
    ];

    for user_message in user_messages {
        println!("User: {user_message}");

        let user_message = ChatMessage::user(user_message.to_owned());
        let resp = coordinator.chat(vec![user_message]).await?;
        println!("Assistant: {}", resp.message.content);
    }

    Ok(())
}
