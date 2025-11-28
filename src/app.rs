use crate::components::board::BoardView;
use crate::engine::search::AlphaBetaEngine;
use crate::engine::Searcher;
use crate::logic::board::Color;
use crate::logic::game::{GameState, GameStatus};
use leptos::*;
use std::time::Duration;

#[component]
pub fn App() -> impl IntoView {
    let (game_state, set_game_state) = create_signal(GameState::new());

    // AI Move Effect
    create_effect(move |_| {
        let state = game_state.get();
        if state.turn == Color::Black && state.status == GameStatus::Playing {
            // Simple timeout to let UI update before AI thinks (simulating "thinking" time)
            set_timeout(
                move || {
                    let mut current_state = game_state.get();
                    // Double check turn in case of race conditions (though unlikely here)
                    if current_state.turn == Color::Black
                        && current_state.status == GameStatus::Playing
                    {
                        let mut engine = AlphaBetaEngine::new();
                        // Depth 3 is reasonable for a simple JS-thread engine
                        if let Some(mv) = engine.search(&current_state, 3) {
                            let _ = current_state.make_move(
                                mv.from_row,
                                mv.from_col,
                                mv.to_row,
                                mv.to_col,
                            );
                            set_game_state.set(current_state);
                        } else {
                            // No moves found? Should be handled by game status check, but just in case
                            leptos::logging::log!("AI has no moves!");
                        }
                    }
                },
                Duration::from_millis(100),
            );
        }
    });

    view! {
        <div class="game-container">
            <h1>"Cờ Tướng"</h1>
            <BoardView game_state=game_state set_game_state=set_game_state />
        </div>
    }
}
