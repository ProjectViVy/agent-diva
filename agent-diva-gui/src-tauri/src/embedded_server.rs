use agent_diva_manager::{start_embedded_gateway_runtime, GatewayRuntimeConfig};
use anyhow::Context;
use std::net::TcpListener;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::watch;

const TOKIO_RUNTIME_SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(5);

/// Handle to the running embedded gateway.
/// Drop or call `shutdown()` to stop the server gracefully.
#[allow(dead_code)]
pub struct EmbeddedGatewayHandle {
    /// The port the server is listening on.
    pub port: u16,
    /// Send `true` to trigger graceful shutdown.
    shutdown_tx: watch::Sender<bool>,
    /// Join handle for the background server thread.
    server_thread: Option<std::thread::JoinHandle<()>>,
    startup_rx: Option<mpsc::Receiver<Result<(), String>>>,
    /// Track whether shutdown has already been initiated.
    shutdown_initiated: Arc<AtomicBool>,
}

impl EmbeddedGatewayHandle {
    #[cfg(test)]
    fn wait_until_ready(&mut self, timeout: Duration) -> anyhow::Result<()> {
        let receiver = self
            .startup_rx
            .take()
            .context("embedded gateway startup result was already consumed")?;
        receiver
            .recv_timeout(timeout)
            .context("embedded gateway did not report startup readiness")?
            .map_err(anyhow::Error::msg)
    }

    /// Signal the server to shut down and wait for the background thread.
    #[allow(dead_code)]
    pub fn shutdown(mut self) {
        if self
            .shutdown_initiated
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::Relaxed)
            .is_ok()
        {
            let _ = self.shutdown_tx.send(true);
            if let Some(handle) = self.server_thread.take() {
                let _ = handle.join();
            }
            tracing::info!("Embedded gateway stopped");
        }
    }
}

impl Drop for EmbeddedGatewayHandle {
    fn drop(&mut self) {
        if self
            .shutdown_initiated
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::Relaxed)
            .is_ok()
        {
            let _ = self.shutdown_tx.send(true);
        }
    }
}

/// Start the embedded gateway server on a background thread.
/// Returns a handle that can be used to shut down the server.
#[allow(dead_code)]
pub fn start_embedded_gateway(
    config: GatewayRuntimeConfig,
) -> anyhow::Result<EmbeddedGatewayHandle> {
    let std_listener = TcpListener::bind("127.0.0.1:0")
        .context("failed to bind embedded gateway listener to 127.0.0.1:0")?;
    let port = std_listener
        .local_addr()
        .context("failed to read embedded gateway listener address")?
        .port();

    tracing::info!("Embedded gateway bound to http://127.0.0.1:{port}");

    let (shutdown_tx, shutdown_rx) = watch::channel(false);
    let (startup_tx, startup_rx) = mpsc::sync_channel(1);
    let shutdown_initiated = Arc::new(AtomicBool::new(false));

    let server_thread = std::thread::Builder::new()
        .name("agent-diva-gateway".into())
        .spawn(move || {
            let runtime = match tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
            {
                Ok(runtime) => runtime,
                Err(error) => {
                    let _ = startup_tx.send(Err(format!(
                        "failed to create embedded gateway runtime: {error}"
                    )));
                    tracing::error!("Failed to create embedded gateway runtime: {}", error);
                    return;
                }
            };

            runtime.block_on(async move {
                if let Err(error) =
                    run_embedded_gateway_task(config, std_listener, shutdown_rx, startup_tx.clone())
                        .await
                {
                    let _ = startup_tx.send(Err(error.to_string()));
                    tracing::error!("Embedded gateway task failed: {}", error);
                }
            });
            runtime.shutdown_timeout(TOKIO_RUNTIME_SHUTDOWN_TIMEOUT);
        })
        .context("failed to spawn embedded gateway thread")?;

    Ok(EmbeddedGatewayHandle {
        port,
        shutdown_tx,
        server_thread: Some(server_thread),
        startup_rx: Some(startup_rx),
        shutdown_initiated,
    })
}

async fn run_embedded_gateway_task(
    config: GatewayRuntimeConfig,
    std_listener: TcpListener,
    shutdown_rx: watch::Receiver<bool>,
    startup_tx: mpsc::SyncSender<Result<(), String>>,
) -> anyhow::Result<()> {
    std_listener
        .set_nonblocking(true)
        .context("failed to set embedded gateway listener nonblocking mode")?;
    let listener = tokio::net::TcpListener::from_std(std_listener)
        .context("failed to convert embedded gateway listener to tokio listener")?;

    let mut lifecycle_rx = shutdown_rx.clone();
    let runtime = start_embedded_gateway_runtime(config, listener, shutdown_rx)
        .await
        .context("failed to bootstrap embedded manager runtime")?;
    let _ = startup_tx.send(Ok(()));

    if !*lifecycle_rx.borrow() {
        lifecycle_rx
            .wait_for(|shutdown| *shutdown)
            .await
            .context("embedded gateway shutdown sender dropped before shutdown")?;
    }

    runtime.shutdown().await;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use agent_diva_core::config::{Config, ConfigLoader};
    use reqwest::StatusCode;
    use std::path::PathBuf;
    use tempfile::TempDir;

    fn write_test_config(config_dir: &TempDir) -> ConfigLoader {
        let loader = ConfigLoader::with_dir(config_dir.path());
        let config = Config::default();
        loader.save(&config).unwrap();
        loader
    }

    fn test_runtime_config(config_dir: &TempDir, workspace_dir: &TempDir) -> GatewayRuntimeConfig {
        let loader = write_test_config(config_dir);
        let config = loader.load().unwrap();
        GatewayRuntimeConfig {
            config,
            loader: loader.clone(),
            workspace: agent_diva_core::workspace::WorkspaceContext {
                root: PathBuf::from(workspace_dir.path()),
                source: agent_diva_core::workspace::WorkspaceSource::Configured,
                agents_md: None,
            },
            cron_store: config_dir.path().join("cron.json"),
            port: 0,
        }
    }

    #[test]
    fn port_binding_uses_ephemeral_port() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        assert!(port > 0);
    }

    #[test]
    fn shutdown_flag_is_single_use() {
        let flag = Arc::new(AtomicBool::new(false));
        assert!(flag
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::Relaxed)
            .is_ok());
        assert!(flag
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::Relaxed)
            .is_err());
    }

    #[test]
    fn drop_sends_shutdown_without_panicking() {
        let (shutdown_tx, shutdown_rx) = watch::channel(false);
        let handle = EmbeddedGatewayHandle {
            port: 12345,
            shutdown_tx,
            server_thread: None,
            startup_rx: None,
            shutdown_initiated: Arc::new(AtomicBool::new(false)),
        };

        drop(handle);
        assert!(*shutdown_rx.borrow());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn immediate_shutdown_during_startup_is_bounded() {
        let config_dir = TempDir::new().unwrap();
        let workspace_dir = TempDir::new().unwrap();
        let config = test_runtime_config(&config_dir, &workspace_dir);
        let handle = start_embedded_gateway(config).unwrap();

        let shutdown = tokio::task::spawn_blocking(move || handle.shutdown());
        tokio::time::timeout(Duration::from_secs(15), shutdown)
            .await
            .expect("gateway shutdown exceeded its timeout")
            .expect("gateway shutdown task panicked");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn embedded_gateway_serves_health_endpoint() {
        let config_dir = TempDir::new().unwrap();
        let workspace_dir = TempDir::new().unwrap();
        let config = test_runtime_config(&config_dir, &workspace_dir);

        let mut handle = start_embedded_gateway(config).unwrap();
        handle
            .wait_until_ready(Duration::from_secs(30))
            .expect("embedded gateway failed during startup");
        let port = handle.port;
        // The desktop process may inherit corporate proxy variables. The
        // embedded gateway is always loopback-only and must never be probed
        // through an external proxy.
        let client = reqwest::Client::builder()
            .no_proxy()
            .timeout(Duration::from_millis(500))
            .build()
            .unwrap();

        let health_result = tokio::time::timeout(Duration::from_secs(10), async {
            let mut last_error = None;
            for _ in 0..20 {
                match client
                    .get(format!("http://127.0.0.1:{port}/api/health"))
                    .send()
                    .await
                {
                    Ok(response)
                        if response.status() == StatusCode::OK
                            || response.status() == StatusCode::SERVICE_UNAVAILABLE =>
                    {
                        return Ok(());
                    }
                    Ok(response) => {
                        last_error = Some(format!("status {}", response.status()));
                    }
                    Err(error) => {
                        last_error = Some(error.to_string());
                    }
                }
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
            Err(last_error.unwrap_or_else(|| "unknown error".to_string()))
        })
        .await;

        let shutdown = tokio::task::spawn_blocking(move || handle.shutdown());
        tokio::time::timeout(Duration::from_secs(15), shutdown)
            .await
            .expect("gateway shutdown exceeded its timeout")
            .expect("gateway shutdown task panicked");

        match health_result {
            Ok(Ok(())) => {}
            Ok(Err(error)) => panic!("embedded gateway health check did not become ready: {error}"),
            Err(_) => panic!("embedded gateway health check exceeded its readiness timeout"),
        }
    }
}
