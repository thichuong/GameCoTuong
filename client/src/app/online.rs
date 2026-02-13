#![allow(clippy::option_option, clippy::too_many_lines)]
use crate::app::{GameMode, OnlineStatus};
use crate::network::NetworkClient;
use cotuong_core::logic::board::Color;
use cotuong_core::logic::game::GameState;
use leptos::{component, view, IntoView, ReadSignal, SignalGet, SignalSet, WriteSignal};
use shared::GameMessage;

#[component]
#[allow(clippy::too_many_arguments)]
#[allow(clippy::too_many_lines, clippy::option_option)]
pub fn OnlineStatusPanel(
    game_mode: ReadSignal<GameMode>,
    online_status: ReadSignal<OnlineStatus>,
    game_state: ReadSignal<GameState>,
    player_side: ReadSignal<Color>,
    network_client: ReadSignal<Option<NetworkClient>>,
    game_end_winner: ReadSignal<Option<Option<Color>>>,
    game_end_reason: ReadSignal<String>,
    is_ready_for_rematch: ReadSignal<bool>,
    set_online_status: WriteSignal<OnlineStatus>,
    set_game_state: WriteSignal<GameState>,
    set_is_ready_for_rematch: WriteSignal<bool>,
) -> impl IntoView {
    view! {
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
                                        set_online_status.set(OnlineStatus::Finding);
                                    } else {
                                        leptos::logging::log!("❌ Cannot find match: NetworkClient is not initialized (Server might be down)");
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
                                            if let Some(client) = network_client.get() {
                                                client.send(&GameMessage::PlayerLeft);
                                            }
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
    }
}
