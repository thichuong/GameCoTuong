use crate::components::board::BoardView;
use cotuong_core::engine::config::EngineConfig;
use cotuong_core::engine::Move;
use cotuong_core::engine::SearchLimit;
use cotuong_core::logic::board::{BoardCoordinate, Color};
use cotuong_core::logic::game::{GameState, GameStatus};
use cotuong_core::logic::rules::is_in_check;
use cotuong_core::worker::{GameWorker, Input, Output};
use gloo_worker::{Spawnable, WorkerBridge};
use leptos::{
    component, create_effect, create_signal, set_timeout, store_value, view, web_sys, Callback,
    IntoView, SignalGet, SignalSet, SignalUpdate, SignalWithUntracked,
};
use shared::{GameMessage, ServerMessage};
use std::rc::Rc;
use std::time::Duration;

use crate::app::config::ConfigPanel;
use crate::app::controls::ControlsArea;
use crate::app::export::export_csv;
use crate::app::log::{LogPanel, ThinkingIndicator};
use crate::app::online::OnlineStatusPanel;
use crate::app::styles::GAME_STYLES;
use crate::app::{Difficulty, GameMode, OnlineStatus};
use crate::network::NetworkClient;

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
    let (game_end_winner, set_game_end_winner) = create_signal(Option::<Option<Color>>::None);
    let (game_end_reason, set_game_end_reason) = create_signal(String::new());
    let (is_ready_for_rematch, set_is_ready_for_rematch) = create_signal(false);

    // Dual Configs
    let (red_config, set_red_config) = create_signal(EngineConfig::default());
    let (black_config, set_black_config) = create_signal(EngineConfig::default());

    // Worker Bridge
    let (worker_bridge, set_worker_bridge) =
        create_signal(Option::<WorkerBridge<GameWorker>>::None);

    create_effect(move |_| {
        let bridge = GameWorker::spawner()
            .callback(move |output| match output {
                Output::MoveFound(mv, stats) => {
                    let mut current_state = game_state.get();
                    if let (Some(from), Some(to)) = (
                        BoardCoordinate::new(mv.from_row as usize, mv.from_col as usize),
                        BoardCoordinate::new(mv.to_row as usize, mv.to_col as usize),
                    ) {
                        match current_state.make_move(from, to) {
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
            .spawn("./worker.js");
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
                ServerMessage::OpponentMove { move_data: m, fen } => {
                    let mut state = game_state.get();
                    if let (Some(from), Some(to)) = (
                        BoardCoordinate::new(m.from_row as usize, m.from_col as usize),
                        BoardCoordinate::new(m.to_row as usize, m.to_col as usize),
                    ) {
                        match state.make_move(from, to) {
                            Ok(()) => {
                                set_game_state.set(state.clone());
                                // Calculate FEN and verify
                                let calculated_fen = state.board.to_fen_string(state.turn);
                                let is_valid = calculated_fen == fen;

                                if !is_valid {
                                    leptos::logging::log!(
                                        "FEN Mismatch! Server: {}, Client: {}",
                                        fen,
                                        calculated_fen
                                    );
                                }

                                if let Some(client) = network_client.get() {
                                    client.send(&GameMessage::VerifyMove {
                                        fen: calculated_fen,
                                        is_valid,
                                    });
                                }
                            }
                            Err(e) => {
                                leptos::logging::log!("Error applying opponent move: {:?}", e);
                                if let Some(client) = network_client.get() {
                                    client.send(&GameMessage::VerifyMove {
                                        fen: state.board.to_fen_string(state.turn),
                                        is_valid: false,
                                    });
                                }
                            }
                        }
                    }
                }
                ServerMessage::GameStateCorrection { fen, turn } => {
                    match cotuong_core::logic::board::Board::from_fen(&fen) {
                        Ok((board, _)) => {
                            let mut state = GameState::new();
                            state.board = board;
                            state.turn = turn;
                            leptos::logging::log!("Correcting Game State to: {} ({:?})", fen, turn);
                            set_game_state.set(state);
                        }
                        Err(e) => {
                            leptos::logging::log!("Failed to parse correction FEN: {}", e);
                        }
                    }
                }
                ServerMessage::OpponentDisconnected => {
                    set_online_status.set(OnlineStatus::OpponentDisconnected);
                    leptos::logging::log!("Opponent disconnected!");
                }
                ServerMessage::OpponentLeftGame => {
                    set_online_status.set(OnlineStatus::None);
                    set_game_state.set(GameState::new());
                    set_is_ready_for_rematch.set(false);
                    leptos::logging::log!("Opponent left the room.");
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
                    let state = game_state.get();
                    let fen = state.board.to_fen_string(state.turn);
                    client.send(&GameMessage::MakeMove { move_data: m, fen });
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
                return;
            }
            set_is_thinking.set(true);

            set_timeout(
                move || {
                    let mut current_state = game_state.get();
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

                        // 1. Check Opening Book
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

    view! {
        <div class="game-container" style="font-family: 'Segoe UI', Tahoma, Geneva, Verdana, sans-serif; min-height: 100vh; background-color: #222; color: #eee; display: flex; flex-direction: column; align-items: center;">
            <style>
                {GAME_STYLES}
            </style>

            <h1 style="margin: 20px 0; color: #f0d9b5; text-shadow: 0 2px 4px rgba(0,0,0,0.5); text-align: center;">"Cờ Tướng"</h1>

            <ThinkingIndicator is_thinking=is_thinking />

            <ControlsArea
                game_mode=game_mode
                set_game_mode=set_game_mode
                player_side=player_side
                set_player_side=set_player_side
                difficulty=difficulty
                set_difficulty=set_difficulty
                is_paused=is_paused
                set_is_paused=set_is_paused
                game_state=game_state
                set_game_state=set_game_state
                is_thinking=is_thinking
                set_is_thinking=set_is_thinking
                on_export_csv=Callback::new(move |_| export_csv(game_state.get()))
            />

            <OnlineStatusPanel
                game_mode=game_mode
                online_status=online_status
                game_state=game_state
                player_side=player_side
                network_client=network_client
                game_end_winner=game_end_winner
                game_end_reason=game_end_reason
                is_ready_for_rematch=is_ready_for_rematch
                set_online_status=set_online_status
                set_game_state=set_game_state
                set_is_ready_for_rematch=set_is_ready_for_rematch
            />

            <div class="game-layout">
                <div class="side-column left">
                    <LogPanel game_state=game_state />
                </div>

                <BoardView game_state=game_state set_game_state=set_game_state game_mode=game_mode player_side=player_side on_move=on_move />

                <div class="side-column right">
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

            <ConfigPanel
                show_config=show_config
                red_config=red_config
                set_red_config=set_red_config
                black_config=black_config
                set_black_config=set_black_config
            />
        </div>
    }
}
