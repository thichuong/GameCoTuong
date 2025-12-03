use bevy::prelude::*;
use cotuong_core::logic::game::GameState;

#[derive(Resource, Default)]
pub struct AppConfig {
    #[allow(dead_code)]
    pub show_log: bool,
    #[allow(dead_code)]
    pub show_config: bool,
}

#[derive(Resource)]
pub struct GameStateWrapper(pub GameState);

impl Default for GameStateWrapper {
    fn default() -> Self {
        Self(GameState::new())
    }
}
