use crate::components::board::BoardView;
use crate::engine::search::AlphaBetaEngine;
use crate::engine::{SearchLimit, Searcher};
use crate::logic::board::Color;
use crate::logic::game::{GameState, GameStatus};
use leptos::*;
use std::time::Duration;
use wasm_bindgen::JsCast;

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

                        if let Some((mv, stats)) = engine.search(&current_state, limit) {
                            if current_state
                                .make_move(mv.from_row, mv.from_col, mv.to_row, mv.to_col)
                                .is_ok()
                            {
                                // Update the last move record with stats
                                if let Some(last) = current_state.history.last_mut() {
                                    last.note = Some(format!(
                                        "Depth: {}, Nodes: {}, Time: {}ms",
                                        stats.depth, stats.nodes, stats.time_ms
                                    ));
                                }
                                set_game_state.set(current_state);
                            }
                        }
                    }
                },
                Duration::from_millis(100),
            );
        }
    });

    let export_csv = move |_| {
        let state = game_state.get();
        let mut csv = String::from("Turn,From,To,Piece,Captured,Note\n");
        for (i, record) in state.history.iter().enumerate() {
            let turn = if i % 2 == 0 { "Red" } else { "Black" };
            let from = format!("({},{})", record.from.0, record.from.1);
            let to = format!("({},{})", record.to.0, record.to.1);
            let piece = format!("{:?}", record.piece.piece_type);
            let captured = record
                .captured
                .map(|p| format!("{:?}", p.piece_type))
                .unwrap_or_default();
            let note = record.note.clone().unwrap_or_default();
            csv.push_str(&format!(
                "{},{},{},{},{},{}\n",
                turn, from, to, piece, captured, note
            ));
        }

        // Create download link
        let blob = web_sys::Blob::new_with_str_sequence(&js_sys::Array::of1(&csv.into()))
            .expect("Failed to create blob");
        let url = web_sys::Url::create_object_url_with_blob(&blob).expect("Failed to create URL");
        let a = document().create_element("a").expect("Failed to create a");
        let a = a
            .dyn_into::<web_sys::HtmlAnchorElement>()
            .expect("Failed to cast to HtmlAnchorElement");
        a.set_href(&url);
        a.set_download("xiangqi_game.csv");
        a.click();
        web_sys::Url::revoke_object_url(&url).expect("Failed to revoke URL");
    };

    view! {
        <div class="game-container" style="display: flex; flex-direction: column; align-items: center; font-family: sans-serif; max-width: 800px; margin: 0 auto;">
            <h1>"Cờ Tướng"</h1>

            <div class="controls" style="margin-bottom: 10px; display: flex; gap: 10px;">
                <div>
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
                <button on:click=export_csv style="padding: 5px 10px; font-size: 16px;">"Xuất CSV"</button>
            </div>

            <div style="display: flex; flex-wrap: wrap; justify-content: center; width: 100%; gap: 20px;">
                <BoardView game_state=game_state set_game_state=set_game_state />

                <div class="log-panel" style="
                    width: 100%; 
                    max-width: 400px; 
                    height: 500px; 
                    border: 1px solid #ccc; 
                    overflow-y: auto; 
                    padding: 10px; 
                    background: #f9f9f9;
                    font-family: monospace;
                ">
                    <h3>"Lịch sử nước đi"</h3>
                    <ul style="list-style: none; padding: 0;">
                        {move || {
                            game_state.get().history.iter().enumerate().map(|(i, record)| {
                                let turn = if i % 2 == 0 { "🔴" } else { "⚫" };
                                let note = record.note.clone().unwrap_or_default();
                                view! {
                                    <li style="border-bottom: 1px solid #eee; padding: 5px 0;">
                                        <div>
                                            {format!("{} {}: ({},{}) -> ({},{})",
                                                i + 1, turn, record.from.0, record.from.1, record.to.0, record.to.1
                                            )}
                                        </div>
                                        {if !note.is_empty() {
                                            view! { <div style="font-size: 0.8em; color: #666; margin-left: 20px;">{note}</div> }.into_view()
                                        } else {
                                            view! {}.into_view()
                                        }}
                                    </li>
                                }
                            }).collect::<Vec<_>>()
                        }}
                    </ul>
                </div>
            </div>
        </div>
    }
}
