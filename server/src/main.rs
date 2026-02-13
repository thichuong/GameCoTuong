use axum::{routing::get, Router};
use game_manager::AppState;
use std::sync::Arc;
use ws::ws_handler;

mod game_manager;
mod ws;

#[tokio::main]
async fn main() {
    // initialize tracing
    tracing_subscriber::fmt::init();

    let state = Arc::new(AppState::new());
    state.clone().spawn_cleanup_task();

    // build our application with a route
    let app = Router::new()
        .route("/ws", get(ws_handler))
        .with_state(state);

    // run our app with hyper
    let host = std::env::var("HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
    let port = std::env::var("PORT").unwrap_or_else(|_| "3000".to_string());
    let addr_str = format!("{}:{}", host, port);

    tracing::info!("listening on {}", addr_str);

    let listener = tokio::net::TcpListener::bind(&addr_str)
        .await
        .expect("Failed to bind to address");
    axum::serve(listener, app)
        .await
        .expect("Failed to start server");
}
