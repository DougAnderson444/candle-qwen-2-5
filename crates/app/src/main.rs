use dioxus::logger::tracing::Level;
use dioxus::prelude::*;

mod components;
mod modules;

use components::chat_view::ChatView;
use modules::server_manager;

const FAVICON: Asset = asset!("/assets/favicon.ico");
const MAIN_CSS: Asset = asset!("/assets/main.css");
const TAILWIND_CSS: Asset = asset!("/assets/tailwind.css");

fn main() {
    dioxus::logger::init(Level::INFO).expect("failed to init logger");
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    let server_status = server_manager::use_server_manager();

    rsx! {
        document::Link { rel: "icon", href: FAVICON }
        document::Link { rel: "stylesheet", href: MAIN_CSS }
        document::Link { rel: "stylesheet", href: TAILWIND_CSS }

        match &*server_status.value().read() {
            Some(Ok(_)) => rsx! { ChatView {} },
            Some(Err(e)) => {
                let error_message = e.to_string();
                rsx! {
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
                    div {
                        class: "container",
                        h1 { "Starting API server..." }
                        p { "Please wait, this may take a moment." }
                    }
                }
            }
        }
    }
}