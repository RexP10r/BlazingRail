use anyhow::Result;
use axum::{Router, routing::post};
use common::{AppConfig, EventInput};
use std::{net::SocketAddr, sync::Arc};
use tokio::{net::TcpListener, sync::mpsc::channel};

mod error;

mod state;
use state::AppState;

mod handler;
use handler::handle_create_event;

#[tokio::main]
async fn main() -> Result<()> {
    let config = Arc::new(AppConfig::new());

    let (tx, _rx) = channel::<EventInput>(config.channel_capacity);

    let state = Arc::new(AppState::new(config.clone(), tx));
    let app: Router = Router::new()
        .route("/v1/events", post(handle_create_event))
        .with_state(state);

    let addr = SocketAddr::from(([127, 0, 0, 1], config.server_port));
    println!("Server launched on {}", &addr);

    let listener = TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
