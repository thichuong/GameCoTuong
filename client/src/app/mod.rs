
pub mod config;
pub mod controls;
pub mod export;
pub mod game_app;
pub mod log;
pub mod online;
pub mod styles;

pub use game_app::App;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Difficulty {
    Level1,
    Level2,
    Level3,
    Level4,
    Level5,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameMode {
    HumanVsComputer,
    ComputerVsComputer,
    HumanVsHuman,
    Online,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OnlineStatus {
    None,                 // Chưa bắt đầu
    Finding,              // Đang tìm trận
    MatchFound,           // Đã tìm thấy đối thủ
    Playing,              // Đang chơi
    OpponentDisconnected, // Đối thủ ngắt kết nối
    GameEnded,            // Trận đấu kết thúc
}
