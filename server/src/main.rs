use axum::{routing::get, Router};
use game_manager::AppState;
use std::{net::SocketAddr, sync::Arc};
use ws::ws_handler;

mod game_manager;
mod ws;

#[tokio::main]
async fn main() {
    // initialize tracing
    tracing_subscriber::fmt::init();

    let state = Arc::new(AppState::new());

    // build our application with a route
    let app = Router::new()
        .route("/ws", get(ws_handler))
        .with_state(state);

    // run our app with hyper
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    tracing::debug!("listening on {}", addr);
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("Failed to bind to address");
    axum::serve(listener, app)
        .await
        .expect("Failed to start server");
}
