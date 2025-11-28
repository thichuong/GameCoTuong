use crate::components::board::BoardView;
use crate::logic::game::GameState;
use leptos::*;

#[component]
pub fn App() -> impl IntoView {
    // Create a signal for the game state
    let (game_state, set_game_state) = create_signal(GameState::new());

    view! {
        <div class="game-container">
            <h1>"Cờ Tướng"</h1>
            <BoardView game_state=game_state set_game_state=set_game_state />
        </div>
    }
}
