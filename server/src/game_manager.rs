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

    pub fn handle_move(&mut self, player_id: String, mv: Move) {
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

                // Apply move
                game.board.apply_move(&mv, game.turn);
                game.turn = game.turn.opposite();

                let opponent_id = if is_red {
                    game.black_player.clone()
                } else {
                    game.red_player.clone()
                };

                // Notify opponent of move
                if let Some(p) = self.players.get(&opponent_id) {
                    let _ = p.tx.send(ServerMessage::OpponentMove(mv));
                }

                // Check for checkmate/stalemate after move
                if !has_any_valid_move(&game.board, game.turn) {
                    // The player whose turn it is has no valid moves = they lose
                    let winner = player_color; // The player who just moved wins
                    game.game_ended = true;

                    // Notify both players of game end
                    if let Some(p) = self.players.get(&game.red_player) {
                        let _ = p.tx.send(ServerMessage::GameEnd {
                            winner: Some(winner),
                            reason: "Checkmate".to_string(),
                        });
                    }
                    if let Some(p) = self.players.get(&game.black_player) {
                        let _ = p.tx.send(ServerMessage::GameEnd {
                            winner: Some(winner),
                            reason: "Checkmate".to_string(),
                        });
                    }
                }
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
