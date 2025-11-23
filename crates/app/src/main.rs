//! Main entry point for the Dioxus desktop application.
// use dioxus::desktop::muda::{Menu, MenuItem, PredefinedMenuItem, Submenu};
use dioxus::prelude::*;
use dioxus::{logger::tracing::Level, router::Navigator};

mod components;
mod modules;

use components::{chat_view::ChatView, dot_display::GraphEditor};
use modules::server_manager;

const FAVICON: Asset = asset!("/assets/favicon.ico");
const TAILWIND_CSS: Asset = asset!("/assets/tailwind.css");

// #[derive(Routable, Clone)]
// enum Route {
//     #[route("/")]
//     Home {},
// }

fn main() {
    let config = dioxus::desktop::Config::new();

    dioxus::logger::init(Level::INFO).expect("failed to init logger");
    LaunchBuilder::desktop().with_cfg(config).launch(App);
}

#[component]
fn App() -> Element {
    let server_status = server_manager::use_server_manager();

    // Provide None as Option<Navigator> to disable routing for now
    use_context_provider(|| None::<Navigator>);

    rsx! {
        // Router::<Route> {}
        document::Link { rel: "icon", href: FAVICON }
        document::Link { rel: "stylesheet", href: TAILWIND_CSS }

        match &*server_status.value().read() {
            Some(Ok(_)) => rsx! {
                // GraphEditor is on top, with a thin chat bar at the bottom, using tailwind flexbox
                div {
                    class: "flex flex-col h-screen",
                    style: "background-color: #f9fafb;",
                    div {
                        class: "flex-grow overflow-auto",
                        GraphEditor {}
                    }
                    div {
                        class: "border-t p-4 bg-white",
                        ChatView {}
                    }
                }
            },
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
