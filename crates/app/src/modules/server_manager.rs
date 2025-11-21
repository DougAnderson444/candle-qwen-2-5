use dioxus::core::use_drop;
use dioxus::logger::tracing;
use dioxus::prelude::*;
use reqwest;
use std::time::Duration;
use tokio;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{Child, Command};

use super::api_client::{self};

/// A resource that holds the server process child, ensuring it's terminated on drop.
pub struct ServerProcess(Option<Child>);

impl ServerProcess {
    /// Gracefully shutdown the server and wait for it to exit
    pub async fn shutdown(mut self) -> Result<(), std::io::Error> {
        if let Some(mut child) = self.0.take() {
            tracing::info!("Shutting down API server...");
            child.kill().await?;
            child.wait().await?;
            tracing::info!("API server exited.");
        }
        Ok(())
    }

    fn new(child: Child) -> Self {
        Self(Some(child))
    }
}

impl Drop for ServerProcess {
    fn drop(&mut self) {
        if let Some(mut child) = self.0.take() {
            tracing::warn!(
                "ServerProcess dropped without explicit shutdown - spawning cleanup task"
            );
            // spawn a thread that would outlives the main context which is shutting down
            std::thread::spawn(move || {
                let rt = tokio::runtime::Runtime::new().unwrap();
                rt.block_on(async move {
                    if let Err(e) = child.kill().await {
                        tracing::error!("Failed to kill server process: {}", e);
                    }
                    // if let Err(e) = Command::new("pkill")
                    //     .args(["-9", "api-server"])
                    //     .output()
                    //     .await
                    // {
                    //     tracing::error!("Failed to kill server process: {}", e);
                    // }
                });
            });
        }
    }
}

pub fn use_server_manager() -> Resource<Result<ServerStatus, anyhow::Error>> {
    let mut server_signal = use_signal(|| None::<ServerProcess>);

    // Cleanup when the component unmounts
    use_drop(move || {
        spawn(async move {
            if let Some(server) = server_signal.write().take() {
                tracing::info!("Application closing, shutting down server...");
                if let Err(e) = server.shutdown().await {
                    tracing::error!("Error shutting down server: {}", e);
                }
            }
        });
    });

    use_resource(move || async move {
        let server_addr = format!("http://localhost:{}", api_client::PORT);
        tracing::info!("Checking for API server at {}...", server_addr);
        if reqwest::get(&server_addr).await.is_ok() {
            tracing::info!("API server is already running.");
            return Ok::<_, anyhow::Error>(ServerStatus::AlreadyRunning);
        }

        tracing::info!("API server not found. Spawning a new one...");
        let mut child = Command::new("../../target/release/api-server")
            .arg("--port")
            .arg(api_client::PORT.to_string())
            .stdout(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| {
                anyhow::anyhow!(
                    "Failed to spawn server: {}. Have you built it with 'just build-release -p api-server'?",
                    e
                )
            })?;

        // Wait for "Listening on http://0.0.0.0:{PORT}" in stdout
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| anyhow::anyhow!("Failed to capture server stdout"))?;
        let mut reader = BufReader::new(stdout).lines();
        let expected = format!("Listening on http://0.0.0.0:{}", api_client::PORT);
        let mut found = false;
        let start = tokio::time::Instant::now();

        while start.elapsed() < Duration::from_secs(10) {
            match tokio::time::timeout(Duration::from_secs(1), reader.next_line()).await {
                Ok(Ok(Some(line))) => {
                    tracing::info!("Server log: {}", line);
                    if line.contains(&expected) {
                        found = true;
                        break;
                    }
                }
                Ok(Ok(None)) => break, // EOF
                Ok(Err(e)) => {
                    let _ = child.kill().await;
                    return Err(anyhow::anyhow!("Error reading server stdout: {}", e));
                }
                Err(_) => continue, // Timeout, keep trying
            }
        }

        if !found {
            let _ = child.kill().await;
            return Err(anyhow::anyhow!(
                "API server did not print '{}' within 10 seconds.",
                expected
            ));
        }

        tracing::info!("API server started successfully.");

        // Store the server process in the signal for cleanup
        *server_signal.write() = Some(ServerProcess::new(child));

        Ok(ServerStatus::Started)
    })
}

#[derive(Debug, Clone, PartialEq)]
pub enum ServerStatus {
    AlreadyRunning,
    Started,
}
