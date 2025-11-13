//! This is the main application that launches a Dioxus-based web UI
use dioxus::logger::tracing::{self, Level};
use dioxus::prelude::*;
use futures_util::StreamExt;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::process::{Child, Command};
use std::time::Duration;

const FAVICON: Asset = asset!("/assets/favicon.ico");
const MAIN_CSS: Asset = asset!("/assets/main.css");
const TAILWIND_CSS: Asset = asset!("/assets/tailwind.css");

// Define the server address
const SERVER_ADDR: &str = "http://localhost:42069";
const API_ENDPOINT: &str = "/v1/chat/completions";

// Structs for API communication (should match the server's)
#[derive(Serialize, Debug)]
struct ChatCompletionRequest {
    messages: Vec<ChatMessage>,
    stream: bool,
    max_tokens: usize,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct ChatMessage {
    role: String,
    content: String,
}

#[derive(Deserialize, Debug)]
struct ChatCompletionChunk {
    choices: Vec<ChunkChoice>,
}

#[derive(Deserialize, Debug)]
struct ChunkChoice {
    delta: ChatMessage,
}

fn main() {
    dioxus::logger::init(Level::INFO).expect("failed to init logger");
    dioxus::launch(App);
}

/// A resource that holds the server process child, ensuring it's terminated on drop.
struct ServerProcess(Child);

impl Drop for ServerProcess {
    fn drop(&mut self) {
        tracing::info!("Shutting down API server...");
        if let Err(e) = self.0.kill() {
            tracing::error!("Failed to kill server process: {}", e);
        }
    }
}

#[component]
fn App() -> Element {
    let mut prompt = use_signal(|| "Q: What is 2 + 2?\nA:".to_string());
    let mut output = use_signal(String::new);
    let mut is_generating = use_signal(|| false);

    // This resource manages the API server's lifecycle.
    let server_status = use_resource(move || async move {
        tracing::info!("Checking for API server...");
        if reqwest::get(SERVER_ADDR).await.is_ok() {
            tracing::info!("API server is already running.");
            return Ok::<_, anyhow::Error>(None);
        }

        tracing::info!("API server not found. Spawning a new one...");
        // This assumes `api-server` has been built in release mode.
        // A more robust solution might check for the binary or build it.
        let child = Command::new("../../target/release/api-server")
            .spawn()
            .map_err(|e| anyhow::anyhow!("Failed to spawn server: {}. Have you built it with 'just build-release -p api-server'?", e))?;

        // Wait a moment for the server to start up.
        tokio::time::sleep(Duration::from_secs(3)).await;

        // Final check to ensure it started.
        reqwest::get(SERVER_ADDR).await?;
        tracing::info!("API server started successfully.");

        Ok(Some(ServerProcess(child)))
    });

    match &*server_status.value().read() {
        Some(Ok(_)) => {
            rsx! {
                document::Link { rel: "icon", href: FAVICON }
                document::Link { rel: "stylesheet", href: MAIN_CSS }
                document::Link { rel: "stylesheet", href: TAILWIND_CSS }
                div {
                    class: "container",
                    h1 { "Qwen2-5 Model Demo (API)" }
                    p { "The release-optimized API server is running in the background." }
                    textarea {
                        value: prompt.read().clone(),
                        oninput: move |e| *prompt.write() = e.value(),
                        rows: 4,
                        cols: 60,
                        placeholder: "Enter your prompt here...",
                    }
                    button {
                        onclick: move |_| {
                            is_generating.set(true);
                            output.set("Generating...".to_string());
                            let prompt_val = prompt.read().clone();

                            spawn(async move {
                                let client = Client::new();
                                let req = ChatCompletionRequest {
                                    messages: vec![ChatMessage {
                                        role: "user".to_string(),
                                        content: prompt_val,
                                    }],
                                    stream: true,
                                    max_tokens: 1000,
                                };

                                let mut first_token = true;
                                match client.post(format!("{}{}", SERVER_ADDR, API_ENDPOINT)).json(&req).send().await {
                                    Ok(res) => {
                                        let mut stream = res.bytes_stream();
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
                                                                let token = &choice.delta.content;
                                                                if first_token {
                                                                    output.set(token.clone());
                                                                    first_token = false;
                                                                } else {
                                                                    output.with_mut(|out| out.push_str(token));
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                                Err(e) => {
                                                    output.set(format!("Error receiving stream: {}", e));
                                                    break;
                                                }
                                            }
                                        }
                                    }
                                    Err(e) => {
                                        output.set(format!("Failed to send request: {}", e));
                                    }
                                }
                                is_generating.set(false);
                            });
                        },
                        disabled: is_generating(),
                        "Generate"
                    }
                    div {
                        style: "white-space: pre-wrap; margin-top: 1em;",
                        "Output:"
                        br {}
                        "{output}"
                    }
                }
            }
        }
        Some(Err(e)) => {
            let error_message = e.to_string();
            rsx! {
                document::Link { rel: "icon", href: FAVICON }
                document::Link { rel: "stylesheet", href: MAIN_CSS }
                document::Link { rel: "stylesheet", href: TAILWIND_CSS }
                div {
                    class: "container",
                    h1 { "Error starting API server" }
                    p { "The following error occurred:" }
                    pre { "{error_message}" }
                }
            }
        }
        None => {
            rsx! {
                document::Link { rel: "icon", href: FAVICON }
                document::Link { rel: "stylesheet", href: MAIN_CSS }
                document::Link { rel: "stylesheet", href: TAILWIND_CSS }
                div {
                    class: "container",
                    h1 { "Starting API server..." }
                    p { "Please wait, this may take a moment." }
                }
            }
        }
    }
}

