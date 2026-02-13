use crate::game_manager::{AppState, GameSession};
use cotuong_core::logic::board::{Board, Color};
use shared::ServerMessage;
use tracing;
use uuid::Uuid;

impl AppState {
    pub async fn find_match(&self, player_id: String) {
        if self.player_to_game.contains_key(&player_id) {
            tracing::warn!(player_id = %player_id, "Player already in game, ignoring find_match");
            return;
        }

        let mut queue = self.matchmaking_queue.lock().await;

        if queue.contains(&player_id) {
            tracing::debug!(player_id = %player_id, "Player already in queue");
            return;
        }

        let opponent_opt = queue.iter().next().cloned();

        if let Some(opponent_id) = opponent_opt {
            tracing::info!(player_id = %player_id, opponent_id = %opponent_id, "Opponent found, starting game");
            queue.remove(&opponent_id);
            drop(queue);
            self.start_game(player_id, opponent_id).await;
        } else {
            tracing::info!(player_id = %player_id, "No opponent found, adding to queue");
            queue.insert(player_id.clone());
            drop(queue);

            if let Some(player) = self.players.get(&player_id) {
                let _ = player.tx.send(ServerMessage::WaitingForMatch);
            }
        }
    }

    async fn start_game(&self, p1_id: String, p2_id: String) {
        let game_id = Uuid::new_v4().to_string();

        let (red_id, black_id) = if rand::random() {
            (p1_id.clone(), p2_id.clone())
        } else {
            (p2_id.clone(), p1_id.clone())
        };

        tracing::info!(game_id = %game_id, red = %red_id, black = %black_id, "Created new game session");

        use std::time::Instant;
        let game = GameSession {
            red_player: red_id.clone(),
            black_player: black_id.clone(),
            board: Board::new(),
            turn: Color::Red,
            game_ended: false,
            red_ready_for_rematch: false,
            black_ready_for_rematch: false,
            pending_move: None,
            last_activity: Instant::now(),
        };

        use tokio::sync::RwLock;
        self.games.insert(game_id.clone(), RwLock::new(game));
        self.player_to_game.insert(p1_id.clone(), game_id.clone());
        self.player_to_game.insert(p2_id.clone(), game_id.clone());

        if let Some(p) = self.players.get(&red_id) {
            let _ = p.tx.send(ServerMessage::MatchFound {
                opponent_id: black_id.clone(),
                your_color: Color::Red,
                game_id: game_id.clone(),
            });
            let _ = p.tx.send(ServerMessage::GameStart(Box::new(Board::new())));
        }

        if let Some(p) = self.players.get(&black_id) {
            let _ = p.tx.send(ServerMessage::MatchFound {
                opponent_id: red_id.clone(),
                your_color: Color::Black,
                game_id: game_id.clone(),
            });
            let _ = p.tx.send(ServerMessage::GameStart(Box::new(Board::new())));
        }
    }
}
