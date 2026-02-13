use cotuong_core::{
    engine::Move,
    logic::board::{Board, Color},
};
use shared::ServerMessage;
use tokio::sync::mpsc;

use std::time::Instant;

pub type Tx = mpsc::UnboundedSender<ServerMessage>;

pub struct Player {
    pub tx: Tx,
    pub last_msg_at: Instant,
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
    pub last_activity: Instant,
}

pub fn has_any_valid_move(board: &Board, color: Color) -> bool {
    use cotuong_core::logic::generator::MoveGenerator;
    let generator = MoveGenerator::new();
    let moves = generator.generate_moves(board, color);
    !moves.is_empty()
}
