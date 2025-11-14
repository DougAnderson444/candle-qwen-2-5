use dioxus::logger::tracing;
use futures_util::StreamExt;
use reqwest::Client;
use serde::{Deserialize, Serialize};

// API constants
pub const PORT: u16 = 42070;
const SERVER_ADDR: &str = "http://localhost";
const API_ENDPOINT: &str = "/v1/chat/completions";

// Structs for API communication
#[derive(Serialize, Debug)]
pub struct ChatCompletionRequest {
    pub messages: Vec<ChatMessage>,
    pub stream: bool,
    pub max_tokens: usize,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

#[derive(Deserialize, Debug)]
struct ChatCompletionChunk {
    choices: Vec<ChunkChoice>,
}

#[derive(Deserialize, Debug)]
struct ChunkChoice {
    delta: ChatMessage,
}

#[derive(Clone)]
pub struct ApiClient {
    client: Client,
    base_url: String,
}

impl ApiClient {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
            base_url: format!("{}:{}", SERVER_ADDR, PORT),
        }
    }

    pub async fn generate_stream(
        &self,
        prompt: String,
        mut on_token: impl FnMut(String),
    ) -> Result<(), reqwest::Error> {
        let req = ChatCompletionRequest {
            messages: vec![ChatMessage {
                role: "user".to_string(),
                content: prompt,
            }],
            stream: true,
            max_tokens: 1000,
        };

        let mut stream = self
            .client
            .post(format!("{}{}", self.base_url, API_ENDPOINT))
            .json(&req)
            .send()
            .await?
            .bytes_stream();

        while let Some(item) = stream.next().await {
            match item {
                Ok(bytes) => {
                    let s = String::from_utf8_lossy(&bytes);
                    for line in s.lines().filter(|l| l.starts_with("data:")) {
                        let data = &line["data: ".len()..];
                        if data.trim() == "[DONE]" {
                            break;
                        }
                        if let Ok(chunk) = serde_json::from_str::<ChatCompletionChunk>(data) {
                            if let Some(choice) = chunk.choices.first() {
                                on_token(choice.delta.content.clone());
                            }
                        }
                    }
                }
                Err(e) => {
                    // Log or handle the error
                    tracing::error!("Error in stream: {}", e);
                    break;
                }
            }
        }
        Ok(())
    }
}
