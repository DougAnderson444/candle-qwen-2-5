use dioxus::logger::tracing;
use dioxus::prelude::*;
use reqwest;
use std::process::{Child, Command};
use std::time::Duration;
use tokio;

use super::api_client::{self};

/// A resource that holds the server process child, ensuring it's terminated on drop.
pub struct ServerProcess(Child);

impl Drop for ServerProcess {
    fn drop(&mut self) {
        tracing::info!("Shutting down API server...");
        if let Err(e) = self.0.kill() {
            tracing::error!("Failed to kill server process: {}", e);
        }
    }
}

pub fn use_server_manager() -> Resource<Result<Option<ServerProcess>, anyhow::Error>> {
    use_resource(move || async move {
        let server_addr = format!("http://localhost:{}", api_client::PORT);
        tracing::info!("Checking for API server at {}...", server_addr);
        if reqwest::get(&server_addr).await.is_ok() {
            tracing::info!("API server is already running.");
            return Ok::<_, anyhow::Error>(None);
        }

        tracing::info!("API server not found. Spawning a new one...");
        let child = Command::new("../../target/release/api-server")
            .arg("--port")
            .arg(api_client::PORT.to_string())
            .spawn()
            .map_err(|e| {
                anyhow::anyhow!(
                    "Failed to spawn server: {}. Have you built it with 'just build-release -p api-server'?",
                    e
                )
            })?;

        // Wait a moment for the server to start up.
        tokio::time::sleep(Duration::from_secs(3)).await;

        // Final check to ensure it started.
        reqwest::get(&server_addr).await?;
        tracing::info!("API server started successfully.");

        Ok(Some(ServerProcess(child)))
    })
}
