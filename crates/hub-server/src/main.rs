// Hub HTTP server binary

use tracing_subscriber::prelude::*;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "tiny_a2a_hub=debug,axum=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let app = hub_server::create_app();

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000").await?;
    tracing::info!("Hub server listening on {}", listener.local_addr()?);

    axum::serve(listener, app).await?;

    Ok(())
}
