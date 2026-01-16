use crate::components::board::BoardView;
use cotuong_core::engine::config::EngineConfig;
use cotuong_core::engine::Move;
use cotuong_core::engine::SearchLimit;
use cotuong_core::logic::board::{BoardCoordinate, Color, PieceType};
use cotuong_core::logic::game::{GameState, GameStatus};
use cotuong_core::logic::rules::is_in_check;
use cotuong_core::worker::{GameWorker, Input, Output};
use gloo_worker::{Spawnable, WorkerBridge};
use leptos::{
    component, create_effect, create_signal, document, event_target_value, set_timeout,
    store_value, view, wasm_bindgen, web_sys, IntoView, SignalGet, SignalSet, SignalUpdate,
    SignalWithUntracked, WriteSignal,
};
use std::rc::Rc;
use std::time::Duration;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;

use std::fmt::Write;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Difficulty {
    Level1,
    Level2,
    Level3,
    Level4,
    Level5,
}

use crate::network::NetworkClient;
use shared::{GameMessage, ServerMessage};

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

#[component]
#[allow(clippy::too_many_lines)]
pub fn App() -> impl IntoView {
    let (game_state, set_game_state) = create_signal(GameState::new());
    let (difficulty, set_difficulty) = create_signal(Difficulty::Level1);
    let (game_mode, set_game_mode) = create_signal(GameMode::HumanVsComputer);
    let (player_side, set_player_side) = create_signal(Color::Red);
    let (is_thinking, set_is_thinking) = create_signal(false);
    let (is_paused, set_is_paused) = create_signal(false);
    let (show_config, set_show_config) = create_signal(false);

    // Network State
    let (network_client, set_network_client) = create_signal(Option::<NetworkClient>::None);
    let (server_msg, set_server_msg) = create_signal(Option::<ServerMessage>::None);
    let (online_status, set_online_status) = create_signal(OnlineStatus::None);

    // Game End State
    #[allow(unused_variables)]
    let (game_end_winner, set_game_end_winner) = create_signal(Option::<Option<Color>>::None);
    #[allow(unused_variables)]
    let (game_end_reason, set_game_end_reason) = create_signal(String::new());
    #[allow(unused_variables)]
    let (is_ready_for_rematch, set_is_ready_for_rematch) = create_signal(false);

    // Dual Configs
    let (red_config, set_red_config) = create_signal(EngineConfig::default());
    let (black_config, set_black_config) = create_signal(EngineConfig::default());

    let get_piece_symbol = |p: PieceType, c: Color| -> &'static str {
        match p {
            PieceType::General => {
                if c == Color::Red {
                    "帥"
                } else {
                    "將"
                }
            }
            PieceType::Advisor => {
                if c == Color::Red {
                    "仕"
                } else {
                    "士"
                }
            }
            PieceType::Elephant => {
                if c == Color::Red {
                    "相"
                } else {
                    "象"
                }
            }
            PieceType::Horse => {
                if c == Color::Red {
                    "傌"
                } else {
                    "馬"
                }
            }
            PieceType::Chariot => {
                if c == Color::Red {
                    "俥"
                } else {
                    "車"
                }
            }
            PieceType::Cannon => {
                if c == Color::Red {
                    "炮"
                } else {
                    "砲"
                }
            }
            PieceType::Soldier => {
                if c == Color::Red {
                    "兵"
                } else {
                    "卒"
                }
            }
        }
    };

    // Worker Bridge
    let (worker_bridge, set_worker_bridge) =
        create_signal(Option::<WorkerBridge<GameWorker>>::None);

    create_effect(move |_| {
        let bridge = GameWorker::spawner()
            .callback(move |output| {
                match output {
                    Output::MoveFound(mv, stats) => {
                        let mut current_state = game_state.get();
                        match current_state.make_move(
                            BoardCoordinate::new(mv.from_row as usize, mv.from_col as usize)
                                .unwrap(),
                            BoardCoordinate::new(mv.to_row as usize, mv.to_col as usize).unwrap(),
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
                                if let Some(last) = current_state.history.last_mut() {
                                    last.note = Some(format!(
                                        "🤖 Depth: {}, Nodes: {}, Time: {}ms",
                                        stats.depth, stats.nodes, stats.time_ms
                                    ));
                                }
                                set_game_state.set(current_state);
                                set_is_thinking.set(false);
                            }
                            Err(e) => {
                                if e == cotuong_core::logic::rules::MoveError::ThreeFoldRepetition {
                                    web_sys::console::log_1(
                                        &format!("⚠️ Move rejected (3-fold), retrying... {mv:?}")
                                            .into(),
                                    );
                                    // Retry logic needs to be handled here or in worker.
                                    // Since worker returned a move, it thought it was valid.
                                    // But worker doesn't check repetition against history stack fully if not passed?
                                    // Actually AlphaBetaEngine checks repetition against history stack.
                                    // But if the move causes repetition in the *game* (which includes history), the engine should have seen it?
                                    // The engine has `history_stack`.
                                    // But `AlphaBetaEngine` in worker is recreated or updated.
                                    // It doesn't know the full game history unless we pass it?
                                    // `GameState` has `history`.
                                    // `AlphaBetaEngine` uses `history_stack` for internal search repetition.
                                    // But for the root move, we need to ensure it's valid in the game context.
                                    // If `make_move` fails, we should exclude this move and retry.
                                    // We need to send a new message to worker with this move excluded.
                                    // But `worker_bridge` is not easily accessible here inside the callback?
                                    // Actually `worker_bridge` signal holds the bridge.
                                    // But we can't call it easily from inside its own callback due to borrowing?
                                    // We can use `set_timeout` to schedule a retry.

                                    // For now, just log error and stop thinking to avoid hang.
                                    // Ideally we implement retry.
                                } else {
                                    web_sys::console::log_1(
                                        &format!("❌ Move error: {e:?}").into(),
                                    );
                                }
                                set_is_thinking.set(false);
                            }
                        }
                    }
                }
            })
            .spawn("./worker.js"); // Ensure this matches the output filename from Trunk
        set_worker_bridge.set(Some(bridge));
    });

    // Initialize Network Client
    create_effect(move |_| {
        if let Ok(client) = NetworkClient::new(set_server_msg) {
            set_network_client.set(Some(client));
        }
    });

    // Handle Server Messages
    create_effect(move |_| {
        if let Some(msg) = server_msg.get() {
            match msg {
                ServerMessage::WaitingForMatch => {
                    set_online_status.set(OnlineStatus::Finding);
                    leptos::logging::log!("Waiting for match...");
                }
                ServerMessage::MatchFound {
                    opponent_id: _,
                    your_color,
                    game_id: _,
                } => {
                    leptos::logging::log!("Match found! You are {:?}", your_color);
                    set_online_status.set(OnlineStatus::MatchFound);
                    set_game_mode.set(GameMode::Online);
                    set_player_side.set(your_color);
                    // Reset game
                    let new_state = GameState::new();
                    set_game_state.set(new_state);
                }
                ServerMessage::GameStart(board) => {
                    set_online_status.set(OnlineStatus::Playing);
                    set_game_mode.set(GameMode::Online);
                    let mut new_state = GameState::new();
                    new_state.board = *board;
                    set_game_state.set(new_state);
                }
                ServerMessage::OpponentMove(m) => {
                    let mut state = game_state.get();
                    if state.make_move(
                        BoardCoordinate::new(m.from_row as usize, m.from_col as usize).unwrap(),
                        BoardCoordinate::new(m.to_row as usize, m.to_col as usize).unwrap(),
                    ) == Ok(())
                    {
                        set_game_state.set(state);
                    }
                }
                ServerMessage::OpponentDisconnected => {
                    set_online_status.set(OnlineStatus::OpponentDisconnected);
                    leptos::logging::log!("Opponent disconnected!");
                }
                ServerMessage::GameEnd { winner, reason } => {
                    set_online_status.set(OnlineStatus::GameEnded);
                    set_game_end_winner.set(Some(winner));
                    set_game_end_reason.set(reason);
                    set_is_ready_for_rematch.set(false);
                }
                ServerMessage::Error(_) => {}
            }
        }
    });

    // on_move handler for BoardView
    let on_move = {
        Rc::new(move |m: Move| {
            if game_mode.get() == GameMode::Online {
                if let Some(client) = network_client.get() {
                    client.send(&GameMessage::MakeMove(m));
                }
            }
        }) as Rc<dyn Fn(Move)>
    };

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
            GameMode::HumanVsComputer => state.turn != player_side.get(),
            GameMode::ComputerVsComputer => true,
            GameMode::HumanVsHuman | GameMode::Online => false,
        };

        if should_play && state.status == GameStatus::Playing {
            if is_thinking.get() {
                return; // Already thinking
            }
            set_is_thinking.set(true);

            // Small delay to let UI update "Thinking..." state
            set_timeout(
                move || {
                    let mut current_state = game_state.get();
                    // Re-check condition
                    let current_mode = game_mode.get();
                    let current_paused = is_paused.get();

                    if current_paused {
                        set_is_thinking.set(false);
                        return;
                    }

                    let should_play_now = match current_mode {
                        GameMode::HumanVsComputer => current_state.turn != player_side.get(),
                        GameMode::ComputerVsComputer => true,
                        GameMode::HumanVsHuman | GameMode::Online => false,
                    };

                    if should_play_now && current_state.status == GameStatus::Playing {
                        let config = if current_state.turn == Color::Red {
                            red_config.get()
                        } else {
                            black_config.get()
                        };

                        let limit = match diff {
                            Difficulty::Level1 => SearchLimit::Time(1000),
                            Difficulty::Level2 => SearchLimit::Time(2000),
                            Difficulty::Level3 => SearchLimit::Time(5000),
                            Difficulty::Level4 => SearchLimit::Time(10000),
                            Difficulty::Level5 => SearchLimit::Time(20000),
                        };

                        // 1. Check Opening Book (Fast, do on main thread)
                        {
                            use cotuong_core::logic::opening;
                            let book_move =
                                opening::get_book_move(&current_state.board, current_state.turn);

                            if let Some((from, to)) = book_move {
                                if current_state.make_move(from, to).is_ok() {
                                    if let Some(last) = current_state.history.last_mut() {
                                        last.note = Some("📖 Book Move".to_string());
                                    }
                                    set_game_state.set(current_state);
                                    set_is_thinking.set(false);
                                    return;
                                }
                            }
                        }

                        // 2. Send to Worker
                        worker_bridge.with_untracked(|bridge| {
                            if let Some(bridge) = bridge {
                                bridge.send(Input::ComputeMove(
                                    current_state,
                                    limit,
                                    config,
                                    Vec::new(),
                                ));
                            } else {
                                web_sys::console::log_1(&"Worker bridge not ready".into());
                                set_is_thinking.set(false);
                            }
                        });
                        // web_sys::console::log_1(&"Worker disabled for debugging".into());
                        // set_is_thinking.set(false);
                    } else {
                        set_is_thinking.set(false);
                    }
                },
                Duration::from_millis(100),
            );
        }
    });

    // Sound Effects
    let move_sound = web_sys::HtmlAudioElement::new_with_src("sounds/move.mp3").ok();
    let capture_sound = web_sys::HtmlAudioElement::new_with_src("sounds/capture.mp3").ok();
    let check_sound = web_sys::HtmlAudioElement::new_with_src("sounds/check.mp3").ok();
    let checkmate_sound = web_sys::HtmlAudioElement::new_with_src("sounds/checkmate.mp3").ok();
    let last_len = store_value(0usize);

    create_effect(move |_| {
        let state = game_state.get();
        let current_len = state.history.len();

        last_len.update_value(|prev| {
            // Only play if history length increased and it's not the initial state (or empty)
            // We also want to skip if we just loaded a game?
            // For now, simple logic: if current > prev, play.
            // Exception: if prev is 0 and current is large (loaded game), maybe skip?
            // But we can't distinguish load vs fast moves easily without more state.
            // Let's just play sound.
            if current_len > *prev {
                if let Some(last_move) = state.history.last() {
                    let mut sound = &move_sound;
                    let is_capture = last_move.captured.is_some();

                    if let GameStatus::Checkmate(_) = state.status {
                        sound = &checkmate_sound;
                    } else if is_in_check(&state.board, state.turn) {
                        sound = &check_sound;
                    } else if is_capture {
                        sound = &capture_sound;
                    }

                    let _ = sound.as_ref().map(|a| a.play());
                }
            }
            *prev = current_len;
        });
    });

    let export_csv = move |_| {
        let state = game_state.get();
        let mut csv = String::from("Turn,From,To,Piece,Captured,Note\n");
        for (i, record) in state.history.iter().enumerate() {
            let turn = if i % 2 == 0 { "Red" } else { "Black" };
            let from = format!("({},{})", record.from.row, record.from.col);
            let to = format!("({},{})", record.to.row, record.to.col);
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

    let dropdown = |label: &'static str,
                    val: i32,
                    options: Vec<(i32, &'static str)>,
                    setter: Box<dyn Fn(i32)>| {
        view! {
            <div style="margin-bottom: 8px;">
                <div style="display: flex; justify-content: space-between; font-size: 0.9em; color: #ccc;">
                    <span>{label}</span>
                </div>
                <select
                    style="width: 100%; padding: 4px; background: #444; color: #eee; border: 1px solid #555; border-radius: 4px;"
                    on:change=move |ev| {
                        if let Ok(v) = event_target_value(&ev).parse::<i32>() {
                            setter(v);
                        }
                    }
                    prop:value=val
                >
                    {options.into_iter().map(|(v, txt)| {
                        view! {
                            <option value=v selected={v == val}>{txt}</option>
                        }
                    }).collect::<Vec<_>>()}
                </select>
            </div>
        }
    };

    let float_slider = |label: &'static str,
                        val: f32,
                        min: f32,
                        max: f32,
                        step: f32,
                        setter: Box<dyn Fn(f32)>| {
        view! {
            <div style="margin-bottom: 8px;">
                <div style="display: flex; justify-content: space-between; font-size: 0.9em; color: #ccc;">
                    <span>{label}</span>
                    <span>{format!("{val:.1}")}</span>
                </div>
                <input
                    type="range"
                    min=min
                    max=max
                    step=step
                    value=val
                    prop:value=val
                    style="width: 100%; accent-color: #a8e6cf;"
                    on:input=move |ev| {
                        if let Ok(v) = event_target_value(&ev).parse::<f32>() {
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

    let handle_file_upload = |setter: WriteSignal<EngineConfig>| {
        move |ev: web_sys::Event| {
            let Some(target) = ev.target() else {
                return;
            };
            let Ok(target) = target.dyn_into::<web_sys::HtmlInputElement>() else {
                return;
            };

            if let Some(files) = target.files() {
                if let Some(file) = files.get(0) {
                    let Ok(reader) = web_sys::FileReader::new() else {
                        return;
                    };
                    let reader_c = reader.clone();

                    let on_load = Closure::wrap(Box::new(move |_e: web_sys::Event| {
                        if let Ok(res) = reader_c.result() {
                            if let Some(text) = res.as_string() {
                                match serde_json::from_str::<EngineConfig>(&text) {
                                    Ok(config) => {
                                        web_sys::console::log_1(
                                            &"Config loaded successfully".into(),
                                        );
                                        setter.set(config);
                                    }
                                    Err(e) => {
                                        web_sys::console::log_1(
                                            &format!("Error parsing config: {e:?}").into(),
                                        );
                                        if let Some(window) = web_sys::window() {
                                            let _ = window.alert_with_message(&format!(
                                                "Error parsing JSON: {e}"
                                            ));
                                        }
                                    }
                                }
                            }
                        }
                    }) as Box<dyn FnMut(_)>);

                    reader.set_onload(Some(on_load.as_ref().unchecked_ref()));
                    on_load.forget();

                    if let Err(e) = reader.read_as_text(&file) {
                        web_sys::console::log_1(&format!("Error reading file: {e:?}").into());
                    }
                }
            }
        }
    };

    let export_config = |config: EngineConfig, filename: &str| {
        if let Ok(json) = serde_json::to_string_pretty(&config) {
            if let Ok(blob) =
                web_sys::Blob::new_with_str_sequence(&js_sys::Array::of1(&json.into()))
            {
                if let Ok(url) = web_sys::Url::create_object_url_with_blob(&blob) {
                    if let Ok(a) = document().create_element("a") {
                        if let Ok(a) = a.dyn_into::<web_sys::HtmlAnchorElement>() {
                            a.set_href(&url);
                            a.set_download(filename);
                            a.click();
                            let _ = web_sys::Url::revoke_object_url(&url);
                        }
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
                        align-items: stretch;
                        justify-content: center;
                        gap: 0; /* Remove gap on desktop to bring log closer */
                    }
                }

                .log-panel {
                    width: 90%;
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
                        width: 480px;
                        height: auto; /* Stretch to match board */
                        margin-top: 45px; /* Align with board canvas (skip captured pieces) */
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

                /* New Controls Design */
                .controls-area {
                    display: flex;
                    flex-direction: column;
                    gap: 15px;
                    width: 90%;
                    /* Removed max-width to let it fit the screen naturally */
                    margin: 20px auto;
                    padding: 20px;
                    background: #2a2a2a;
                    border-radius: 12px;
                    box-shadow: 0 4px 6px rgba(0,0,0,0.2);
                    border: 1px solid #444;
                    box-sizing: border-box;
                }

                .controls-config {
                    display: flex;
                    gap: 15px;
                    width: 100%;
                }

                .controls-actions {
                    display: grid;
                    grid-template-columns: 1fr 1fr;
                    gap: 15px;
                    width: 100%;
                }

                .control-group {
                    display: flex;
                    flex-direction: column;
                    gap: 5px;
                    flex: 1; /* Config items take equal width */
                }

                .control-label {
                    font-size: 0.85em;
                    color: #aaa;
                    margin-left: 2px;
                }

                select, button.control-btn {
                    width: 100%;
                    padding: 10px 14px;
                    border-radius: 8px;
                    border: 1px solid #555;
                    background: #3a3a3a;
                    color: #eee;
                    font-size: 14px;
                    cursor: pointer;
                    transition: all 0.2s ease;
                    outline: none;
                    font-family: inherit;
                    box-sizing: border-box; /* Ensure padding doesn't affect width */
                }

                select:hover, button.control-btn:hover {
                    background: #4a4a4a;
                    border-color: #777;
                    transform: translateY(-1px);
                    box-shadow: 0 2px 4px rgba(0,0,0,0.2);
                }
                
                select:focus, button.control-btn:focus {
                    border-color: #a8e6cf;
                    box-shadow: 0 0 0 2px rgba(168, 230, 207, 0.2);
                }

                button.btn-primary {
                    background: #4CAF50;
                    color: white;
                    border: none;
                }
                button.btn-primary:hover {
                    background: #45a049;
                }

                button.btn-warning {
                    background: #FF9800;
                    color: black;
                    border: none;
                    font-weight: 500;
                }
                button.btn-warning:hover {
                    background: #f57c00;
                }
                
                button.btn-danger {
                    background: #f44336;
                    color: white;
                    border: none;
                }
                 button.btn-danger:hover {
                    background: #d32f2f;
                }

                button.btn-info {
                    background: #2196F3;
                    color: white;
                    border: none;
                }
                button.btn-info:hover {
                    background: #1976D2;
                }

                @media (max-width: 480px) {
                    .controls-area {
                         /* Mobile tweaks if needed, but flex/grid generally handle it */
                         padding: 15px;
                    }
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

                .captured-panel {
                    padding: 10px;
                    background: #3a3a3a;
                    border-bottom: 1px solid #555;
                    display: flex;
                    flex-direction: column;
                    gap: 5px;
                }

                .captured-row {
                    display: flex;
                    align-items: center;
                    gap: 10px;
                    font-size: 0.9em;
                }

                .captured-label {
                    width: 60px;
                    font-weight: bold;
                    color: #aaa;
                }

                .captured-pieces {
                    display: flex;
                    flex-wrap: wrap;
                    gap: 2px;
                }

                .captured-piece {
                    width: 24px;
                    height: 24px;
                    border-radius: 50%;
                    background: #f0d9b5;
                    display: flex;
                    justify-content: center;
                    align-items: center;
                    font-family: 'KaiTi', '楷体', serif;
                    font-weight: bold;
                    font-size: 16px;
                    line-height: 1;
                    border: 1px solid #5c3a1e;
                }

                .layout-spacer {
                    display: none;
                }

                .side-column {
                    display: contents; /* Mobile: just show content */
                }

                @media (min-width: 1100px) {
                    .side-column {
                        display: flex;
                        flex: 1;
                        min-width: 0; /* Allow shrinking */
                    }
                    
                    /* Left column aligns right (next to board) */
                    .side-column.left {
                        justify-content: flex-end;
                        padding-right: 5px; /* Minimal padding */
                    }
                    
                    /* Right column aligns left (next to board) */
                    .side-column.right {
                        justify-content: flex-start;
                        padding-left: 5px; /* Minimal padding */
                    }

                    .log-panel {
                        width: 480px;
                        min-width: 350px; /* Prevent it from becoming too narrow */
                        height: auto; /* Stretch to match board */
                        margin-top: 45px; /* Align with board canvas (skip captured pieces) */
                    }
                }
                "
            </style>

            <h1 style="margin: 20px 0; color: #f0d9b5; text-shadow: 0 2px 4px rgba(0,0,0,0.5); text-align: center;">"Cờ Tướng"</h1>

            <div class="controls-area">
                <div class="controls-config">
                    <div class="control-group">
                        <span class="control-label">"Chế độ"</span>
                        <select
                            on:change=move |ev| {
                                let val = event_target_value(&ev);
                                match val.as_str() {
                                    "HumanVsComputer" => {
                                        set_game_mode.set(GameMode::HumanVsComputer);
                                        set_is_paused.set(false);
                                        set_online_status.set(OnlineStatus::None);
                                    },
                                    "ComputerVsComputer" => {
                                        set_game_mode.set(GameMode::ComputerVsComputer);
                                        set_is_paused.set(true); // Auto-pause on switch to CvC
                                        set_online_status.set(OnlineStatus::None);
                                    },
                                    "HumanVsHuman" => {
                                        set_game_mode.set(GameMode::HumanVsHuman);
                                        set_is_paused.set(false);
                                        set_online_status.set(OnlineStatus::None);
                                    },
                                    "Online" => {
                                        set_game_mode.set(GameMode::Online);
                                        set_is_paused.set(false);
                                        set_online_status.set(OnlineStatus::None);
                                    },
                                    _ => {},
                                }
                            }
                        >
                            <option value="HumanVsComputer">"Người vs Máy"</option>
                            <option value="ComputerVsComputer">"Máy vs Máy"</option>
                            <option value="HumanVsHuman">"Người vs Người"</option>
                            <option value="Online">"🌐 Chơi Online"</option>
                        </select>
                    </div>

                    <div class="control-group">
                        <span class="control-label">"Chọn bên"</span>
                        <select
                            on:change=move |ev| {
                                let val = event_target_value(&ev);
                                match val.as_str() {
                                    "Red" => set_player_side.set(Color::Red),
                                    "Black" => set_player_side.set(Color::Black),
                                    _ => {},
                                }
                            }
                        >
                            <option value="Red">"Đỏ (Đi trước)"</option>
                            <option value="Black">"Đen (Đi sau)"</option>
                        </select>
                    </div>

                    <div class="control-group">
                        <span class="control-label">"Độ khó"</span>
                        <select
                            on:change=move |ev| {
                                let val = event_target_value(&ev);
                                match val.as_str() {
                                    "Level1" => set_difficulty.set(Difficulty::Level1),
                                    "Level2" => set_difficulty.set(Difficulty::Level2),
                                    "Level3" => set_difficulty.set(Difficulty::Level3),
                                    "Level4" => set_difficulty.set(Difficulty::Level4),
                                    "Level5" => set_difficulty.set(Difficulty::Level5),
                                    _ => {},
                                }
                            }
                        >
                            <option value="Level1">"Mức 1 (1s)"</option>
                            <option value="Level2">"Mức 2 (2s)"</option>
                            <option value="Level3">"Mức 3 (5s)"</option>
                            <option value="Level4">"Mức 4 (10s)"</option>
                            <option value="Level5">"Mức 5 (20s)"</option>
                        </select>
                    </div>
                </div>

                <div class="controls-actions">
                    {move || {
                        if game_mode.get() == GameMode::ComputerVsComputer {
                            if is_paused.get() {
                                view! { <button class="control-btn btn-primary" on:click=move |_| set_is_paused.set(false)>"▶ Bắt đầu"</button> }.into_view()
                            } else {
                                view! { <button class="control-btn btn-danger" on:click=move |_| set_is_paused.set(true)>"⏸ Tạm dừng"</button> }.into_view()
                            }
                        } else {
                            view! {}.into_view()
                        }
                    }}

                    <button class="control-btn btn-info" on:click=move |_| {
                        set_game_state.set(GameState::new());
                        set_is_thinking.set(false);
                        if game_mode.get() == GameMode::ComputerVsComputer {
                            set_is_paused.set(true);
                        }
                    }>"Chơi mới"</button>

                    <button class="control-btn btn-warning" on:click=move |_| {
                        if is_thinking.get() {
                            return;
                        }
                        let mut state = game_state.get();
                        let mode = game_mode.get();

                        if mode == GameMode::HumanVsComputer && state.turn == Color::Red && state.history.len() >= 2 {
                            state.undo_move();
                        }
                        state.undo_move();
                        set_game_state.set(state);
                    }>"Đi lại"</button>

                    <button class="control-btn" on:click=export_csv>"Xuất CSV"</button>
                </div>
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

            // Online Status Panel
            {move || {
                let mode = game_mode.get();
                let status = online_status.get();
                let state = game_state.get();
                let side = player_side.get();

                if mode == GameMode::Online {
                    let status_content = match status {
                        OnlineStatus::None => view! {
                            <div style="display: flex; flex-direction: column; align-items: center; gap: 15px; padding: 20px;">
                                <div style="font-size: 1.2em; color: #a8e6cf;">
                                    "🌐 Chế độ chơi Online"
                                </div>
                                <button
                                    class="control-btn btn-primary"
                                    style="padding: 15px 40px; font-size: 1.1em;"
                                    on:click=move |_| {
                                        if let Some(client) = network_client.get() {
                                            client.send(&GameMessage::FindMatch);
                                        }
                                    }
                                >
                                    "🎮 Tìm trận"
                                </button>
                            </div>
                        }.into_view(),
                        OnlineStatus::Finding => view! {
                            <div style="display: flex; flex-direction: column; align-items: center; gap: 15px; padding: 20px;">
                                <div class="thinking-indicator" style="visibility: visible;">
                                    <span style="font-size: 1.2em;">"🔍 Đang tìm trận..."</span>
                                </div>
                                <button
                                    class="control-btn btn-danger"
                                    on:click=move |_| {
                                        if let Some(client) = network_client.get() {
                                            client.send(&GameMessage::CancelFindMatch);
                                        }
                                        set_online_status.set(OnlineStatus::None);
                                    }
                                >
                                    "❌ Huỷ tìm"
                                </button>
                            </div>
                        }.into_view(),
                        OnlineStatus::MatchFound => view! {
                            <div style="display: flex; flex-direction: column; align-items: center; gap: 10px; padding: 20px;">
                                <div style="font-size: 1.5em; color: #4CAF50; animation: pulse 1s infinite;">
                                    "✅ Đã tìm thấy đối thủ!"
                                </div>
                                <div style="font-size: 1.1em; color: #eee;">
                                    {format!("Bạn là bên: {}", if side == Color::Red { "🔴 Đỏ (đi trước)" } else { "⚫ Đen (đi sau)" })}
                                </div>
                            </div>
                        }.into_view(),
                        OnlineStatus::Playing => {
                            let is_my_turn = state.turn == side;
                            let turn_style = if is_my_turn {
                                "background: linear-gradient(135deg, #4CAF50, #45a049); color: white; padding: 15px 30px; border-radius: 12px; font-size: 1.2em; font-weight: bold; box-shadow: 0 4px 15px rgba(76, 175, 80, 0.4); animation: pulse 1.5s infinite;"
                            } else {
                                "background: linear-gradient(135deg, #555, #444); color: #aaa; padding: 15px 30px; border-radius: 12px; font-size: 1.2em; box-shadow: 0 2px 8px rgba(0,0,0,0.3);"
                            };
                            let turn_text = if is_my_turn {
                                if side == Color::Red { "🔴 Lượt của bạn!" } else { "⚫ Lượt của bạn!" }
                            } else {
                                "⏳ Đang chờ đối thủ..."
                            };
                            view! {
                                <div style="display: flex; flex-direction: column; align-items: center; gap: 15px; padding: 15px;">
                                    <div style=turn_style>
                                        {turn_text}
                                    </div>
                                    <div style="display: flex; gap: 10px;">
                                        <button
                                            class="control-btn btn-danger"
                                            style="padding: 10px 20px;"
                                            on:click=move |_| {
                                                if let Some(client) = network_client.get() {
                                                    client.send(&GameMessage::Surrender);
                                                }
                                            }
                                        >
                                            "🏳️ Đầu hàng"
                                        </button>
                                    </div>
                                </div>
                            }.into_view()
                        },
                        OnlineStatus::OpponentDisconnected => view! {
                            <div style="display: flex; flex-direction: column; align-items: center; gap: 15px; padding: 20px;">
                                <div style="font-size: 1.3em; color: #FF9800;">
                                    "⚠️ Đối thủ đã mất kết nối!"
                                </div>
                                <button
                                    class="control-btn btn-primary"
                                    on:click=move |_| {
                                        set_online_status.set(OnlineStatus::None);
                                        set_game_state.set(GameState::new());
                                    }
                                >
                                    "🔄 Tìm trận mới"
                                </button>
                            </div>
                        }.into_view(),
                        OnlineStatus::GameEnded => {
                            let winner = game_end_winner.get();
                            let reason = game_end_reason.get();
                            let ready = is_ready_for_rematch.get();

                            // Determine win/loss status
                            let (result_icon, result_text, result_color) = match winner {
                                Some(Some(w)) if w == side => ("🏆", "Bạn thắng!", "#4CAF50"),
                                Some(Some(_)) => ("😔", "Bạn thua!", "#f44336"),
                                Some(None) => ("🤝", "Hòa cờ!", "#FF9800"),
                                None => ("🏁", "Kết thúc", "#aaa"),
                            };

                            // Translate reason
                            let reason_text = match reason.as_str() {
                                "Checkmate" => "Chiếu hết",
                                "Surrender" => "Đầu hàng",
                                "Draw" => "Hòa",
                                "Disconnect" => "Mất kết nối",
                                _ => reason.as_str(),
                            };

                            view! {
                                <div style="display: flex; flex-direction: column; align-items: center; gap: 15px; padding: 20px;">
                                    <div style=format!("font-size: 2em; color: {};", result_color)>
                                        {format!("{result_icon} {result_text}")}
                                    </div>
                                    <div style="font-size: 1em; color: #aaa;">
                                        {format!("Lý do: {reason_text}")}
                                    </div>

                                    <div style="display: flex; flex-direction: column; gap: 10px; width: 100%; align-items: center;">
                                        {if ready {
                                            view! {
                                                <div style="background: #4CAF50; color: white; padding: 12px 24px; border-radius: 8px; font-weight: bold;">
                                                    "✅ Đã sẵn sàng - Đang chờ đối thủ..."
                                                </div>
                                            }.into_view()
                                        } else {
                                            view! {
                                                <button
                                                    class="control-btn btn-primary"
                                                    style="padding: 15px 40px; font-size: 1.1em;"
                                                    on:click=move |_| {
                                                        if let Some(client) = network_client.get() {
                                                            client.send(&GameMessage::PlayAgain);
                                                        }
                                                        set_is_ready_for_rematch.set(true);
                                                    }
                                                >
                                                    "🎮 Sẵn sàng (Chơi tiếp)"
                                                </button>
                                            }.into_view()
                                        }}

                                        <button
                                            class="control-btn"
                                            style="padding: 10px 20px;"
                                            on:click=move |_| {
                                                set_online_status.set(OnlineStatus::None);
                                                set_game_state.set(GameState::new());
                                                set_is_ready_for_rematch.set(false);
                                            }
                                        >
                                            "🚪 Thoát"
                                        </button>
                                    </div>
                                </div>
                            }.into_view()
                        },
                    };

                    view! {
                        <div style="background: linear-gradient(180deg, #2a2a2a, #333); border: 1px solid #444; border-radius: 12px; margin: 15px auto; max-width: 500px; box-shadow: 0 4px 20px rgba(0,0,0,0.3);">
                            {status_content}
                        </div>
                    }.into_view()
                } else {
                    view! {}.into_view()
                }
            }}

            <div class="game-layout">
                // Log Panel (Order 1 in HTML, but reversed on mobile to be bottom)
                <div class="side-column left">
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
                                    let from_str = format!("({}, {})", record.from.row, record.from.col);
                                    let to_str = format!("({}, {})", record.to.row, record.to.col);

                                    view! {
                                        <li class="log-item">
                                            <div class="move-info">
                                                <span>{format!("{}. {} {}", i + 1, turn_icon, turn_text)}</span>
                                                <div style="display: flex; align-items: center; gap: 5px;">
                                                    <span>{format!("{from_str} ➝ {to_str}")}</span>
                                                    {record.captured.map_or_else(
                                                        || view! {}.into_view(),
                                                        |cap| view! {
                                                            <span style="color: #ff9800; font-size: 0.9em; margin-left: 5px;">
                                                                {format!("(Eat {})", get_piece_symbol(cap.piece_type, cap.color))}
                                                            </span>
                                                        }.into_view()
                                                    )}
                                                </div>
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
                </div>

                // Board View (Order 2 in HTML)
                // Board View (Order 2 in HTML)
                <BoardView game_state=game_state set_game_state=set_game_state game_mode=game_mode player_side=player_side on_move=on_move />

                // Spacer for centering (Order 3)
                <div class="side-column right">
                    // Empty spacer div, or just the column itself acts as spacer
                </div>
            </div>

            <div style="margin-top: 20px; text-align: center;">
                <button
                    style=move || if show_config.get() {
                        "background: #555; color: white; padding: 15px 30px; font-size: 1.2em; font-weight: bold; border-radius: 8px; border: 1px solid #666; cursor: pointer; transition: all 0.2s ease;"
                    } else {
                        "background: #444; color: #eee; padding: 15px 30px; font-size: 1.2em; font-weight: bold; border-radius: 8px; border: 1px solid #555; cursor: pointer; transition: all 0.2s ease;"
                    }
                    on:click=move |_| set_show_config.update(|v| *v = !*v)
                >
                    "Cấu hình engine"
                </button>
            </div>

            {move || {
                if show_config.get() {
                    view! {
                        <div class="config-panel">
                            <div class="config-column">
                                <div class="config-title" style="color: #ff6b6b;">"Cấu hình Đỏ (Red)"</div>
                                <div style="margin-bottom: 15px; text-align: center;">
                                    <label style="display: block; margin-bottom: 5px; color: #ccc; font-size: 0.9em;">"Load JSON Config"</label>
                                    <input type="file" accept=".json" on:change=handle_file_upload(set_red_config) style="color: #ccc;" />
                                    <button style="margin-top: 5px; font-size: 0.8em;" on:click=move |_| export_config(red_config.get(), "red_config.json")>"Export JSON"</button>
                                </div>
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
                                                {dropdown("Pruning Method", config.pruning_method, vec![
                                                    (0, "Dynamic Limiting"),
                                                    (1, "Late Move Reductions (LMR)"),
                                                    (2, "Both (Aggressive)"),
                                                ], Box::new(move |v| { let mut c = red_config.get(); c.pruning_method = v; set_red_config.set(c); }))}
                                                {float_slider("Multiplier", config.pruning_multiplier, 0.1, 2.0, 0.1, Box::new(move |v| { let mut c = red_config.get(); c.pruning_multiplier = v; set_red_config.set(c); }))}
                                            </div>
                                        }
                                    }
                                }
                            </div>
                            <div class="config-column">
                                <div class="config-title" style="color: #a8e6cf;">"Cấu hình Đen (Black)"</div>
                                <div style="margin-bottom: 15px; text-align: center;">
                                    <label style="display: block; margin-bottom: 5px; color: #ccc; font-size: 0.9em;">"Load JSON Config"</label>
                                    <input type="file" accept=".json" on:change=handle_file_upload(set_black_config) style="color: #ccc;" />
                                    <button style="margin-top: 5px; font-size: 0.8em;" on:click=move |_| export_config(black_config.get(), "black_config.json")>"Export JSON"</button>
                                </div>
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
                                                {dropdown("Pruning Method", config.pruning_method, vec![
                                                    (0, "Dynamic Limiting"),
                                                    (1, "Late Move Reductions (LMR)"),
                                                    (2, "Both (Aggressive)"),
                                                ], Box::new(move |v| { let mut c = black_config.get(); c.pruning_method = v; set_black_config.set(c); }))}
                                                {float_slider("Multiplier", config.pruning_multiplier, 0.1, 2.0, 0.1, Box::new(move |v| { let mut c = black_config.get(); c.pruning_multiplier = v; set_black_config.set(c); }))}
                                            </div>
                                        }
                                    }
                                }
                            </div>
                        </div>
                    }.into_view()
                } else {
                    view! {}.into_view()
                }
            }}
        </div>
    }
}
