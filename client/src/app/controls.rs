use crate::app::{Difficulty, GameMode};
use cotuong_core::logic::board::Color;
use cotuong_core::logic::game::GameState;
use leptos::{
    component, event_target_value, view, Callback, Callable, IntoView, ReadSignal, SignalGet,
    SignalSet, WriteSignal,
};

#[component]
#[allow(clippy::too_many_arguments)]
pub fn ControlsArea(
    game_mode: ReadSignal<GameMode>,
    set_game_mode: WriteSignal<GameMode>,
    player_side: ReadSignal<Color>,
    set_player_side: WriteSignal<Color>,
    difficulty: ReadSignal<Difficulty>,
    set_difficulty: WriteSignal<Difficulty>,
    is_paused: ReadSignal<bool>,
    set_is_paused: WriteSignal<bool>,
    game_state: ReadSignal<GameState>,
    set_game_state: WriteSignal<GameState>,
    is_thinking: ReadSignal<bool>,
    set_is_thinking: WriteSignal<bool>,
    on_export_csv: Callback<()>,
) -> impl IntoView {
    view! {
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
                                },
                                "ComputerVsComputer" => {
                                    set_game_mode.set(GameMode::ComputerVsComputer);
                                    set_is_paused.set(true);
                                },
                                "HumanVsHuman" => {
                                    set_game_mode.set(GameMode::HumanVsHuman);
                                    set_is_paused.set(false);
                                },
                                "Online" => {
                                    set_game_mode.set(GameMode::Online);
                                    set_is_paused.set(false);
                                },
                                _ => {},
                            }
                        }
                        prop:value=move || match game_mode.get() {
                            GameMode::HumanVsComputer => "HumanVsComputer",
                            GameMode::ComputerVsComputer => "ComputerVsComputer",
                            GameMode::HumanVsHuman => "HumanVsHuman",
                            GameMode::Online => "Online",
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
                        prop:value=move || match player_side.get() {
                            Color::Red => "Red",
                            Color::Black => "Black",
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
                        prop:value=move || match difficulty.get() {
                            Difficulty::Level1 => "Level1",
                            Difficulty::Level2 => "Level2",
                            Difficulty::Level3 => "Level3",
                            Difficulty::Level4 => "Level4",
                            Difficulty::Level5 => "Level5",
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

                <button class="control-btn" on:click=move |_| on_export_csv.call(())>"Xuất CSV"</button>
            </div>
        </div>
    }
}
