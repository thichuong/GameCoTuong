use crate::game_manager::GameManager;
use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::IntoResponse,
};
use futures::{sink::SinkExt, stream::StreamExt};
use shared::{GameMessage, ServerMessage};
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;

pub struct AppState {
    pub game_manager: Mutex<GameManager>,
}

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
            let json = serde_json::to_string(&msg).unwrap();
            if sender.send(Message::Text(json)).await.is_err() {
                break;
            }
        }
    });

    // Generate a random ID for the player
    let player_id = uuid::Uuid::new_v4().to_string();

    // Add player to manager
    {
        let mut gm = state.game_manager.lock().unwrap();
        gm.add_player(player_id.clone(), tx);
    }

    while let Some(Ok(msg)) = receiver.next().await {
        if let Message::Text(text) = msg {
            if let Ok(game_msg) = serde_json::from_str::<GameMessage>(&text) {
                let mut gm = state.game_manager.lock().unwrap();
                match game_msg {
                    GameMessage::FindMatch => gm.find_match(player_id.clone()),
                    GameMessage::MakeMove(mv) => gm.handle_move(player_id.clone(), mv),
                    GameMessage::CancelFindMatch => {
                        // TODO
                    }
                    // Other messages...
                    _ => {}
                }
            }
        }
    }

    // Client disconnected
    {
        let mut gm = state.game_manager.lock().unwrap();
        gm.remove_player(&player_id);
    }
}
