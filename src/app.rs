use crate::components::board::BoardView;
use crate::engine::search::AlphaBetaEngine;
use crate::engine::{SearchLimit, Searcher};
use crate::logic::board::Color;
use crate::logic::game::{GameState, GameStatus};
use leptos::*;
use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Difficulty {
    Easy,
    Medium,
    Hard,
}

#[component]
pub fn App() -> impl IntoView {
    let (game_state, set_game_state) = create_signal(GameState::new());
    let (difficulty, set_difficulty) = create_signal(Difficulty::Easy);

    // AI Move Effect
    create_effect(move |_| {
        let state = game_state.get();
        let diff = difficulty.get();

        if state.turn == Color::Black && state.status == GameStatus::Playing {
            set_timeout(
                move || {
                    let mut current_state = game_state.get();
                    if current_state.turn == Color::Black
                        && current_state.status == GameStatus::Playing
                    {
                        let mut engine = AlphaBetaEngine::new();

                        let limit = match diff {
                            Difficulty::Easy => SearchLimit::Time(500),
                            Difficulty::Medium => SearchLimit::Time(2000),
                            Difficulty::Hard => SearchLimit::Time(5000),
                        };

                        if let Some(mv) = engine.search(&current_state, limit) {
                            let _ = current_state.make_move(
                                mv.from_row,
                                mv.from_col,
                                mv.to_row,
                                mv.to_col,
                            );
                            set_game_state.set(current_state);
                        }
                    }
                },
                Duration::from_millis(100),
            );
        }
    });

    view! {
        <div class="game-container" style="display: flex; flex-direction: column; align-items: center; font-family: sans-serif;">
            <h1>"Cờ Tướng"</h1>

            <div class="controls" style="margin-bottom: 10px;">
                <label style="margin-right: 10px;">"Độ khó: "</label>
                <select
                    on:change=move |ev| {
                        let val = event_target_value(&ev);
                        match val.as_str() {
                            "Easy" => set_difficulty.set(Difficulty::Easy),
                            "Medium" => set_difficulty.set(Difficulty::Medium),
                            "Hard" => set_difficulty.set(Difficulty::Hard),
                            _ => {},
                        }
                    }
                    style="padding: 5px; font-size: 16px;"
                >
                    <option value="Easy">"Dễ (0.5s)"</option>
                    <option value="Medium">"Trung bình (2s)"</option>
                    <option value="Hard">"Khó (5s)"</option>
                </select>
            </div>

            <BoardView game_state=game_state set_game_state=set_game_state />
        </div>
    }
}
