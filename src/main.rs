use std::net::SocketAddr;

use tokio::net::TcpListener;
use tracing::info;

use items_service::config::Config;
use items_service::routes::{router, AppState};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt().init();

    let config = Config::from_env()?;
    let address = SocketAddr::from(([0, 0, 0, 0], config.port));
    info!(
        %address,
        pod_id = %config.pod_id,
        datacenter = %config.datacenter,
        "items-service запущен"
    );

    let listener = TcpListener::bind(address).await?;
    axum::serve(listener, router(AppState::new(config)?)).await?;

    Ok(())
}
