use candle_qwen2_5_core::{ModelArgs, Qwen2Model, Which};
use dioxus::logger::tracing::{debug, error, info, warn, Level};
use dioxus::prelude::*;

use std::sync::{Arc, Mutex};
use std::thread;

const FAVICON: Asset = asset!("/assets/favicon.ico");
const MAIN_CSS: Asset = asset!("/assets/main.css");
const TAILWIND_CSS: Asset = asset!("/assets/tailwind.css");

fn main() {
    dioxus::logger::init(Level::INFO).expect("failed to init logger");
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    let mut prompt = use_signal(|| "Q: What is 2 + 2?\nA:".to_string());
    let mut output = use_signal(String::new);

    // Use `use_resource` to load the model once when the component mounts.
    let model_resource = use_resource(move || async move {
        let model_args = ModelArgs {
            model: None,
            sample_len: 1000,
            tokenizer: None,
            temperature: 0.8,
            top_p: None,
            top_k: None,
            seed: 299792458,
            tracing: false,
            split_prompt: false,
            cpu: false,
            repeat_penalty: 1.1,
            repeat_last_n: 64,
            which: Which::W25_3b,
        };
        // Directly await the new async `new` function.
        let model = Qwen2Model::new(&model_args).await?;
        // Wrap the model in Arc<Mutex<>> for thread-safe sharing and interior mutability.
        Ok::<_, anyhow::Error>(Arc::new(Mutex::new(model)))
    });

    // Render the UI based on the state of the model resource.
    match &*model_resource.value().read() {
        // State: Model is loaded and ready.
        Some(Ok(model)) => {
            let model = model.clone();
            let mut is_generating = use_signal(|| false);

            rsx! {
                document::Link { rel: "icon", href: FAVICON }
                document::Link { rel: "stylesheet", href: MAIN_CSS }
                document::Link { rel: "stylesheet", href: TAILWIND_CSS }
                div {
                    class: "container",
                    h1 { "Qwen2-5 Model Demo" }
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
                            let model_clone = model.clone();

                            let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<String>();

                            // Spawn a Dioxus task to receive tokens and update the UI.
                            spawn(async move {
                                let mut first = true;
                                while let Some(token) = rx.recv().await {
                                    if first {
                                        output.set(token);
                                        first = false;
                                    } else {
                                        output.with_mut(|out| out.push_str(&token));
                                    }
                                }
                                is_generating.set(false);
                            });

                            // Spawn a thread for the blocking generation task.
                            // The `generate` method itself is still synchronous.
                            thread::spawn(move || {
                                // Lock the mutex to get mutable access to the model.
                                let mut model_guard = model_clone.lock().unwrap();
                                if let Err(e) = model_guard.generate(&prompt_val, 1000, |token| {
                                    let _ = tx.send(token.to_string());
                                    Ok(())
                                }) {
                                    let _ = tx.send(format!("Generation error: {e}"));
                                }
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
        // State: An error occurred while loading the model.
        Some(Err(e)) => {
            let error_message = e.to_string();
            rsx! {
                document::Link { rel: "icon", href: FAVICON }
                document::Link { rel: "stylesheet", href: MAIN_CSS }
                document::Link { rel: "stylesheet", href: TAILWIND_CSS }
                div {
                    class: "container",
                    h1 { "Error loading model" }
                    p { "The following error occurred:" }
                    pre { "{error_message}" }
                }
            }
        }
        // State: Model is still loading.
        None => {
            rsx! {
                document::Link { rel: "icon", href: FAVICON }
                document::Link { rel: "stylesheet", href: MAIN_CSS }
                document::Link { rel: "stylesheet", href: TAILWIND_CSS }
                div {
                    class: "container",
                    h1 { "Loading model..." }
                    p { "Please wait, this may take a moment on the first run." }
                }
            }
        }
    }
}
