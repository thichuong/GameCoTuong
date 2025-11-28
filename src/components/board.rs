use crate::logic::board::{Color, Piece, PieceType};
use crate::logic::game::GameState;
use leptos::*;

#[component]
pub fn BoardView(
    game_state: ReadSignal<GameState>,
    set_game_state: WriteSignal<GameState>,
) -> impl IntoView {
    let (selected, set_selected) = create_signal(Option::<(usize, usize)>::None);

    let cell_size = 50; // px
    let grid_width = 9 * cell_size;
    let grid_height = 10 * cell_size;
    let half_cell = cell_size / 2;

    // Style for the container
    let container_style = format!(
        "
        position: relative;
        width: {}px;
        height: {}px;
        background-color: #eecfa1; /* Wood color */
        border: 2px solid #5c3a1e;
        user-select: none;
        margin: 20px;
        box-shadow: 0 5px 15px rgba(0,0,0,0.3);
    ",
        grid_width, grid_height
    );

    // Style for the pieces layer (CSS Grid)
    let pieces_layer_style = format!(
        "
        position: absolute;
        top: 0;
        left: 0;
        width: 100%;
        height: 100%;
        display: grid;
        grid-template-columns: repeat(9, {}px);
        grid-template-rows: repeat(10, {}px);
        z-index: 10;
    ",
        cell_size, cell_size
    );

    let handle_click = move |row: usize, col: usize| {
        let state = game_state.get();
        let current_turn = state.turn;
        let clicked_piece = state.board.get_piece(row, col);

        if let Some((from_row, from_col)) = selected.get() {
            if from_row == row && from_col == col {
                set_selected.set(None);
            } else if let Some(p) = clicked_piece {
                if p.color == current_turn {
                    set_selected.set(Some((row, col)));
                } else {
                    // If a piece of the opposite color is clicked, try to move to it
                    let mut new_state = state.clone();
                    match new_state.make_move(from_row, from_col, row, col) {
                        Ok(_) => {
                            set_game_state.set(new_state);
                            set_selected.set(None);
                        }
                        Err(e) => {
                            leptos::logging::log!("Invalid move: {:?}", e);
                        }
                    }
                }
            } else {
                // No piece clicked, try to move to empty square
                let mut new_state = state.clone();
                match new_state.make_move(from_row, from_col, row, col) {
                    Ok(_) => {
                        set_game_state.set(new_state);
                        set_selected.set(None);
                    }
                    Err(e) => {
                        leptos::logging::log!("Invalid move: {:?}", e);
                    }
                }
            }
        } else {
            if let Some(p) = clicked_piece {
                if p.color == current_turn {
                    set_selected.set(Some((row, col)));
                }
            }
        }
    };

    view! {
        <div style="display: flex; flex-direction: column; align-items: center;">
            <div style=container_style>
                // Layer 1: SVG Board Lines
                <svg width=grid_width height=grid_height style="position: absolute; top: 0; left: 0; z-index: 1;">
                    // Background
                    <rect x="0" y="0" width=grid_width height=grid_height fill="#eecfa1" />

                    // Grid Lines
                    {
                        let mut lines = Vec::new();
                        // Horizontal lines (10 rows)
                        for r in 0..10 {
                            let y = r * cell_size + half_cell;
                            let x1 = half_cell;
                            let x2 = grid_width - half_cell;
                            lines.push(view! { <line x1=x1 y1=y x2=x2 y2=y stroke="#5c3a1e" stroke-width="2" /> });
                        }

                        // Vertical lines (9 cols)
                        for c in 0..9 {
                            let x = c * cell_size + half_cell;
                            // Top half (rows 0-4 in visual, but 9-5 in logic? No, let's stick to visual coordinates)
                            // Visual row 0 is top. Logic row 9 is top.
                            // We render from top to bottom visually.
                            // Top half: Visual y from half_cell to 4*cell_size + half_cell
                            let y_top_start = half_cell;
                            let y_top_end = 4 * cell_size + half_cell;

                            // Bottom half: Visual y from 5*cell_size + half_cell to end
                            let y_bot_start = 5 * cell_size + half_cell;
                            let y_bot_end = 9 * cell_size + half_cell;

                            if c == 0 || c == 8 {
                                // Side lines go all the way
                                lines.push(view! { <line x1=x y1=y_top_start x2=x y2=y_bot_end stroke="#5c3a1e" stroke-width="2" /> });
                            } else {
                                // Inner lines interrupted by river
                                lines.push(view! { <line x1=x y1=y_top_start x2=x y2=y_top_end stroke="#5c3a1e" stroke-width="2" /> });
                                lines.push(view! { <line x1=x y1=y_bot_start x2=x y2=y_bot_end stroke="#5c3a1e" stroke-width="2" /> });
                            }
                        }

                        // Palaces (X shapes)
                        // Top Palace (Visual rows 0-2, cols 3-5)
                        // (3,0) to (5,2) and (5,0) to (3,2)
                        let p_start = 3 * cell_size + half_cell;
                        let p_end = 5 * cell_size + half_cell;
                        let r0 = half_cell;
                        let r2 = 2 * cell_size + half_cell;
                        lines.push(view! { <line x1=p_start y1=r0 x2=p_end y2=r2 stroke="#5c3a1e" stroke-width="2" /> });
                        lines.push(view! { <line x1=p_end y1=r0 x2=p_start y2=r2 stroke="#5c3a1e" stroke-width="2" /> });

                        // Bottom Palace (Visual rows 7-9, cols 3-5)
                        let r7 = 7 * cell_size + half_cell;
                        let r9 = 9 * cell_size + half_cell;
                        lines.push(view! { <line x1=p_start y1=r7 x2=p_end y2=r9 stroke="#5c3a1e" stroke-width="2" /> });
                        lines.push(view! { <line x1=p_end y1=r7 x2=p_start y2=r9 stroke="#5c3a1e" stroke-width="2" /> });

                        lines
                    }

                    // River Text (Optional)
                    <text x={grid_width / 4} y={grid_height / 2 + 8} font-family="serif" font-size="24" fill="#5c3a1e" text-anchor="middle" style="opacity: 0.5;">"楚 河"</text>
                    <text x={grid_width * 3 / 4} y={grid_height / 2 + 8} font-family="serif" font-size="24" fill="#5c3a1e" text-anchor="middle" style="opacity: 0.5;">"漢 界"</text>
                </svg>

                // Layer 2: Pieces (Interactive)
                <div style=pieces_layer_style>
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
                                        style="display: flex; justify_content: center; align_items: center; cursor: pointer;"
                                        on:click=move |_| handle_click(row, col)
                                    >
                                        {render_piece(piece, is_selected)}
                                    </div>
                                });
                            }
                        }
                        cells
                    }}
                </div>
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

fn render_piece(piece: Option<Piece>, is_selected: bool) -> impl IntoView {
    match piece {
        Some(p) => {
            let (color, bg_color, border_color) = match p.color {
                Color::Red => ("#c00", "#f0d9b5", "#c00"),
                Color::Black => ("#000", "#f0d9b5", "#000"),
            };

            let scale = if is_selected {
                "transform: scale(1.15);"
            } else {
                ""
            };
            let shadow = if is_selected {
                "box-shadow: 0 0 10px #a8e6cf, 2px 2px 5px rgba(0,0,0,0.4);"
            } else {
                "box-shadow: 1px 1px 4px rgba(0,0,0,0.4);"
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
                    width: 40px; 
                    height: 40px; 
                    border-radius: 50%; 
                    background-color: {}; 
                    color: {}; 
                    border: 2px solid {};
                    display: flex; 
                    justify_content: center; 
                    align_items: center; 
                    font-size: 24px; 
                    font-family: 'KaiTi', '楷体', serif;
                    font-weight: bold;
                    transition: transform 0.1s;
                    z-index: 20;
                    {};
                    {}
                ", bg_color, color, border_color, shadow, scale)>
                    {symbol}
                </div>
            }
        }
        None => view! {
            // Transparent placeholder for click target
            <div style=format!("
                width: 40px; 
                height: 40px; 
                border-radius: 50%; 
                opacity: 0.3;
                background-color: {};
            ", if is_selected { "#a8e6cf" } else { "transparent" })></div> 
        },
    }
}
