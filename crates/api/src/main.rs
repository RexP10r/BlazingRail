use anyhow::Result;
use axum::{Router, routing::post};
use std::{net::SocketAddr, sync::Arc};
use tokio::net::TcpListener;

mod state;
use state::AppState;

mod handler;
use handler::handle_create_event;

#[tokio::main]
async fn main() -> Result<()> {
    let state = Arc::new(AppState::init());

    let app: Router = Router::new()
        .route("/v1/events", post(handle_create_event))
        .with_state(state);

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    println!("Server launched on {}", &addr);

    let listener = TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
