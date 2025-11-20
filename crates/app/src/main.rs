use dioxus::desktop::muda::{Menu, MenuItem, PredefinedMenuItem, Submenu};
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
    let menu = Menu::new();

    let edit_menu = Submenu::new("Edit", true);

    edit_menu
        .append_items(&[
            &PredefinedMenuItem::undo(None),
            &PredefinedMenuItem::redo(None),
            &PredefinedMenuItem::separator(),
            &PredefinedMenuItem::cut(None),
            &PredefinedMenuItem::copy(None),
            &PredefinedMenuItem::paste(None),
            &PredefinedMenuItem::select_all(None),
            &MenuItem::with_id("switch-text", "Switch text", true, None),
        ])
        .unwrap();

    menu.append(&edit_menu).unwrap();

    let config = dioxus::desktop::Config::new().with_menu(menu);

    dioxus::logger::init(Level::INFO).expect("failed to init logger");
    LaunchBuilder::desktop().with_cfg(config).launch(App);
}

#[component]
fn App() -> Element {
    // : Resource<Result<Option<ServerProcess>>>
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
