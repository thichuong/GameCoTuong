use dashmap::DashMap;
use std::collections::HashSet;
use tokio::sync::{Mutex, RwLock};

pub mod lifecycle;
pub mod matchmaking;
pub mod move_handler;
pub mod session;
#[cfg(test)]
pub mod tests;

pub use session::{GameSession, Player, Tx};

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

    pub fn check_rate_limit(&self, player_id: &str) -> bool {
        use std::time::Instant;
        if let Some(mut player) = self.players.get_mut(player_id) {
            let now = Instant::now();
            let elapsed = now.duration_since(player.last_msg_at).as_secs_f32();
            if elapsed < 0.1 {
                // Allow max 10 messages per second
                return false;
            }
            player.last_msg_at = now;
            true
        } else {
            false
        }
    }
}
