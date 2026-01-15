use cotuong_core::{
    engine::Move,
    logic::board::{Board, Color},
};
use shared::ServerMessage;
use std::collections::{HashMap, HashSet};
use tokio::sync::mpsc;
use uuid::Uuid;

type Tx = mpsc::UnboundedSender<ServerMessage>;

pub struct Player {
    pub id: String,
    pub tx: Tx,
}

pub struct GameSession {
    pub game_id: String,
    pub red_player: String,
    pub black_player: String,
    pub board: Board,
    pub turn: Color,
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
        self.players.insert(id.clone(), Player { id, tx });
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
        if let Some(opponent_id) = self.matchmaking_queue.iter().cloned().next() {
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
            game_id: game_id.clone(),
            red_player: red_id.clone(),
            black_player: black_id.clone(),
            board: Board::new(),
            turn: Color::Red,
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
            let _ = p.tx.send(ServerMessage::GameStart(Board::new()));
        }

        if let Some(p) = self.players.get(&black_id) {
            let _ = p.tx.send(ServerMessage::MatchFound {
                opponent_id: red_id.clone(),
                your_color: Color::Black,
                game_id: game_id.clone(),
            });
            let _ = p.tx.send(ServerMessage::GameStart(Board::new()));
        }
    }

    pub fn handle_move(&mut self, player_id: String, mv: Move) {
        if let Some(game_id) = self.player_to_game.get(&player_id) {
            if let Some(game) = self.games.get_mut(game_id) {
                // Check valid turn
                let is_red = game.red_player == player_id;
                let player_color = if is_red { Color::Red } else { Color::Black };

                if game.turn != player_color {
                    // Not your turn
                    return;
                }

                // Apply move (Check validity logic should be here or in Board)
                // For now assume valid or client checks
                // Ideally server checks. Board::new() has apply_move logic.

                // Using core logic to apply move
                game.board.apply_move(&mv, game.turn);
                game.turn = game.turn.opposite();

                let opponent_id = if is_red {
                    game.black_player.clone()
                } else {
                    game.red_player.clone()
                };

                // Notify opponent
                if let Some(p) = self.players.get(&opponent_id) {
                    let _ = p.tx.send(ServerMessage::OpponentMove(mv));
                }
            }
        }
    }
}
