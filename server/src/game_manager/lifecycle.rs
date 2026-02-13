use crate::game_manager::{session::Player, AppState};
use cotuong_core::logic::board::{Board, Color};
use shared::ServerMessage;
use tracing; // Added tracing import

impl AppState {
    pub fn add_player(&self, id: String, tx: crate::game_manager::Tx) {
        use std::time::Instant;
        tracing::info!(player_id = %id, "Player added to AppState");
        self.players.insert(
            id,
            Player {
                tx,
                last_msg_at: Instant::now(),
            },
        );
    }

    pub async fn remove_player(&self, id: &str) {
        tracing::info!(player_id = %id, "Removing player from AppState");
        self.players.remove(id);

        {
            let mut queue = self.matchmaking_queue.lock().await;
            if queue.remove(id) {
                tracing::info!(player_id = %id, "Player removed from matchmaking queue");
            }
        }

        if let Some((_, game_id)) = self.player_to_game.remove(id) {
            tracing::info!(player_id = %id, game_id = %game_id, "Player was in a game, cleaning up session");
            if let Some((_, game_lock)) = self.games.remove(&game_id) {
                let game = game_lock.read().await;
                let opponent_id = if game.red_player == id {
                    game.black_player.clone()
                } else {
                    game.red_player.clone()
                };

                let winner = if game.red_player == id {
                    Color::Black
                } else {
                    Color::Red
                };

                tracing::info!(game_id = %game_id, disconnected_player = %id, opponent_id = %opponent_id, "Notifying opponent of disconnection");
                drop(game);

                if let Some(player) = self.players.get(&opponent_id) {
                    let _ = player.tx.send(ServerMessage::OpponentDisconnected);
                    let _ = player.tx.send(ServerMessage::GameEnd {
                        winner: Some(winner),
                        reason: "Opponent Disconnected".to_string(),
                    });
                }
                self.player_to_game.remove(&opponent_id);
            }
        }
    }

    pub async fn handle_surrender(&self, player_id: String) {
        if let Some(game_id) = self.player_to_game.get(&player_id) {
            let game_id = game_id.value().clone();
            tracing::info!(player_id = %player_id, game_id = %game_id, "Player surrendered");
            if let Some(game_lock) = self.games.get(&game_id) {
                let mut game = game_lock.write().await;
                if game.game_ended {
                    tracing::debug!(game_id = %game_id, "Surrender ignored: game already ended");
                    return;
                }

                game.game_ended = true;
                let is_red = game.red_player == player_id;
                let winner = if is_red { Color::Black } else { Color::Red };

                let red_id = game.red_player.clone();
                let black_id = game.black_player.clone();

                drop(game);

                tracing::info!(game_id = %game_id, winner = ?winner, "Game ended by surrender");
                if let Some(p) = self.players.get(&red_id) {
                    let _ = p.tx.send(ServerMessage::GameEnd {
                        winner: Some(winner),
                        reason: "Surrender".to_string(),
                    });
                }
                if let Some(p) = self.players.get(&black_id) {
                    let _ = p.tx.send(ServerMessage::GameEnd {
                        winner: Some(winner),
                        reason: "Surrender".to_string(),
                    });
                }
            }
        }
    }

    pub async fn handle_play_again(&self, player_id: String) {
        if let Some(game_id) = self.player_to_game.get(&player_id) {
            let game_id = game_id.value().clone();
            tracing::info!(player_id = %player_id, game_id = %game_id, "Player requested rematch");
            if let Some(game_lock) = self.games.get(&game_id) {
                let mut game = game_lock.write().await;

                let is_red = game.red_player == player_id;
                if is_red {
                    game.red_ready_for_rematch = true;
                } else {
                    game.black_ready_for_rematch = true;
                }

                if game.red_ready_for_rematch && game.black_ready_for_rematch {
                    tracing::info!(game_id = %game_id, "Both players ready, restarting game");
                    let red_id = game.red_player.clone();
                    let black_id = game.black_player.clone();

                    game.board = Board::new();
                    game.turn = Color::Red;
                    game.game_ended = false;
                    game.red_ready_for_rematch = false;
                    game.black_ready_for_rematch = false;
                    game.pending_move = None;

                    drop(game);

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
        }
    }

    pub async fn leave_game(&self, player_id: &str) {
        tracing::info!(player_id = %player_id, "Player leaving current game");
        {
            let mut queue = self.matchmaking_queue.lock().await;
            queue.remove(player_id);
        }

        if let Some((_, game_id)) = self.player_to_game.remove(player_id) {
            tracing::info!(player_id = %player_id, game_id = %game_id, "Cleaning up game session for left player");
            if let Some((_, game_lock)) = self.games.remove(&game_id) {
                let game = game_lock.read().await;
                let opponent_id = if game.red_player == player_id {
                    game.black_player.clone()
                } else {
                    game.red_player.clone()
                };
                let winner = if game.red_player == player_id {
                    Color::Black
                } else {
                    Color::Red
                };
                let game_ended = game.game_ended;
                drop(game);

                self.player_to_game.remove(&opponent_id);

                if !game_ended {
                    tracing::info!(game_id = %game_id, player_id = %player_id, opponent_id = %opponent_id, "In-progress game ended because player left");
                    if let Some(player) = self.players.get(&opponent_id) {
                        let _ = player.tx.send(ServerMessage::OpponentDisconnected);
                        let _ = player.tx.send(ServerMessage::GameEnd {
                            winner: Some(winner),
                            reason: "Opponent Left".to_string(),
                        });
                    }
                } else {
                    tracing::info!(game_id = %game_id, player_id = %player_id, opponent_id = %opponent_id, "Player left room after game ended");
                    if let Some(player) = self.players.get(&opponent_id) {
                        let _ = player.tx.send(ServerMessage::OpponentLeftGame);
                    }
                }
            }
        }
    }

    pub async fn handle_player_left(&self, player_id: String) {
        self.leave_game(&player_id).await;
    }

    pub fn spawn_cleanup_task(self: std::sync::Arc<Self>) {
        tokio::spawn(async move {
            use std::time::{Duration, Instant};
            let mut interval = tokio::time::interval(Duration::from_secs(300)); // Every 5 mins
            loop {
                interval.tick().await;
                let now = Instant::now();
                let mut games_to_remove = Vec::new();

                for entry in self.games.iter() {
                    let game = entry.value().read().await;
                    if now.duration_since(game.last_activity) > Duration::from_secs(3600) {
                        games_to_remove.push(entry.key().clone());
                    }
                }

                for game_id in games_to_remove {
                    tracing::info!("Cleaning up inactive game: {}", game_id);
                    if let Some((_, game_lock)) = self.games.remove(&game_id) {
                        let game = game_lock.read().await;
                        self.player_to_game.remove(&game.red_player);
                        self.player_to_game.remove(&game.black_player);
                    }
                }
            }
        });
    }
}
