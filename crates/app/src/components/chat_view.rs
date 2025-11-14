use crate::modules::api_client::ApiClient;
use dioxus::prelude::*;

#[component]
pub fn ChatView() -> Element {
    let mut prompt = use_signal(|| "Q: What is 2 + 2?\nA:".to_string());
    let mut output = use_signal(String::new);
    let mut is_generating = use_signal(|| false);
    let api_client = use_hook(|| ApiClient::new);

    rsx! {
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
                        let mut first_token = true;
                        let result = api_client().generate_stream(prompt_val, move |token| {
                            if first_token {
                                output.set(token);
                                first_token = false;
                            } else {
                                output.with_mut(|out| out.push_str(&token));
                            }
                        }).await;

                        if let Err(e) = result {
                            output.set(format!("API request failed: {}", e));
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
