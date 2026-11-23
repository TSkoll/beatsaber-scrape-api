use crate::api::AppState;

pub mod api;
pub mod fetch;
pub mod maps;

const BIND_ADDRESS: &str = "0.0.0.0:3000";

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    tracing::info!(
        application = env!("CARGO_PKG_NAME"),
        version = env!("CARGO_PKG_VERSION"),
        "starting application"
    );

    let state = AppState::new();
    fetch::spawn_refresh_task(state.clone());

    let listener = tokio::net::TcpListener::bind(BIND_ADDRESS)
        .await
        .unwrap_or_else(|err| panic!("failed to bind {BIND_ADDRESS}: {err}"));
    let address = listener
        .local_addr()
        .expect("failed to read listener address");
    tracing::info!(%address, "listening");

    axum::serve(listener, api::router(state))
        .with_graceful_shutdown(shutdown_signal())
        .await
        .expect("server failed");
}

async fn shutdown_signal() {
    if let Err(err) = tokio::signal::ctrl_c().await {
        tracing::error!(%err, "failed to wait for shutdown signal");
    }
}
