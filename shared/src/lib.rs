use cotuong_core::{
    engine::Move,
    logic::board::{Board, Color},
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GameMessage {
    FindMatch,
    CancelFindMatch,
    MakeMove { move_data: Move, fen: String },
    VerifyMove { fen: String, is_valid: bool },
    Surrender,
    RequestDraw,
    AcceptDraw,
    PlayAgain,
    PlayerLeft,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ServerMessage {
    MatchFound {
        opponent_id: String,
        your_color: Color,
        game_id: String,
    },
    GameStart(Box<Board>),
    OpponentMove {
        move_data: Move,
        fen: String,
    },
    GameStateCorrection {
        fen: String,
        turn: Color,
    },
    GameEnd {
        winner: Option<Color>,
        reason: String, // "Checkmate", "Surrender", "Draw", "Disconnect"
    },
    Error(String),
    WaitingForMatch,
    OpponentDisconnected,
    OpponentLeftGame,
}
