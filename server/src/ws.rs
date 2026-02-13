use crate::game_manager::AppState;
use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::IntoResponse,
};
use futures::{sink::SinkExt, stream::StreamExt};
use shared::GameMessage;
use std::sync::Arc;
use tokio::sync::mpsc;

pub async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_socket(socket, state))
}

async fn handle_socket(socket: WebSocket, state: Arc<AppState>) {
    let (mut sender, mut receiver) = socket.split();
    let (tx, mut rx) = mpsc::unbounded_channel();

    // Spawn a task to forward messages from the channel to the WebSocket
    tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if let Ok(json) = serde_json::to_string(&msg) {
                if sender.send(Message::Text(json)).await.is_err() {
                    break;
                }
            }
        }
    });

    // Generate a random ID for the player
    let player_id = uuid::Uuid::new_v4().to_string();

    // Add player to manager
    state.add_player(player_id.clone(), tx);

    while let Some(Ok(msg)) = receiver.next().await {
        if let Message::Text(text) = msg {
            if let Ok(game_msg) = serde_json::from_str::<GameMessage>(&text) {
                match game_msg {
                    GameMessage::FindMatch => state.find_match(player_id.clone()).await,
                    GameMessage::MakeMove { move_data, fen } => {
                        state.handle_move(player_id.clone(), move_data, fen).await
                    }
                    GameMessage::VerifyMove { fen, is_valid } => {
                        state
                            .handle_verify_move(player_id.clone(), fen, is_valid)
                            .await
                    }
                    GameMessage::CancelFindMatch => {
                        let mut queue = state.matchmaking_queue.lock().await;
                        queue.remove(&player_id);
                    }
                    GameMessage::Surrender => state.handle_surrender(player_id.clone()).await,
                    GameMessage::PlayAgain => state.handle_play_again(player_id.clone()).await,
                    GameMessage::PlayerLeft => state.handle_player_left(player_id.clone()).await,
                    _ => {}
                }
            }
        }
    }

    // Client disconnected
    state.remove_player(&player_id).await;
}
