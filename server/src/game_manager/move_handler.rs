use crate::game_manager::{session::has_any_valid_move, AppState};
use cotuong_core::{
    engine::Move,
    logic::board::{Board, Color},
};
use shared::ServerMessage;

impl AppState {
    pub async fn handle_move(&self, player_id: String, mv: Move, fen: String) {
        let game_id = if let Some(gid) = self.player_to_game.get(&player_id) {
            gid.value().clone()
        } else {
            return;
        };

        if let Some(game_lock) = self.games.get(&game_id) {
            let mut game = game_lock.write().await;

            if game.game_ended {
                return;
            }

            let is_red = game.red_player == player_id;
            let player_color = if is_red { Color::Red } else { Color::Black };

            if game.turn != player_color {
                return;
            }

            game.pending_move = Some((player_id.clone(), mv, fen.clone()));
            use std::time::Instant;
            game.last_activity = Instant::now();

            let opponent_id = if is_red {
                game.black_player.clone()
            } else {
                game.red_player.clone()
            };

            drop(game);

            if let Some(p) = self.players.get(&opponent_id) {
                let _ =
                    p.tx.send(ServerMessage::OpponentMove { move_data: mv, fen });
            }
        }
    }

    pub async fn handle_verify_move(&self, player_id: String, _fen: String, is_valid: bool) {
        let game_id = if let Some(gid) = self.player_to_game.get(&player_id) {
            gid.value().clone()
        } else {
            return;
        };

        if let Some(game_lock) = self.games.get(&game_id) {
            let mut game = game_lock.write().await;

            let (pending_data, red_id, black_id) = (
                game.pending_move.clone(),
                game.red_player.clone(),
                game.black_player.clone(),
            );

            if let Some((mover_id, mv, claimed_fen)) = pending_data {
                let is_mover_red = mover_id == red_id;
                let opponent_id = if is_mover_red {
                    black_id.clone()
                } else {
                    red_id.clone()
                };

                if player_id != opponent_id {
                    return;
                }

                if is_valid {
                    if let Ok((new_board, new_turn)) = Board::from_fen(&claimed_fen) {
                        use std::time::Instant;
                        game.last_activity = Instant::now();
                        game.board = new_board;
                        game.turn = new_turn;
                        game.pending_move = None;

                        let board_snapshot = game.board.clone();
                        let turn_snapshot = game.turn;

                        let has_moves = has_any_valid_move(&board_snapshot, turn_snapshot);
                        if !has_moves {
                            let winner = if game.turn == Color::Red {
                                Color::Black
                            } else {
                                Color::Red
                            };
                            game.game_ended = true;

                            drop(game);

                            self.notify_game_end(&game_id, winner, "Checkmate".to_string())
                                .await;
                        }
                    } else {
                        drop(game);
                        self.resolve_conflict(&game_id, &mv).await;
                    }
                } else {
                    drop(game);
                    self.resolve_conflict(&game_id, &mv).await;
                }
            }
        }
    }

    async fn resolve_conflict(&self, game_id: &str, mv: &Move) {
        if let Some(game_lock) = self.games.get(game_id) {
            let mut game = game_lock.write().await;

            use cotuong_core::logic::rules::is_valid_move;

            let from = if let Some(c) = cotuong_core::logic::board::BoardCoordinate::new(
                mv.from_row as usize,
                mv.from_col as usize,
            ) {
                c
            } else {
                return;
            };

            let to = if let Some(c) = cotuong_core::logic::board::BoardCoordinate::new(
                mv.to_row as usize,
                mv.to_col as usize,
            ) {
                c
            } else {
                return;
            };

            let is_legal = is_valid_move(&game.board, from, to, game.turn).is_ok();

            let true_fen: String;
            let true_turn: Color;

            let current_turn = game.turn;

            if is_legal {
                game.board.apply_move(mv, current_turn);
                game.turn = current_turn.opposite();
                true_turn = game.turn;
                true_fen = game.board.to_fen_string(true_turn);
            } else {
                true_turn = current_turn;
                true_fen = game.board.to_fen_string(true_turn);
            }

            game.pending_move = None;

            let msg = ServerMessage::GameStateCorrection {
                fen: true_fen,
                turn: true_turn,
            };

            let board_snapshot = game.board.clone();
            let turn_snapshot = game.turn;
            let has_moves = has_any_valid_move(&board_snapshot, turn_snapshot);

            let end_data = if !has_moves {
                let winner = if game.turn == Color::Red {
                    Color::Black
                } else {
                    Color::Red
                };
                game.game_ended = true;
                Some(winner)
            } else {
                None
            };

            let red_id = game.red_player.clone();
            let black_id = game.black_player.clone();

            drop(game);

            if let Some(p) = self.players.get(&red_id) {
                let _ = p.tx.send(msg.clone());
            }
            if let Some(p) = self.players.get(&black_id) {
                let _ = p.tx.send(msg);
            }

            if let Some(winner) = end_data {
                self.notify_game_end(game_id, winner, "Checkmate".to_string())
                    .await;
            }
        }
    }

    pub async fn notify_game_end(&self, game_id: &str, winner: Color, reason: String) {
        if let Some(game_lock) = self.games.get(game_id) {
            let game = game_lock.read().await;
            let msg = ServerMessage::GameEnd {
                winner: Some(winner),
                reason,
            };
            if let Some(p) = self.players.get(&game.red_player) {
                let _ = p.tx.send(msg.clone());
            }
            if let Some(p) = self.players.get(&game.black_player) {
                let _ = p.tx.send(msg);
            }
        }
    }
}
