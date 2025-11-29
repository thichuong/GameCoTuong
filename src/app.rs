use crate::components::board::BoardView;
use crate::engine::search::AlphaBetaEngine;
use crate::engine::{SearchLimit, Searcher};
use crate::logic::board::Color;
use crate::logic::game::{GameState, GameStatus};
use leptos::{
    component, create_effect, create_signal, document, event_target_value, set_timeout, view,
    wasm_bindgen, web_sys, IntoView, SignalGet, SignalSet,
};
use std::fmt::Write;
use std::time::Duration;
use wasm_bindgen::JsCast;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Difficulty {
    Easy,
    Medium,
    Hard,
}

#[component]
#[allow(clippy::too_many_lines)]
pub fn App() -> impl IntoView {
    let (game_state, set_game_state) = create_signal(GameState::new());
    let (difficulty, set_difficulty) = create_signal(Difficulty::Easy);
    let (is_thinking, set_is_thinking) = create_signal(false);

    // AI Move Effect
    create_effect(move |_| {
        let state = game_state.get();
        let diff = difficulty.get();

        if state.turn == Color::Black && state.status == GameStatus::Playing {
            set_is_thinking.set(true);
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

                        // 1. Check Opening Book
                        {
                            use crate::logic::opening;
                            let fen = current_state.board.to_fen_string(current_state.turn);
                            web_sys::console::log_1(&format!("Current FEN: {fen}").into());
                            let book_move =
                                opening::get_book_move(&current_state.board, current_state.turn);

                            if let Some((from, to)) = book_move {
                                if current_state.make_move(from.0, from.1, to.0, to.1).is_ok() {
                                    web_sys::console::log_1(&"📖 Book Move played".into());
                                    if let Some(last) = current_state.history.last_mut() {
                                        last.note = Some("📖 Book Move".to_string());
                                    }
                                    set_game_state.set(current_state);
                                }
                            } else if let Some((mv, stats)) = engine.search(&current_state, limit) {
                                if current_state
                                    .make_move(mv.from_row, mv.from_col, mv.to_row, mv.to_col)
                                    .is_ok()
                                {
                                    #[allow(clippy::cast_precision_loss)]
                                    let time_s = stats.time_ms as f64 / 1000.0;
                                    web_sys::console::log_1(
                                        &format!(
                                            "🤖 Engine Move: Depth {}, Nodes {} ({:.1}s)",
                                            stats.depth, stats.nodes, time_s
                                        )
                                        .into(),
                                    );
                                    // Update the last move record with stats
                                    if let Some(last) = current_state.history.last_mut() {
                                        last.note = Some(format!(
                                            "🤖 Depth: {}, Nodes: {}, Time: {}ms",
                                            stats.depth, stats.nodes, stats.time_ms
                                        ));
                                    }
                                    set_game_state.set(current_state);
                                }
                            }
                        }
                    }
                    set_is_thinking.set(false);
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
            let _ = writeln!(csv, "{turn},{from},{to},{piece},{captured},{note}");
        }

        // Create download link
        if let Ok(blob) = web_sys::Blob::new_with_str_sequence(&js_sys::Array::of1(&csv.into())) {
            if let Ok(url) = web_sys::Url::create_object_url_with_blob(&blob) {
                if let Ok(a) = document().create_element("a") {
                    if let Ok(a) = a.dyn_into::<web_sys::HtmlAnchorElement>() {
                        a.set_href(&url);
                        a.set_download("xiangqi_game.csv");
                        a.click();
                        let _ = web_sys::Url::revoke_object_url(&url);
                    }
                }
            }
        }
    };

    view! {
        <div class="game-container" style="font-family: 'Segoe UI', Tahoma, Geneva, Verdana, sans-serif; min-height: 100vh; background-color: #222; color: #eee; display: flex; flex-direction: column; align-items: center;">
            <style>
                "
                .game-layout {
                    display: flex;
                    flex-direction: column-reverse; /* Mobile: Board top (2), Log bottom (1) */
                    align-items: center;
                    gap: 20px;
                    width: 100%;
                    padding: 20px;
                    box-sizing: border-box;
                }
                
                @media (min-width: 1100px) {
                    .game-layout {
                        flex-direction: row; /* Desktop: Log left (1), Board right (2) */
                        align-items: flex-start;
                        justify-content: center;
                    }
                }

                .log-panel {
                    width: 100%;
                    max-width: 500px; /* Wider on mobile */
                    height: 60vh; /* Dynamic height */
                    max-height: 600px;
                    background: #333;
                    border-radius: 8px;
                    box-shadow: 0 4px 6px rgba(0,0,0,0.3);
                    display: flex;
                    flex-direction: column;
                    border: 1px solid #444;
                }
                
                @media (min-width: 1100px) {
                    .log-panel {
                        width: 350px;
                        height: 72vh; /* Match board height roughly */
                    }
                }

                .log-header {
                    background: #444;
                    color: #f0d9b5;
                    padding: 15px;
                    font-weight: bold;
                    text-align: center;
                    border-bottom: 1px solid #555;
                    display: flex;
                    justify-content: space-between;
                    align-items: center;
                }

                .log-list {
                    flex: 1;
                    overflow-y: auto;
                    padding: 0;
                    margin: 0;
                    list-style: none;
                    scrollbar-width: thin;
                    scrollbar-color: #666 #333;
                }
                
                .log-list::-webkit-scrollbar {
                    width: 8px;
                }
                .log-list::-webkit-scrollbar-track {
                    background: #333;
                }
                .log-list::-webkit-scrollbar-thumb {
                    background-color: #666;
                    border-radius: 4px;
                }

                .log-item {
                    padding: 10px 15px;
                    border-bottom: 1px solid #444;
                    font-size: 14px;
                    display: flex;
                    flex-direction: column;
                    gap: 4px;
                }

                .log-item:nth-child(even) {
                    background-color: #3a3a3a;
                }
                
                .log-item:last-child {
                    border-left: 3px solid #f0d9b5;
                    background-color: #444;
                }

                .move-info {
                    display: flex;
                    justify-content: space-between;
                    font-weight: 500;
                }
                
                .ai-stats {
                    font-size: 0.85em;
                    color: #aaa;
                    font-family: monospace;
                }

                .controls-area {
                    display: flex;
                    gap: 10px;
                    align-items: center;
                    margin-bottom: 20px;
                }
                
                select, button {
                    padding: 8px 12px;
                    border-radius: 4px;
                    border: 1px solid #555;
                    background: #444;
                    color: #eee;
                    font-size: 14px;
                    cursor: pointer;
                }
                
                select:hover, button:hover {
                    background: #555;
                }

                .thinking-indicator {
                    display: flex;
                    align-items: center;
                    justify-content: center;
                    gap: 10px;
                    color: #a8e6cf;
                    font-weight: bold;
                    margin: 10px 0;
                    height: 24px;
                    animation: pulse 1.5s infinite;
                }

                @keyframes pulse {
                    0% { opacity: 0.6; }
                    50% { opacity: 1; }
                    100% { opacity: 0.6; }
                }
                "
            </style>

            <h1 style="margin: 20px 0; color: #f0d9b5; text-shadow: 0 2px 4px rgba(0,0,0,0.5); text-align: center;">"Cờ Tướng"</h1>

            <div class="controls-area">
                <div>
                    <label style="margin-right: 10px; color: #ccc;">"Độ khó: "</label>
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
                    >
                        <option value="Easy">"Dễ (0.5s)"</option>
                        <option value="Medium">"Trung bình (2s)"</option>
                        <option value="Hard">"Khó (5s)"</option>
                    </select>
                </div>
                <button on:click=export_csv>"Xuất CSV"</button>
            </div>

            {move || {
                let style = if is_thinking.get() {
                    "visibility: visible;"
                } else {
                    "visibility: hidden;"
                };
                view! {
                    <div class="thinking-indicator" style=style>
                        <span>"Máy đang nghĩ..."</span>
                        <div style="width: 10px; height: 10px; background: #a8e6cf; border-radius: 50%; display: inline-block;"></div>
                    </div>
                }
            }}

            <div class="game-layout">
                // Log Panel (Order 1 in HTML, but reversed on mobile to be bottom)
                <div class="log-panel">
                    <div class="log-header">
                        <span>"Lịch sử nước đi"</span>
                        <span style="font-size: 0.8em; opacity: 0.7;">{move || format!("{} moves", game_state.get().history.len())}</span>
                    </div>
                    <ul class="log-list">
                        {move || {
                            game_state.get().history.iter().enumerate().rev().map(|(i, record)| {
                                let turn_icon = if i % 2 == 0 { "🔴" } else { "⚫" };
                                let turn_text = if i % 2 == 0 { "Red" } else { "Black" };
                                let note = record.note.clone().unwrap_or_default();

                                // Format coordinates to be more readable (e.g. A1, B2 style or just standard)
                                // Here we stick to (row, col) but maybe 1-based for user friendliness?
                                // Let's keep 0-8, 0-9 for now but make it clear.
                                // Actually, Xiangqi notation is complex. Let's stick to coordinates but make them clear.
                                let from_str = format!("({}, {})", record.from.0, record.from.1);
                                let to_str = format!("({}, {})", record.to.0, record.to.1);

                                view! {
                                    <li class="log-item">
                                        <div class="move-info">
                                            <span>{format!("{}. {} {}", i + 1, turn_icon, turn_text)}</span>
                                            <span>{format!("{from_str} ➝ {to_str}")}</span>
                                        </div>
                                        {if note.is_empty() {
                                            view! {}.into_view()
                                        } else {
                                            view! { <div class="ai-stats">{note}</div> }.into_view()
                                        }}
                                    </li>
                                }
                            }).collect::<Vec<_>>()
                        }}
                    </ul>
                </div>

                // Board View (Order 2 in HTML)
                <BoardView game_state=game_state set_game_state=set_game_state />
            </div>
        </div>
    }
}
