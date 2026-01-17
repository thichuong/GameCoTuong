use cotuong_core::{
    engine::Move,
    logic::board::{Board, Color},
};
use shared::ServerMessage;
use std::collections::{HashMap, HashSet};
use tokio::sync::mpsc;
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

pub struct GameManager {
    pub players: HashMap<String, Player>,
    pub matchmaking_queue: HashSet<String>,
    pub games: HashMap<String, GameSession>,
    pub player_to_game: HashMap<String, String>,
}

impl GameManager {
    pub fn new() -> Self {
        Self {
            players: HashMap::new(),
            matchmaking_queue: HashSet::new(),
            games: HashMap::new(),
            player_to_game: HashMap::new(),
        }
    }

    pub fn add_player(&mut self, id: String, tx: Tx) {
        self.players.insert(id, Player { tx });
    }

    pub fn remove_player(&mut self, id: &str) {
        self.players.remove(id);
        self.matchmaking_queue.remove(id);

        // Handle disconnect during game
        if let Some(game_id) = self.player_to_game.remove(id) {
            if let Some(game) = self.games.remove(&game_id) {
                let opponent_id = if game.red_player == id {
                    game.black_player.clone()
                } else {
                    game.red_player.clone()
                };

                // Notify opponent
                if let Some(player) = self.players.get(&opponent_id) {
                    let _ = player.tx.send(ServerMessage::OpponentDisconnected);
                    let _ = player.tx.send(ServerMessage::GameEnd {
                        winner: Some(if game.red_player == id {
                            Color::Black
                        } else {
                            Color::Red
                        }),
                        reason: "Opponent Disconnected".to_string(),
                    });
                }
                self.player_to_game.remove(&opponent_id);
            }
        }
    }

    pub fn find_match(&mut self, player_id: String) {
        if self.matchmaking_queue.contains(&player_id) {
            return;
        }

        // Check if there is someone in the queue
        if let Some(opponent_id) = self.matchmaking_queue.iter().next().cloned() {
            // Remove opponent from queue
            self.matchmaking_queue.remove(&opponent_id);

            self.start_game(player_id, opponent_id);
        } else {
            // Add self to queue
            self.matchmaking_queue.insert(player_id.clone());
            // Removed redundant notification logic
            if let Some(player) = self.players.get(&player_id) {
                // Re-borrow to send message
                let _ = player.tx.send(ServerMessage::WaitingForMatch);
            }
        }
    }

    fn start_game(&mut self, p1_id: String, p2_id: String) {
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

        self.games.insert(game_id.clone(), game);
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

    pub fn handle_move(&mut self, player_id: String, mv: Move, fen: String) {
        if let Some(game_id) = self.player_to_game.get(&player_id).cloned() {
            if let Some(game) = self.games.get_mut(&game_id) {
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

                // Notify opponent of move (Relay)
                if let Some(p) = self.players.get(&opponent_id) {
                    let _ =
                        p.tx.send(ServerMessage::OpponentMove { move_data: mv, fen });
                }
            }
        }
    }

    pub fn handle_verify_move(&mut self, player_id: String, _fen: String, is_valid: bool) {
        let game_id = if let Some(gid) = self.player_to_game.get(&player_id) {
            gid.clone()
        } else {
            return;
        };

        let (pending_data, red_id, black_id) = {
            if let Some(game) = self.games.get(&game_id) {
                (
                    game.pending_move.clone(),
                    game.red_player.clone(),
                    game.black_player.clone(),
                )
            } else {
                return;
            }
        };

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
                if let Ok((new_board, new_turn)) = Board::from_fen(&claimed_fen) {
                    let (game_ended, winner) = {
                        let game = self.games.get_mut(&game_id).unwrap();
                        game.board = new_board;
                        game.turn = new_turn;
                        game.pending_move = None;

                        // Check Checkmate/Stalemate
                        if !has_any_valid_move(&game.board, game.turn) {
                            let winner = if game.turn == Color::Red {
                                Color::Black
                            } else {
                                Color::Red
                            };
                            game.game_ended = true;
                            (true, Some(winner))
                        } else {
                            (false, None)
                        }
                    };

                    if game_ended {
                        if let Some(w) = winner {
                            self.notify_game_end(&game_id, w, "Checkmate".to_string());
                        }
                    }
                } else {
                    self.resolve_conflict(&game_id, &mv);
                }
            } else {
                // 2. Conflict Path: Mismatch
                self.resolve_conflict(&game_id, &mv);
            }
        }
    }

    fn resolve_conflict(&mut self, game_id: &str, mv: &Move) {
        let (msg, end_data) = {
            if let Some(game) = self.games.get_mut(game_id) {
                // Apply logic strictly on current server board
                use cotuong_core::logic::rules::is_valid_move;

                let from = if let Some(c) = cotuong_core::logic::board::BoardCoordinate::new(
                    mv.from_row as usize,
                    mv.from_col as usize,
                ) {
                    c
                } else {
                    // Invalid coords ? restore current
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

                if is_legal {
                    game.board.apply_move(mv, game.turn);
                    game.turn = game.turn.opposite();
                    true_turn = game.turn;
                    true_fen = game.board.to_fen_string(game.turn);
                } else {
                    // Illegal move. Revert to current (Pre-move).
                    true_turn = game.turn;
                    true_fen = game.board.to_fen_string(game.turn);
                }

                game.pending_move = None;

                let msg = ServerMessage::GameStateCorrection {
                    fen: true_fen,
                    turn: true_turn,
                };

                let end_data = if !has_any_valid_move(&game.board, game.turn) {
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

                (Some(msg), end_data)
            } else {
                (None, None)
            }
        };

        if let Some(message) = msg {
            if let Some(game) = self.games.get(game_id) {
                let red_id = game.red_player.clone();
                let black_id = game.black_player.clone();
                if let Some(p) = self.players.get(&red_id) {
                    let _ = p.tx.send(message.clone());
                }
                if let Some(p) = self.players.get(&black_id) {
                    let _ = p.tx.send(message);
                }
            }
        }

        if let Some(winner) = end_data {
            self.notify_game_end(game_id, winner, "Checkmate".to_string());
        }
    }

    fn notify_game_end(&mut self, game_id: &str, winner: Color, reason: String) {
        if let Some(game) = self.games.get(game_id) {
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

    pub fn handle_surrender(&mut self, player_id: String) {
        if let Some(game_id) = self.player_to_game.get(&player_id).cloned() {
            if let Some(game) = self.games.get_mut(&game_id) {
                if game.game_ended {
                    return;
                }

                game.game_ended = true;
                let is_red = game.red_player == player_id;
                let winner = if is_red { Color::Black } else { Color::Red };

                // Notify both players
                if let Some(p) = self.players.get(&game.red_player) {
                    let _ = p.tx.send(ServerMessage::GameEnd {
                        winner: Some(winner),
                        reason: "Surrender".to_string(),
                    });
                }
                if let Some(p) = self.players.get(&game.black_player) {
                    let _ = p.tx.send(ServerMessage::GameEnd {
                        winner: Some(winner),
                        reason: "Surrender".to_string(),
                    });
                }
            }
        }
    }

    pub fn handle_play_again(&mut self, player_id: String) {
        if let Some(game_id) = self.player_to_game.get(&player_id).cloned() {
            if let Some(game) = self.games.get_mut(&game_id) {
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

    pub fn handle_player_left(&mut self, player_id: String) {
        if let Some(game_id) = self.player_to_game.remove(&player_id) {
            if let Some(game) = self.games.remove(&game_id) {
                let opponent_id = if game.red_player == player_id {
                    game.black_player.clone()
                } else {
                    game.red_player.clone()
                };

                // Remove opponent mapping as well since game is gone
                self.player_to_game.remove(&opponent_id);

                // Notify opponent
                if let Some(player) = self.players.get(&opponent_id) {
                    let _ = player.tx.send(ServerMessage::OpponentDisconnected);
                }
            }
        } else {
            // Player might be in matchmaking queue
            self.matchmaking_queue.remove(&player_id);
        }
    }
}

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
        let mut gm = GameManager::new();
        let (tx1, mut rx1) = mpsc::unbounded_channel();
        let (tx2, mut rx2) = mpsc::unbounded_channel();

        let p1_id = "p1".to_string();
        let p2_id = "p2".to_string();

        gm.add_player(p1_id.clone(), tx1);
        gm.add_player(p2_id.clone(), tx2);

        // Matchmake
        gm.find_match(p1_id.clone());
        gm.find_match(p2_id.clone());

        // Drain setup
        drain_setup_messages(&mut rx1).await;
        drain_setup_messages(&mut rx2).await;

        let game_id = gm
            .player_to_game
            .get(&p1_id)
            .clone()
            .expect("Game should exist")
            .clone();
        let game = gm.games.get(&game_id).expect("Game session missing");
        let red_id = game.red_player.clone();
        let is_p1_red = red_id == p1_id;

        // Identify Black
        let black_id = if is_p1_red {
            p2_id.clone()
        } else {
            p1_id.clone()
        };

        // Generate valid move (Red)
        let board = Board::new();
        let mut gen = cotuong_core::logic::generator::MoveGenerator::new();
        let moves = gen.generate_moves(&board, Color::Red);
        let valid_move = moves.first().expect("Should have moves").clone();

        // Calculate expected FEN
        let mut test_board = board.clone();
        test_board.apply_move(&valid_move, Color::Red);
        let expected_fen = test_board.to_fen_string(Color::Black);

        // P1 sends MakeMove
        gm.handle_move(red_id.clone(), valid_move.clone(), expected_fen.clone());

        // Verify Pending
        {
            let game = gm.games.get(&game_id).unwrap();
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
        gm.handle_verify_move(black_id.clone(), expected_fen.clone(), true);

        // Verify Server State Updated
        {
            let game = gm.games.get(&game_id).unwrap();
            assert!(game.pending_move.is_none());
            assert_eq!(game.turn, Color::Black);
            assert_eq!(game.board.to_fen_string(Color::Black), expected_fen);
        }
    }

    #[tokio::test]
    async fn test_conflict_resolution() {
        let mut gm = GameManager::new();
        let (tx1, mut rx1) = mpsc::unbounded_channel();
        let (tx2, mut rx2) = mpsc::unbounded_channel();

        let p1_id = "p1".to_string();
        let p2_id = "p2".to_string();

        gm.add_player(p1_id.clone(), tx1);
        gm.add_player(p2_id.clone(), tx2);

        gm.find_match(p1_id.clone());
        gm.find_match(p2_id.clone());

        drain_setup_messages(&mut rx1).await;
        drain_setup_messages(&mut rx2).await;

        let game_id = gm.player_to_game.get(&p1_id).unwrap().clone();
        let game = gm.games.get(&game_id).unwrap();
        let red_id = game.red_player.clone();
        let is_p1_red = red_id == p1_id;
        let black_id = if is_p1_red {
            p2_id.clone()
        } else {
            p1_id.clone()
        };

        // Valid Move
        let board = Board::new();
        let mut gen = cotuong_core::logic::generator::MoveGenerator::new();
        let moves = gen.generate_moves(&board, Color::Red);
        let valid_move = moves.first().unwrap().clone();

        // Correct FEN
        let mut test_board = board.clone();
        test_board.apply_move(&valid_move, Color::Red);
        let valid_fen = test_board.to_fen_string(Color::Black);

        // Incorrect FEN (simulated malicious/buggy client)
        let initial_fen = board.to_fen_string(Color::Red);

        // P1 sends VALID move but claims INITIAL FEN (invalid state transition logic)
        gm.handle_move(red_id.clone(), valid_move.clone(), initial_fen.clone());

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
        gm.handle_verify_move(black_id.clone(), valid_fen.clone(), false);

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
            let game = gm.games.get(&game_id).unwrap();
            assert!(game.pending_move.is_none());
            // Since move was valid, server corrected state to valid_fen
            assert_eq!(game.board.to_fen_string(Color::Black), valid_fen);
        }
    }
}
