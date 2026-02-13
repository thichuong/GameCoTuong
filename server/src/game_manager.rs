use cotuong_core::{
    engine::Move,
    logic::board::{Board, Color},
};
use dashmap::DashMap;
use shared::ServerMessage;
use std::collections::HashSet;
use tokio::sync::{mpsc, Mutex, RwLock};
use uuid::Uuid;

type Tx = mpsc::UnboundedSender<ServerMessage>;

// Helper function to check if a player has any valid moves
fn has_any_valid_move(board: &Board, color: Color) -> bool {
    use cotuong_core::logic::generator::MoveGenerator;
    let generator = MoveGenerator::new();
    let moves = generator.generate_moves(board, color);
    !moves.is_empty()
}

pub struct Player {
    pub tx: Tx,
}

pub struct GameSession {
    pub red_player: String,
    pub black_player: String,
    pub board: Board,
    pub turn: Color,
    pub game_ended: bool,
    pub red_ready_for_rematch: bool,
    pub black_ready_for_rematch: bool,
    pub pending_move: Option<(String, Move, String)>,
}

pub struct AppState {
    pub players: DashMap<String, Player>,
    pub games: DashMap<String, RwLock<GameSession>>,
    pub player_to_game: DashMap<String, String>,
    pub matchmaking_queue: Mutex<HashSet<String>>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            players: DashMap::new(),
            games: DashMap::new(),
            player_to_game: DashMap::new(),
            matchmaking_queue: Mutex::new(HashSet::new()),
        }
    }

    pub fn add_player(&self, id: String, tx: Tx) {
        self.players.insert(id, Player { tx });
    }

    pub async fn remove_player(&self, id: &str) {
        self.players.remove(id);

        // Remove from matchmaking queue
        {
            let mut queue = self.matchmaking_queue.lock().await;
            queue.remove(id);
        }

        // Handle disconnect during game
        if let Some((_, game_id)) = self.player_to_game.remove(id) {
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

                drop(game); // Release lock

                // Notify opponent
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

    pub async fn find_match(&self, player_id: String) {
        // Reject if player is still in an active game
        if self.player_to_game.contains_key(&player_id) {
            return;
        }

        let mut queue = self.matchmaking_queue.lock().await;

        if queue.contains(&player_id) {
            return;
        }

        // Check if there is someone in the queue
        // We need to find an opponent that is NOT us (just in case)
        // But since we checked contains, and Queue is Set, any item in queue is valid opponent?
        // Wait, `iter().next()` gives ANY item.
        // If queue is empty -> None.
        // If queue has items -> Some(opponent).

        // We need to pop form queue.
        // Since HashSet doesn't support pop easily, we clone an item and remove it.
        let opponent_opt = queue.iter().next().cloned();

        if let Some(opponent_id) = opponent_opt {
            queue.remove(&opponent_id);
            drop(queue); // Release lock before starting game to avoid holding it during game creation logic
            self.start_game(player_id, opponent_id).await;
        } else {
            queue.insert(player_id.clone());
            drop(queue);

            // Removed redundant notification logic
            if let Some(player) = self.players.get(&player_id) {
                let _ = player.tx.send(ServerMessage::WaitingForMatch);
            }
        }
    }

    async fn start_game(&self, p1_id: String, p2_id: String) {
        let game_id = Uuid::new_v4().to_string();

        // Randomize colors
        let (red_id, black_id) = if rand::random() {
            (p1_id.clone(), p2_id.clone())
        } else {
            (p2_id.clone(), p1_id.clone())
        };

        let game = GameSession {
            red_player: red_id.clone(),
            black_player: black_id.clone(),
            board: Board::new(),
            turn: Color::Red,
            game_ended: false,
            red_ready_for_rematch: false,
            black_ready_for_rematch: false,
            pending_move: None,
        };

        self.games.insert(game_id.clone(), RwLock::new(game));
        self.player_to_game.insert(p1_id.clone(), game_id.clone());
        self.player_to_game.insert(p2_id.clone(), game_id.clone());

        // Notify players
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

            // Check valid turn
            let is_red = game.red_player == player_id;
            let player_color = if is_red { Color::Red } else { Color::Black };

            if game.turn != player_color {
                // Not your turn
                return;
            }

            // Store pending move (Optimistic Relay)
            game.pending_move = Some((player_id.clone(), mv, fen.clone()));

            let opponent_id = if is_red {
                game.black_player.clone()
            } else {
                game.red_player.clone()
            };

            // Release lock early if possible? No, we need pending_move set.
            drop(game);

            // Notify opponent of move (Relay)
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

        // We need to read game state first to check if verify is valid.
        // But we need write lock to update state.
        // Let's take write lock directly since verify usually leads to update.

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
                    // 1. Happy Path: Verification Success
                    // Perform Board::from_fen OUTSIDE lock? No, trivial.
                    if let Ok((new_board, new_turn)) = Board::from_fen(&claimed_fen) {
                        game.board = new_board;
                        game.turn = new_turn;
                        game.pending_move = None;

                        // Check Checkmate/Stalemate - CPU INTENSIVE
                        // We are holding WRITE LOCK here!
                        // Optimization: snapshot board and turn, release lock, calculate, then re-acquire?
                        // Or just calculate quickly. has_any_valid_move is somewhat expensive (MoveGen).
                        // Plan said: Move heavy calculation outside.

                        let board_snapshot = game.board.clone();
                        let turn_snapshot = game.turn;

                        // Use a flag to indicate we need to check end game
                        // But we need to result to update game_ended.

                        // Let's do it inside for correctness first, then optimize if needed.
                        // Or create a separate task? No, standard logic.

                        // Correct approach: Calculate with snapshot?
                        // But we need to write back game_ended.

                        let has_moves = has_any_valid_move(&board_snapshot, turn_snapshot);
                        if !has_moves {
                            let winner = if game.turn == Color::Red {
                                Color::Black
                            } else {
                                Color::Red
                            };
                            game.game_ended = true;

                            drop(game); // Release lock

                            self.notify_game_end(&game_id, winner, "Checkmate".to_string())
                                .await;
                        } else {
                            // nothing
                        }
                    } else {
                        // This branch implies conflict even if is_valid=true (shouldn't happen if client is honest)
                        // We need to resolve conflict.
                        // Need to release lock and call resolve_conflict?
                        // resolve_conflict takes lock.
                        // So we must release lock before calling it.
                        drop(game);
                        self.resolve_conflict(&game_id, &mv).await;
                    }
                } else {
                    // 2. Conflict Path: Mismatch
                    drop(game);
                    self.resolve_conflict(&game_id, &mv).await;
                }
            }
        }
    }

    async fn resolve_conflict(&self, game_id: &str, mv: &Move) {
        if let Some(game_lock) = self.games.get(game_id) {
            let mut game = game_lock.write().await;

            // Apply logic strictly on current server board
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
                // Illegal move. Revert to current (Pre-move).
                true_turn = current_turn;
                true_fen = game.board.to_fen_string(true_turn);
            }

            game.pending_move = None;

            let msg = ServerMessage::GameStateCorrection {
                fen: true_fen,
                turn: true_turn,
            };

            // Check end game
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

            drop(game); // Release lock

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

    async fn notify_game_end(&self, game_id: &str, winner: Color, reason: String) {
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

    pub async fn handle_surrender(&self, player_id: String) {
        if let Some(game_id) = self.player_to_game.get(&player_id) {
            let game_id = game_id.value().clone();
            if let Some(game_lock) = self.games.get(&game_id) {
                let mut game = game_lock.write().await;
                if game.game_ended {
                    return;
                }

                game.game_ended = true;
                let is_red = game.red_player == player_id;
                let winner = if is_red { Color::Black } else { Color::Red };

                let red_id = game.red_player.clone();
                let black_id = game.black_player.clone();

                drop(game);

                // Notify both players
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
            if let Some(game_lock) = self.games.get(&game_id) {
                // We need to modify state
                let mut game = game_lock.write().await;

                // Set ready flag for this player
                let is_red = game.red_player == player_id;
                if is_red {
                    game.red_ready_for_rematch = true;
                } else {
                    game.black_ready_for_rematch = true;
                }

                // Check if both are ready
                if game.red_ready_for_rematch && game.black_ready_for_rematch {
                    // Start new game with same players
                    let red_id = game.red_player.clone();
                    let black_id = game.black_player.clone();

                    // Reset game state
                    game.board = Board::new();
                    game.turn = Color::Red;
                    game.game_ended = false;
                    game.red_ready_for_rematch = false;
                    game.black_ready_for_rematch = false;
                    game.pending_move = None;

                    drop(game); // Release lock

                    // Notify both players of new game
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

    /// Player leaves current game but stays connected.
    /// Unlike remove_player(), this preserves the WebSocket connection
    /// so the player can find a new match.
    pub async fn leave_game(&self, player_id: &str) {
        // Remove from matchmaking queue (in case they were searching)
        {
            let mut queue = self.matchmaking_queue.lock().await;
            queue.remove(player_id);
        }

        // Cleanup game session
        if let Some((_, game_id)) = self.player_to_game.remove(player_id) {
            // Remove game session first to avoid DashMap deadlock (get() holds Ref shard lock)
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

                // Remove opponent mapping
                self.player_to_game.remove(&opponent_id);

                // Notify opponent only if game wasn't already ended
                if !game_ended {
                    if let Some(player) = self.players.get(&opponent_id) {
                        let _ = player.tx.send(ServerMessage::OpponentDisconnected);
                        let _ = player.tx.send(ServerMessage::GameEnd {
                            winner: Some(winner),
                            reason: "Opponent Left".to_string(),
                        });
                    }
                }
            }
        }
    }

    pub async fn handle_player_left(&self, player_id: String) {
        self.leave_game(&player_id).await;
    }
}

// Tests section needs update because GameManager struct is gone and we use async
#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use tokio::sync::mpsc;

    // Helper to receive next message with timeout
    async fn expect_msg_timeout(rx: &mut mpsc::UnboundedReceiver<ServerMessage>) -> ServerMessage {
        tokio::time::timeout(Duration::from_millis(1500), rx.recv())
            .await
            .expect("Timed out waiting for message")
            .expect("Channel closed")
    }

    // Drain setup messages (MatchFound, GameStart, Waiting)
    async fn drain_setup_messages(rx: &mut mpsc::UnboundedReceiver<ServerMessage>) {
        loop {
            match tokio::time::timeout(Duration::from_millis(50), rx.recv()).await {
                Ok(Some(msg)) => match msg {
                    ServerMessage::GameStart(_) => break,
                    _ => continue,
                },
                _ => break,
            }
        }
    }

    #[tokio::test]
    async fn test_happy_path_distributed_validation() {
        let app_state = AppState::new();
        let (tx1, mut rx1) = mpsc::unbounded_channel();
        let (tx2, mut rx2) = mpsc::unbounded_channel();

        let p1_id = "p1".to_string();
        let p2_id = "p2".to_string();

        app_state.add_player(p1_id.clone(), tx1);
        app_state.add_player(p2_id.clone(), tx2);

        // Matchmake
        app_state.find_match(p1_id.clone()).await;
        app_state.find_match(p2_id.clone()).await;

        // Drain setup
        drain_setup_messages(&mut rx1).await;
        drain_setup_messages(&mut rx2).await;

        let game_id = app_state
            .player_to_game
            .get(&p1_id)
            .expect("Game should exist")
            .value()
            .clone();

        let game_lock = app_state.games.get(&game_id).expect("Game session missing");
        let game = game_lock.read().await;
        let red_id = game.red_player.clone();
        let is_p1_red = red_id == p1_id;
        drop(game); // Release lock

        // Identify Black
        let black_id = if is_p1_red {
            p2_id.clone()
        } else {
            p1_id.clone()
        };

        // Generate valid move (Red)
        let board = Board::new();
        let gen = cotuong_core::logic::generator::MoveGenerator::new();
        let moves = gen.generate_moves(&board, Color::Red);
        let valid_move = moves.first().expect("Should have moves").clone();

        // Calculate expected FEN
        let mut test_board = board.clone();
        test_board.apply_move(&valid_move, Color::Red);
        let expected_fen = test_board.to_fen_string(Color::Black);

        // P1 sends MakeMove
        app_state
            .handle_move(red_id.clone(), valid_move.clone(), expected_fen.clone())
            .await;

        // Verify Pending
        {
            let game_lock = app_state.games.get(&game_id).unwrap();
            let game = game_lock.read().await;
            assert!(game.pending_move.is_some());
            if let Some((pid, m, f)) = &game.pending_move {
                assert_eq!(pid, &red_id);
                assert_eq!(m.from_row, valid_move.from_row);
                assert_eq!(f, &expected_fen);
            }
        }

        // Opponent (Black) should receive OpponentMove
        let mut opponent_rx = if is_p1_red { &mut rx2 } else { &mut rx1 };

        match expect_msg_timeout(&mut opponent_rx).await {
            ServerMessage::OpponentMove { move_data, fen } => {
                assert_eq!(move_data.from_row, valid_move.from_row);
                assert_eq!(fen, expected_fen);
            }
            // Ignore other messages if any (like Waiting)
            other => {
                // If we got something else, try one more time (maybe race on setup messages?)
                match expect_msg_timeout(&mut opponent_rx).await {
                    ServerMessage::OpponentMove { move_data, fen } => {
                        assert_eq!(move_data.from_row, valid_move.from_row);
                        assert_eq!(fen, expected_fen);
                    }
                    _ => panic!("Expected OpponentMove, got {:?}", other),
                }
            }
        }

        // Opponent Verifies (TRUE)
        app_state
            .handle_verify_move(black_id.clone(), expected_fen.clone(), true)
            .await;

        // Verify Server State Updated
        {
            let game_lock = app_state.games.get(&game_id).unwrap();
            let game = game_lock.read().await;
            assert!(game.pending_move.is_none());
            assert_eq!(game.turn, Color::Black);
            assert_eq!(game.board.to_fen_string(Color::Black), expected_fen);
        }
    }

    #[tokio::test]
    async fn test_conflict_resolution() {
        let app_state = AppState::new();
        let (tx1, mut rx1) = mpsc::unbounded_channel();
        let (tx2, mut rx2) = mpsc::unbounded_channel();

        let p1_id = "p1".to_string();
        let p2_id = "p2".to_string();

        app_state.add_player(p1_id.clone(), tx1);
        app_state.add_player(p2_id.clone(), tx2);

        app_state.find_match(p1_id.clone()).await;
        app_state.find_match(p2_id.clone()).await;

        drain_setup_messages(&mut rx1).await;
        drain_setup_messages(&mut rx2).await;

        let game_id = app_state
            .player_to_game
            .get(&p1_id)
            .unwrap()
            .value()
            .clone();
        let game_lock = app_state.games.get(&game_id).unwrap();
        let game = game_lock.read().await;
        let red_id = game.red_player.clone();
        let is_p1_red = red_id == p1_id;
        let black_id = if is_p1_red {
            p2_id.clone()
        } else {
            p1_id.clone()
        };
        drop(game);

        // Valid Move
        let board = Board::new();
        let gen = cotuong_core::logic::generator::MoveGenerator::new();
        let moves = gen.generate_moves(&board, Color::Red);
        let valid_move = moves.first().unwrap().clone();

        // Correct FEN
        let mut test_board = board.clone();
        test_board.apply_move(&valid_move, Color::Red);
        let valid_fen = test_board.to_fen_string(Color::Black);

        // Incorrect FEN (simulated malicious/buggy client)
        let initial_fen = board.to_fen_string(Color::Red);

        // P1 sends VALID move but claims INITIAL FEN (invalid state transition logic)
        app_state
            .handle_move(red_id.clone(), valid_move.clone(), initial_fen.clone())
            .await;

        // Destructure to avoid borrow checker confusion
        let (p1_rx, p2_rx) = if is_p1_red {
            (&mut rx1, &mut rx2)
        } else {
            (&mut rx2, &mut rx1)
        };
        // p1_rx is Mover, p2_rx is Opponent

        // Ensure P2 gets relay (might skip other messages)
        loop {
            let msg = expect_msg_timeout(&mut *p2_rx).await;
            match msg {
                ServerMessage::OpponentMove { fen, .. } => {
                    assert_eq!(fen, initial_fen);
                    break;
                }
                _ => continue,
            }
        }

        // P2 reports CONFLICT (false) because they calc valid_fen != initial_fen
        app_state
            .handle_verify_move(black_id.clone(), valid_fen.clone(), false)
            .await;

        // Check P1 (Red) receives correction
        // Might need loop if other messages queued
        loop {
            let msg = expect_msg_timeout(&mut *p1_rx).await;
            match msg {
                ServerMessage::GameStateCorrection { fen, turn } => {
                    assert_eq!(fen, valid_fen);
                    assert_eq!(turn, Color::Black);
                    break;
                }
                _ => continue,
            }
        }

        // Check P2 (Black) receives correction
        loop {
            let msg = expect_msg_timeout(&mut *p2_rx).await;
            match msg {
                ServerMessage::GameStateCorrection { fen, turn } => {
                    assert_eq!(fen, valid_fen);
                    assert_eq!(turn, Color::Black);
                    break;
                }
                _ => continue,
            }
        }

        // Verify Server State Updated
        {
            let game_lock = app_state.games.get(&game_id).unwrap();
            let game = game_lock.read().await;
            assert!(game.pending_move.is_none());
            // Since move was valid, server corrected state to valid_fen
            assert_eq!(game.board.to_fen_string(Color::Black), valid_fen);
        }
    }
}
