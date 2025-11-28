use crate::logic::board::{Color, Piece, PieceType};
use crate::logic::game::GameState;
use leptos::*;

#[component]
pub fn BoardView(
    game_state: ReadSignal<GameState>,
    set_game_state: WriteSignal<GameState>,
) -> impl IntoView {
    let (selected, set_selected) = create_signal(Option::<(usize, usize)>::None);

    let board_style = "
        display: grid;
        grid-template-columns: repeat(9, 45px);
        grid-template-rows: repeat(10, 45px);
        gap: 1px;
        background-color: #d18b47;
        padding: 10px;
        border: 4px solid #5c3a1e;
        border-radius: 4px;
        box-shadow: 0 4px 6px rgba(0,0,0,0.3);
    ";

    let cell_style = |is_selected: bool, is_valid_target: bool| {
        let bg = if is_selected {
            "#a8e6cf" // Selected color
        } else if is_valid_target {
            "#ffaaa5" // Target hint (optional, maybe later)
        } else {
            "#eecfa1" // Default
        };
        format!(
            "
            width: 45px;
            height: 45px;
            display: flex;
            justify_content: center;
            align_items: center;
            background-color: {};
            cursor: pointer;
            user-select: none;
            position: relative;
        ",
            bg
        )
    };

    let handle_click = move |row: usize, col: usize| {
        let state = game_state.get();
        let current_turn = state.turn;
        let clicked_piece = state.board.get_piece(row, col);

        if let Some((from_row, from_col)) = selected.get() {
            // A piece is already selected
            if from_row == row && from_col == col {
                // Clicked same piece, deselect
                set_selected.set(None);
            } else {
                // Try to move
                // Check if clicked friendly piece (change selection)
                if let Some(p) = clicked_piece {
                    if p.color == current_turn {
                        set_selected.set(Some((row, col)));
                        return;
                    }
                }

                // Attempt move
                let mut new_state = state.clone();
                match new_state.make_move(from_row, from_col, row, col) {
                    Ok(_) => {
                        set_game_state.set(new_state);
                        set_selected.set(None);
                    }
                    Err(e) => {
                        leptos::logging::log!("Invalid move: {:?}", e);
                        // Optional: Show error feedback
                    }
                }
            }
        } else {
            // No piece selected, try to select
            if let Some(p) = clicked_piece {
                if p.color == current_turn {
                    set_selected.set(Some((row, col)));
                }
            }
        }
    };

    view! {
        <div style="display: flex; flex-direction: column; align-items: center;">
            <div style=board_style>
                {move || {
                    let state = game_state.get();
                    let mut cells = Vec::new();

                    // Render from top (row 9) to bottom (row 0)
                    for row in (0..10).rev() {
                        for col in 0..9 {
                            let piece = state.board.get_piece(row, col);
                            let is_selected = selected.get() == Some((row, col));

                            cells.push(view! {
                                <div
                                    style=cell_style(is_selected, false)
                                    on:click=move |_| handle_click(row, col)
                                >
                                    {render_piece(piece)}
                                    {move || {
                                        // Optional: Show coordinate for debugging
                                        // view! { <span style="position: absolute; bottom: 0; right: 0; font-size: 8px; opacity: 0.5;">{format!("{},{}", row, col)}</span> }
                                    }}
                                </div>
                            });
                        }
                    }
                    cells
                }}
            </div>
            <div class="status" style="margin-top: 20px; font-size: 1.2em; color: #eee;">
                {move || {
                    let state = game_state.get();
                    match state.status {
                        crate::logic::game::GameStatus::Playing => format!("Turn: {:?}", state.turn),
                        crate::logic::game::GameStatus::Checkmate(winner) => format!("Checkmate! {:?} Wins!", winner),
                        crate::logic::game::GameStatus::Stalemate => "Stalemate!".to_string(),
                    }
                }}
            </div>
        </div>
    }
}

fn render_piece(piece: Option<Piece>) -> impl IntoView {
    match piece {
        Some(p) => {
            let (color, bg_color) = match p.color {
                Color::Red => ("#c00", "#fff"),
                Color::Black => ("#000", "#fff"),
            };
            let symbol = match p.piece_type {
                PieceType::General => {
                    if p.color == Color::Red {
                        "帥"
                    } else {
                        "將"
                    }
                }
                PieceType::Advisor => {
                    if p.color == Color::Red {
                        "仕"
                    } else {
                        "士"
                    }
                }
                PieceType::Elephant => {
                    if p.color == Color::Red {
                        "相"
                    } else {
                        "象"
                    }
                }
                PieceType::Horse => {
                    if p.color == Color::Red {
                        "傌"
                    } else {
                        "馬"
                    }
                }
                PieceType::Chariot => {
                    if p.color == Color::Red {
                        "俥"
                    } else {
                        "車"
                    }
                }
                PieceType::Cannon => {
                    if p.color == Color::Red {
                        "炮"
                    } else {
                        "砲"
                    }
                }
                PieceType::Soldier => {
                    if p.color == Color::Red {
                        "兵"
                    } else {
                        "卒"
                    }
                }
            };

            view! {
                <div style=format!("
                    width: 36px; 
                    height: 36px; 
                    border-radius: 50%; 
                    background-color: {}; 
                    color: {}; 
                    border: 2px solid {};
                    display: flex; 
                    justify_content: center; 
                    align_items: center; 
                    font-size: 20px; 
                    font-weight: bold;
                    box-shadow: 1px 1px 3px rgba(0,0,0,0.3);
                ", bg_color, color, color)>
                    {symbol}
                </div>
            }
        }
        None => view! { <div style="width: 36px; height: 36px;"></div> },
    }
}
