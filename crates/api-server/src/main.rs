//! An OpenAI-compatible API server for the Qwen 2.5B models using the candle-qwen2-5-core library.
use anyhow::Result;
use axum::{
    extract::{Json, State},
    http::StatusCode,
    response::{sse::Event, IntoResponse, Sse},
    routing::post,
    Router,
};
use candle_qwen2_5_core::{ModelArgs, Qwen2Model, Which as CoreWhich};
use clap::{Parser, ValueEnum};
use serde::{Deserialize, Serialize};
use std::{
    convert::Infallible,
    net::SocketAddr,
    sync::{Arc, Mutex},
};
use tokio::sync::mpsc;
use tokio_stream::{wrappers::ReceiverStream, StreamExt};
use tracing::info;
use uuid::Uuid;

#[derive(Clone, Debug, Copy, PartialEq, Eq, ValueEnum)]
enum Which {
    #[value(name = "0.5b")]
    W25_0_5b,
    #[value(name = "1.5b")]
    W25_1_5b,
    #[value(name = "3b")]
    W25_3b,
    #[value(name = "7b")]
    W25_7b,
}

impl From<Which> for CoreWhich {
    fn from(w: Which) -> Self {
        match w {
            Which::W25_0_5b => CoreWhich::W25_0_5b,
            Which::W25_1_5b => CoreWhich::W25_1_5b,
            Which::W25_3b => CoreWhich::W25_3b,
            Which::W25_7b => CoreWhich::W25_7b,
        }
    }
}

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// GGUF file to load.
    #[arg(long)]
    model: Option<String>,

    /// The tokenizer config in json format.
    #[arg(long)]
    tokenizer: Option<String>,

    /// The temperature used to generate samples.
    #[arg(long, default_value_t = 0.8)]
    temperature: f64,

    /// Nucleus sampling probability cutoff.
    #[arg(long)]
    top_p: Option<f64>,

    /// Only sample among the top K samples.
    #[arg(long)]
    top_k: Option<usize>,

    /// The seed to use when generating random samples.
    #[arg(long, default_value_t = 299792458)]
    seed: u64,

    /// Run on CPU rather than GPU.
    #[arg(long)]
    cpu: bool,

    /// Penalty for repeating tokens.
    #[arg(long, default_value_t = 1.1)]
    repeat_penalty: f32,

    /// Context size for repeat penalty.
    #[arg(long, default_value_t = 64)]
    repeat_last_n: usize,

    /// The model size to use.
    #[arg(long, default_value = "0.5b")]
    which: Which,

    /// Port to listen on.
    #[arg(long, default_value = "42069")]
    port: u16,

    /// Log level.
    #[arg(long, default_value = "info")]
    log_level: String,
}

// OpenAI-compatible request and response structures

#[derive(Deserialize, Debug)]
struct ChatCompletionRequest {
    messages: Vec<ChatMessage>,
    #[serde(default)]
    stream: bool,
    #[serde(default = "default_sample_len")]
    max_tokens: usize,
}

fn default_sample_len() -> usize {
    1000
}

#[derive(Deserialize, Debug, Serialize, Clone)]
struct ChatMessage {
    role: String,
    content: String,
}

#[derive(Serialize, Debug)]
struct ChatCompletionResponse {
    id: String,
    object: String,
    created: u64,
    model: String,
    choices: Vec<Choice>,
}

#[derive(Serialize, Debug)]
struct Choice {
    index: usize,
    message: ChatMessage,
    finish_reason: String,
}

#[derive(Serialize, Debug)]
struct ChatCompletionChunk {
    id: String,
    object: String,
    created: u64,
    model: String,
    choices: Vec<ChunkChoice>,
}

#[derive(Serialize, Debug, Clone)]
struct ChunkChoice {
    index: usize,
    delta: ChatMessage,
    finish_reason: Option<String>,
}

type AppState = Arc<Mutex<Qwen2Model>>;

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    let level = match args.log_level.to_lowercase().as_str() {
        "error" => tracing::Level::ERROR,
        "warn" => tracing::Level::WARN,
        "info" => tracing::Level::INFO,
        "debug" => tracing::Level::DEBUG,
        "trace" => tracing::Level::TRACE,
        _ => tracing::Level::INFO,
    };
    tracing_subscriber::fmt().with_max_level(level).init();

    info!("Starting server with args: {:?}", args);

    let model_args = ModelArgs {
        model: args.model,
        sample_len: 0, // This will be overridden by request
        tokenizer: args.tokenizer,
        temperature: args.temperature,
        top_p: args.top_p,
        top_k: args.top_k,
        seed: args.seed,
        tracing: false,
        split_prompt: false,
        cpu: args.cpu,
        repeat_penalty: args.repeat_penalty,
        repeat_last_n: args.repeat_last_n,
        which: args.which.into(),
    };

    info!("Loading model...");
    let model = Qwen2Model::new(&model_args).await?;
    let app_state = Arc::new(Mutex::new(model));
    info!("Model loaded successfully.");

    let app = Router::new()
        .route("/v1/chat/completions", post(chat_completions_handler))
        .with_state(app_state);

    let addr = SocketAddr::from(([0, 0, 0, 0], args.port));
    info!("Listening on http://{}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

async fn chat_completions_handler(
    State(state): State<AppState>,
    Json(payload): Json<ChatCompletionRequest>,
) -> impl IntoResponse {
    let prompt = payload
        .messages
        .last()
        .map(|m| m.content.clone())
        .unwrap_or_default();
    let sample_len = payload.max_tokens;
    let model_name = "qwen2.5-gguf"; // Or derive from args

    if payload.stream {
        let (tx, rx) = mpsc::channel::<Result<String, anyhow::Error>>(100);

        let stream = ReceiverStream::new(rx);

        tokio::task::spawn_blocking(move || {
            let mut model_guard = state.lock().unwrap();
            let res = model_guard.generate(&prompt, sample_len, |token| {
                if tx.blocking_send(Ok(token)).is_err() {
                    // If the receiver is dropped, stop generation.
                    return Err(anyhow::anyhow!("Client disconnected"));
                }
                Ok(())
            });

            if let Err(e) = res {
                let _ = tx.blocking_send(Err(e.into()));
            }
        });

        let sse_stream = stream.map(move |res| {
            let event = match res {
                Ok(token) => {
                    let chunk_id = format!("cmpl-{}", Uuid::new_v4());
                    let created = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_secs();
                    let chunk = ChatCompletionChunk {
                        id: chunk_id,
                        object: "chat.completion.chunk".to_string(),
                        created,
                        model: model_name.to_string(),
                        choices: vec![ChunkChoice {
                            index: 0,
                            delta: ChatMessage {
                                role: "assistant".to_string(),
                                content: token,
                            },
                            finish_reason: None,
                        }],
                    };
                    Event::default()
                        .json_data(chunk)
                        .unwrap_or_else(|_| Event::default().data("Error serializing chunk"))
                }
                Err(e) => Event::default().data(format!("[ERROR]: {}", e)),
            };
            Ok::<_, Infallible>(event)
        });

        let final_stream = sse_stream.chain(futures_util::stream::once(async {
            Ok(Event::default().data("[DONE]"))
        }));

        Sse::new(final_stream).into_response()
    } else {
        let model_clone = Arc::clone(&state);
        let generation_task = tokio::task::spawn_blocking(move || {
            let mut model_guard = model_clone.lock().unwrap();
            let mut full_response = String::new();
            let result = model_guard.generate(&prompt, sample_len, |token| {
                full_response.push_str(&token);
                Ok(())
            });
            (full_response, result)
        });

        let (full_response, result) = generation_task.await.unwrap();

        if let Err(e) = result {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": e.to_string()})),
            )
                .into_response();
        }

        let response = ChatCompletionResponse {
            id: format!("cmpl-{}", Uuid::new_v4()),
            object: "chat.completion".to_string(),
            created: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            model: model_name.to_string(),
            choices: vec![Choice {
                index: 0,
                message: ChatMessage {
                    role: "assistant".to_string(),
                    content: full_response,
                },
                finish_reason: "stop".to_string(),
            }],
        };

        (StatusCode::OK, Json(response)).into_response()
    }
}

