use crate::components::board::BoardView;
use crate::engine::config::EngineConfig;
use crate::engine::search::AlphaBetaEngine;
use crate::engine::{SearchLimit, Searcher};
use crate::logic::board::Color;
use crate::logic::game::{GameState, GameStatus};
use leptos::{
    component, create_effect, create_signal, document, event_target_value, set_timeout, view,
    wasm_bindgen, web_sys, IntoView, SignalGet, SignalSet,
};
use std::fmt::Write;
use std::sync::Arc;
use std::time::Duration;
use wasm_bindgen::JsCast;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Difficulty {
    Easy,
    Medium,
    Hard,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GameMode {
    HumanVsComputer,
    ComputerVsComputer,
    HumanVsHuman,
}

#[component]
#[allow(clippy::too_many_lines)]
pub fn App() -> impl IntoView {
    let (game_state, set_game_state) = create_signal(GameState::new());
    let (difficulty, set_difficulty) = create_signal(Difficulty::Easy);
    let (game_mode, set_game_mode) = create_signal(GameMode::HumanVsComputer);
    let (is_thinking, set_is_thinking) = create_signal(false);
    let (is_paused, set_is_paused) = create_signal(false);

    // Dual Configs
    let (red_config, set_red_config) = create_signal(EngineConfig::default());
    let (black_config, set_black_config) = create_signal(EngineConfig::default());

    // AI Move Effect
    create_effect(move |_| {
        let state = game_state.get();
        let diff = difficulty.get();
        let mode = game_mode.get();
        let paused = is_paused.get();

        if paused {
            return;
        }

        let should_play = match mode {
            GameMode::HumanVsComputer => state.turn == Color::Black,
            GameMode::ComputerVsComputer => true,
            GameMode::HumanVsHuman => false,
        };

        if should_play && state.status == GameStatus::Playing {
            set_is_thinking.set(true);
            set_timeout(
                move || {
                    const MAX_RETRIES: usize = 5;
                    let mut current_state = game_state.get();
                    // Re-check condition inside timeout to avoid race conditions
                    let current_mode = game_mode.get();
                    let current_paused = is_paused.get();

                    if current_paused {
                        set_is_thinking.set(false);
                        return;
                    }

                    let should_play_now = match current_mode {
                        GameMode::HumanVsComputer => current_state.turn == Color::Black,
                        GameMode::ComputerVsComputer => true,
                        GameMode::HumanVsHuman => false,
                    };

                    if should_play_now && current_state.status == GameStatus::Playing {
                        // Select Config based on turn
                        let config = if current_state.turn == Color::Red {
                            Arc::new(red_config.get())
                        } else {
                            Arc::new(black_config.get())
                        };

                        let mut engine = AlphaBetaEngine::new(config);

                        let limit = match diff {
                            Difficulty::Easy => SearchLimit::Time(500),
                            Difficulty::Medium => SearchLimit::Time(2000),
                            Difficulty::Hard => SearchLimit::Time(5000),
                        };

                        // 1. Check Opening Book
                        {
                            use crate::logic::opening;
                            let _fen = current_state.board.to_fen_string(current_state.turn);
                            // web_sys::console::log_1(&format!("Current FEN: {fen}").into());
                            let book_move =
                                opening::get_book_move(&current_state.board, current_state.turn);

                            if let Some((from, to)) = book_move {
                                if current_state.make_move(from.0, from.1, to.0, to.1).is_ok() {
                                    // web_sys::console::log_1(&"📖 Book Move played".into());
                                    if let Some(last) = current_state.history.last_mut() {
                                        last.note = Some("📖 Book Move".to_string());
                                    }
                                    set_game_state.set(current_state);
                                    set_is_thinking.set(false);
                                    return;
                                }
                            }
                        }

                        // Retry Loop
                        let mut excluded_moves = Vec::new();
                        let mut loop_count = 0;

                        while loop_count < MAX_RETRIES {
                            loop_count += 1;

                            if let Some((mv, stats)) =
                                engine.search(&current_state, limit, &excluded_moves)
                            {
                                match current_state.make_move(
                                    mv.from_row,
                                    mv.from_col,
                                    mv.to_row,
                                    mv.to_col,
                                ) {
                                    Ok(()) => {
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
                                        break; // Success, exit loop
                                    }
                                    Err(e) => {
                                        if e == crate::logic::rules::MoveError::ThreeFoldRepetition
                                        {
                                            web_sys::console::log_1(
                                                &format!(
                                                    "⚠️ Move rejected (3-fold), retrying... {mv:?}"
                                                )
                                                .into(),
                                            );
                                            excluded_moves.push(mv);
                                            // Continue loop to search again
                                        } else {
                                            web_sys::console::log_1(
                                                &format!("❌ Move error: {e:?}").into(),
                                            );
                                            break; // Other error, stop
                                        }
                                    }
                                }
                            } else {
                                break; // No move found
                            }
                        }
                    }
                    set_is_thinking.set(false);
                },
                Duration::from_millis(100), // Small delay for UI update
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

    // Helper component for sliders
    let slider = |label: &'static str,
                  val: i32,
                  min: i32,
                  max: i32,
                  step: i32,
                  setter: Box<dyn Fn(i32)>| {
        view! {
            <div style="margin-bottom: 8px;">
                <div style="display: flex; justify-content: space-between; font-size: 0.9em; color: #ccc;">
                    <span>{label}</span>
                    <span>{val}</span>
                </div>
                <input
                    type="range"
                    min=min
                    max=max
                    step=step
                    value=val // This sets the initial value
                    prop:value=val // This ensures the DOM property is updated on re-renders
                    style="width: 100%; accent-color: #a8e6cf;"
                    on:input=move |ev| {
                        if let Ok(v) = event_target_value(&ev).parse::<i32>() {
                            setter(v);
                        }
                    }
                />
            </div>
        }
    };

    // Helper to create setters for config fields
    // We need to clone the config, update field, and set it back.
    // Since we can't easily pass generic field accessors, we'll just inline the logic in the view or create specific closures.

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
                    flex-wrap: wrap;
                    justify-content: center;
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

                .config-panel {
                    display: flex;
                    gap: 20px;
                    width: 100%;
                    max-width: 1000px;
                    margin-top: 20px;
                    margin-bottom: 20px;
                    flex-wrap: wrap;
                    justify-content: center;
                }

                .config-column {
                    flex: 1;
                    min-width: 300px;
                    background: #333;
                    padding: 15px;
                    border-radius: 8px;
                    border: 1px solid #444;
                }

                .config-title {
                    color: #f0d9b5;
                    font-weight: bold;
                    text-align: center;
                    margin-bottom: 15px;
                    border-bottom: 1px solid #555;
                    padding-bottom: 10px;
                }
                "
            </style>

            <h1 style="margin: 20px 0; color: #f0d9b5; text-shadow: 0 2px 4px rgba(0,0,0,0.5); text-align: center;">"Cờ Tướng"</h1>

            <div class="controls-area">
                <div>
                    <label style="margin-right: 10px; color: #ccc;">"Chế độ: "</label>
                    <select
                        on:change=move |ev| {
                            let val = event_target_value(&ev);
                            match val.as_str() {
                                "HumanVsComputer" => {
                                    set_game_mode.set(GameMode::HumanVsComputer);
                                    set_is_paused.set(false);
                                },
                                "ComputerVsComputer" => {
                                    set_game_mode.set(GameMode::ComputerVsComputer);
                                    set_is_paused.set(true); // Auto-pause on switch to CvC
                                },
                                "HumanVsHuman" => {
                                    set_game_mode.set(GameMode::HumanVsHuman);
                                    set_is_paused.set(false);
                                },
                                _ => {},
                            }
                        }
                    >
                        <option value="HumanVsComputer">"Người vs Máy"</option>
                        <option value="ComputerVsComputer">"Máy vs Máy"</option>
                        <option value="HumanVsHuman">"Người vs Người"</option>
                    </select>
                </div>
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

                {move || {
                    if game_mode.get() == GameMode::ComputerVsComputer {
                        if is_paused.get() {
                            view! { <button style="background: #4CAF50; color: white;" on:click=move |_| set_is_paused.set(false)>"▶ Bắt đầu"</button> }.into_view()
                        } else {
                            view! { <button style="background: #f44336; color: white;" on:click=move |_| set_is_paused.set(true)>"⏸ Tạm dừng"</button> }.into_view()
                        }
                    } else {
                        view! {}.into_view()
                    }
                }}

                <button style="background: #2196F3; color: white;" on:click=move |_| {
                    set_game_state.set(GameState::new());
                    set_is_thinking.set(false);
                    // If CvC, maybe pause? User preference. Let's keep current pause state or reset.
                    // Let's reset pause to true for CvC to avoid instant chaos.
                    if game_mode.get() == GameMode::ComputerVsComputer {
                        set_is_paused.set(true);
                    }
                }>"Chơi mới"</button>

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

            <div class="config-panel">
                <div class="config-column">
                    <div class="config-title" style="color: #ff6b6b;">"Cấu hình Đỏ (Red)"</div>
                    {
                        move || {
                            let config = red_config.get();
                            view! {
                                <div>
                                    {slider("Tốt (Pawn)", config.val_pawn, 0, 200, 10, Box::new(move |v| { let mut c = red_config.get(); c.val_pawn = v; set_red_config.set(c); }))}
                                    {slider("Sĩ (Advisor)", config.val_advisor, 0, 400, 10, Box::new(move |v| { let mut c = red_config.get(); c.val_advisor = v; set_red_config.set(c); }))}
                                    {slider("Tượng (Elephant)", config.val_elephant, 0, 400, 10, Box::new(move |v| { let mut c = red_config.get(); c.val_elephant = v; set_red_config.set(c); }))}
                                    {slider("Mã (Horse)", config.val_horse, 0, 800, 10, Box::new(move |v| { let mut c = red_config.get(); c.val_horse = v; set_red_config.set(c); }))}
                                    {slider("Pháo (Cannon)", config.val_cannon, 0, 900, 10, Box::new(move |v| { let mut c = red_config.get(); c.val_cannon = v; set_red_config.set(c); }))}
                                    {slider("Xe (Rook)", config.val_rook, 0, 1800, 10, Box::new(move |v| { let mut c = red_config.get(); c.val_rook = v; set_red_config.set(c); }))}
                                    {slider("Tướng (King)", config.val_king, 5000, 20000, 100, Box::new(move |v| { let mut c = red_config.get(); c.val_king = v; set_red_config.set(c); }))}
                                    <hr style="border-color: #444; margin: 10px 0;"/>
                                    {slider("Hash Move", config.score_hash_move, 0, 5_000_000, 100_000, Box::new(move |v| { let mut c = red_config.get(); c.score_hash_move = v; set_red_config.set(c); }))}
                                    {slider("Capture Base", config.score_capture_base, 0, 2_000_000, 100_000, Box::new(move |v| { let mut c = red_config.get(); c.score_capture_base = v; set_red_config.set(c); }))}
                                    {slider("Killer Move", config.score_killer_move, 0, 2_000_000, 100_000, Box::new(move |v| { let mut c = red_config.get(); c.score_killer_move = v; set_red_config.set(c); }))}
                                    {slider("History Max", config.score_history_max, 0, 2_000_000, 100_000, Box::new(move |v| { let mut c = red_config.get(); c.score_history_max = v; set_red_config.set(c); }))}
                                    {slider("Pruning Ratio (%)", config.pruning_discard_ratio, 0, 90, 5, Box::new(move |v| { let mut c = red_config.get(); c.pruning_discard_ratio = v; set_red_config.set(c); }))}
                                </div>
                            }
                        }
                    }
                </div>
                <div class="config-column">
                    <div class="config-title" style="color: #a8e6cf;">"Cấu hình Đen (Black)"</div>
                    {
                        move || {
                            let config = black_config.get();
                            view! {
                                <div>
                                    {slider("Tốt (Pawn)", config.val_pawn, 0, 200, 10, Box::new(move |v| { let mut c = black_config.get(); c.val_pawn = v; set_black_config.set(c); }))}
                                    {slider("Sĩ (Advisor)", config.val_advisor, 0, 400, 10, Box::new(move |v| { let mut c = black_config.get(); c.val_advisor = v; set_black_config.set(c); }))}
                                    {slider("Tượng (Elephant)", config.val_elephant, 0, 400, 10, Box::new(move |v| { let mut c = black_config.get(); c.val_elephant = v; set_black_config.set(c); }))}
                                    {slider("Mã (Horse)", config.val_horse, 0, 800, 10, Box::new(move |v| { let mut c = black_config.get(); c.val_horse = v; set_black_config.set(c); }))}
                                    {slider("Pháo (Cannon)", config.val_cannon, 0, 900, 10, Box::new(move |v| { let mut c = black_config.get(); c.val_cannon = v; set_black_config.set(c); }))}
                                    {slider("Xe (Rook)", config.val_rook, 0, 1800, 10, Box::new(move |v| { let mut c = black_config.get(); c.val_rook = v; set_black_config.set(c); }))}
                                    {slider("Tướng (King)", config.val_king, 5000, 20000, 100, Box::new(move |v| { let mut c = black_config.get(); c.val_king = v; set_black_config.set(c); }))}
                                    <hr style="border-color: #444; margin: 10px 0;"/>
                                    {slider("Hash Move", config.score_hash_move, 0, 5_000_000, 100_000, Box::new(move |v| { let mut c = black_config.get(); c.score_hash_move = v; set_black_config.set(c); }))}
                                    {slider("Capture Base", config.score_capture_base, 0, 2_000_000, 100_000, Box::new(move |v| { let mut c = black_config.get(); c.score_capture_base = v; set_black_config.set(c); }))}
                                    {slider("Killer Move", config.score_killer_move, 0, 2_000_000, 100_000, Box::new(move |v| { let mut c = black_config.get(); c.score_killer_move = v; set_black_config.set(c); }))}
                                    {slider("History Max", config.score_history_max, 0, 2_000_000, 100_000, Box::new(move |v| { let mut c = black_config.get(); c.score_history_max = v; set_black_config.set(c); }))}
                                    {slider("Pruning Ratio (%)", config.pruning_discard_ratio, 0, 90, 5, Box::new(move |v| { let mut c = black_config.get(); c.pruning_discard_ratio = v; set_black_config.set(c); }))}
                                </div>
                            }
                        }
                    }
                </div>
            </div>
        </div>
    }
}
