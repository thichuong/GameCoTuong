#![no_main]

use cotuong_core::engine::config::EngineConfig;
use cotuong_core::logic::game::GameState;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        let _ = serde_json::from_str::<EngineConfig>(s);
        if let Ok(mut state) = serde_json::from_str::<GameState>(s) {
            state.board.refresh_nnue();
        }
    }
});
