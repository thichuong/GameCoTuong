use cotuong_core::logic::game::GameState;
use leptos::{component, view, IntoView, ReadSignal, SignalGet};

#[component]
pub fn LogPanel(game_state: ReadSignal<GameState>) -> impl IntoView {
    view! {
        <div class="log-panel">
            <div class="log-header">
                <span>"📜 Biên bản"</span>
                <span style="font-size: 0.8em; opacity: 0.8;">{move || format!("{} nước", game_state.get().history.len())}</span>
            </div>
            <ul class="log-list">
                {move || {
                    let state = game_state.get();
                    state.history.iter().enumerate().rev().map(|(i, record)| {
                        let turn_num = (i / 2) + 1;
                        let side = if i % 2 == 0 { "🔴" } else { "⚫" };
                        view! {
                            <li class="log-item">
                                <div class="move-info">
                                    <span>{format!("{turn_num}. {side} {} → {}",
                                        format!("({},{})", record.from.row, record.from.col),
                                        format!("({},{})", record.to.row, record.to.col)
                                    )}</span>
                                    <span style="color: #f0d9b5;">{format!("{:?}", record.piece.piece_type)}</span>
                                </div>
                                {if let Some(note) = &record.note {
                                    view! { <div class="ai-stats">{note}</div> }.into_view()
                                } else {
                                    view! {}.into_view()
                                }}
                            </li>
                        }
                    }).collect::<Vec<_>>()
                }}
            </ul>
        </div>
    }
}

#[component]
pub fn ThinkingIndicator(is_thinking: ReadSignal<bool>) -> impl IntoView {
    view! {
        <div class="thinking-indicator" style=move || if is_thinking.get() { "visibility: visible;" } else { "visibility: hidden;" }>
            <span>"Máy đang nghĩ..."</span>
            <div style="width: 10px; height: 10px; background: #a8e6cf; border-radius: 50%; display: inline-block;"></div>
        </div>
    }
}
