use crate::logic::board::{Color, Piece, PieceType};
use crate::logic::game::GameState;
use leptos::{
    component, create_effect, create_signal, set_timeout, view, IntoView, Props, ReadSignal,
    SignalGet, SignalSet, WriteSignal,
};
use std::time::Duration;

#[component]
pub fn BoardView(
    game_state: ReadSignal<GameState>,
    set_game_state: WriteSignal<GameState>,
) -> impl IntoView {
    let (selected, set_selected) = create_signal(Option::<(usize, usize)>::None);
    let (anim_dest, set_anim_dest) = create_signal(Option::<(usize, usize)>::None);

    // Trigger animation only when a move happens
    create_effect(move |_| {
        let state = game_state.get();
        if let Some((_, (tr, tc))) = state.last_move {
            set_anim_dest.set(Some((tr, tc)));
            set_timeout(
                move || {
                    set_anim_dest.set(None);
                },
                Duration::from_millis(500),
            );
        }
    });

    // Robust sizing:
    // - Mobile: 96vw (almost full width)
    // - Desktop: Constrained by height (80vh) to prevent scrolling.
    // - Aspect Ratio: 9/10 (0.9)
    // - Width = min(96vw, 80vh * 0.9) = min(96vw, 72vh)
    let container_style = "
        position: relative;
        width: min(96vw, 72vh);
        aspect-ratio: 9 / 10;
        background-color: #eecfa1;
        border: 2px solid #5c3a1e;
        user-select: none;
        margin: 0 auto;
        box-shadow: 0 5px 15px rgba(0,0,0,0.3);
        box-sizing: border-box;
    ";

    let pieces_layer_style = "
        position: absolute;
        top: 0;
        left: 0;
        width: 100%;
        height: 100%;
        display: grid;
        grid-template-columns: repeat(9, 1fr);
        grid-template-rows: repeat(10, 1fr);
        z-index: 10;
    ";

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
                    let mut new_state = state.clone();
                    match new_state.make_move(from_row, from_col, row, col) {
                        Ok(()) => {
                            set_game_state.set(new_state);
                            set_selected.set(None);
                        }
                        Err(e) => {
                            leptos::logging::log!("Invalid move: {:?}", e);
                        }
                    }
                }
            } else {
                let mut new_state = state.clone();
                match new_state.make_move(from_row, from_col, row, col) {
                    Ok(()) => {
                        set_game_state.set(new_state);
                        set_selected.set(None);
                    }
                    Err(e) => {
                        leptos::logging::log!("Invalid move: {:?}", e);
                    }
                }
            }
        } else if let Some(p) = clicked_piece {
            if p.color == current_turn {
                set_selected.set(Some((row, col)));
            }
        }
    };

    view! {
        <div style="display: flex; flex-direction: column; align-items: center; width: 100%; padding: 5px; box-sizing: border-box;">
            <div style=container_style>
                // Layer 1: SVG Board Lines
                <svg viewBox="0 0 450 500" style="display: block; width: 100%; height: 100%; z-index: 1;">
                    // Background
                    <rect x="0" y="0" width="450" height="500" fill="#eecfa1" />

                    // Grid Lines
                    {
                        let cell = 50;
                        let half = 25;
                        let width = 450;

                        let mut lines = Vec::new();
                        // Horizontal lines
                        for r in 0..10 {
                            let y = r * cell + half;
                            let x1 = half;
                            let x2 = width - half;
                            lines.push(view! { <line x1=x1 y1=y x2=x2 y2=y stroke="#5c3a1e" stroke-width="2" /> });
                        }

                        // Vertical lines
                        for c in 0..9 {
                            let x = c * cell + half;
                            let y_top_start = half;
                            let y_top_end = 4 * cell + half;
                            let y_bot_start = 5 * cell + half;
                            let y_bot_end = 9 * cell + half;

                            if c == 0 || c == 8 {
                                lines.push(view! { <line x1=x y1=y_top_start x2=x y2=y_bot_end stroke="#5c3a1e" stroke-width="2" /> });
                            } else {
                                lines.push(view! { <line x1=x y1=y_top_start x2=x y2=y_top_end stroke="#5c3a1e" stroke-width="2" /> });
                                lines.push(view! { <line x1=x y1=y_bot_start x2=x y2=y_bot_end stroke="#5c3a1e" stroke-width="2" /> });
                            }
                        }

                        // Palaces
                        let p_start = 3 * cell + half;
                        let p_end = 5 * cell + half;
                        let r0 = half;
                        let r2 = 2 * cell + half;
                        lines.push(view! { <line x1=p_start y1=r0 x2=p_end y2=r2 stroke="#5c3a1e" stroke-width="2" /> });
                        lines.push(view! { <line x1=p_end y1=r0 x2=p_start y2=r2 stroke="#5c3a1e" stroke-width="2" /> });

                        let r7 = 7 * cell + half;
                        let r9 = 9 * cell + half;
                        lines.push(view! { <line x1=p_start y1=r7 x2=p_end y2=r9 stroke="#5c3a1e" stroke-width="2" /> });
                        lines.push(view! { <line x1=p_end y1=r7 x2=p_start y2=r9 stroke="#5c3a1e" stroke-width="2" /> });

                        lines
                    }

                    // Last Move Trajectory Line
                    {move || {
                        let state = game_state.get();
                        if let Some(((fr, fc), (tr, tc))) = state.last_move {
                            let cell = 50;
                            let half = 25;

                            // Map logic coordinates to visual coordinates
                            // Logic Row 0 -> Visual Y = 475 (Bottom)
                            // Logic Row 9 -> Visual Y = 25 (Top)
                            let x1 = fc * cell + half;
                            let y1 = (9 - fr) * cell + half;
                            let x2 = tc * cell + half;
                            let y2 = (9 - tr) * cell + half;

                            view! {
                                <line
                                    x1=x1 y1=y1
                                    x2=x2 y2=y2
                                    stroke="rgba(255, 165, 0, 0.6)"
                                    stroke-width="6"
                                    stroke-linecap="round"
                                    style="pointer-events: none;"
                                />
                                // Optional: Dot at start
                                <circle cx=x1 cy=y1 r="5" fill="rgba(255, 165, 0, 0.6)" />
                                // Optional: Dot at end
                                <circle cx=x2 cy=y2 r="5" fill="rgba(255, 165, 0, 0.6)" />
                            }.into_view()
                        } else {
                            view!{}.into_view()
                        }
                    }}

                    <text x="112.5" y="258" font-family="serif" font-size="24" fill="#5c3a1e" text-anchor="middle" style="opacity: 0.5;">"楚 河"</text>
                    <text x="337.5" y="258" font-family="serif" font-size="24" fill="#5c3a1e" text-anchor="middle" style="opacity: 0.5;">"漢 界"</text>
                </svg>

                // Layer 2: Pieces (Interactive)
                <div style=pieces_layer_style>
                    {move || {
                        let state = game_state.get();
                        let mut cells = Vec::new();

                        for row in (0..10).rev() {
                            for col in 0..9 {
                                let piece = state.board.get_piece(row, col);
                                let is_selected = selected.get() == Some((row, col));

                                let is_last_move = if let Some(((from_r, from_c), (to_r, to_c))) = state.last_move {
                                    (row == from_r && col == from_c) || (row == to_r && col == to_c)
                                } else {
                                    false
                                };

                                let should_animate = anim_dest.get() == Some((row, col));

                                cells.push(view! {
                                    <div
                                        style="display: flex; justify_content: center; align_items: center; cursor: pointer; position: relative;"
                                        on:click=move |_| handle_click(row, col)
                                    >
                                        {if is_last_move {
                                            view! { <div style="position: absolute; width: 100%; height: 100%; background-color: rgba(255, 255, 0, 0.4); z-index: 15; border-radius: 50%;"></div> }.into_view()
                                        } else {
                                            view! {}.into_view()
                                        }}

                                        {render_piece(piece, is_selected, should_animate)}
                                    </div>
                                });
                            }
                        }
                        cells
                    }}
                </div>
            </div>

            <div class="status" style="margin-top: 10px; font-size: 1.2em; color: #eee;">
                {move || {
                    let state = game_state.get();
                    match state.status {
                        crate::logic::game::GameStatus::Playing => format!("Turn: {:?}", state.turn),
                        crate::logic::game::GameStatus::Checkmate(winner) => format!("Checkmate! {winner:?} Wins!"),
                        crate::logic::game::GameStatus::Stalemate => "Stalemate!".to_string(),
                    }
                }}
            </div>
        </div>
    }
}

fn render_piece(piece: Option<Piece>, is_selected: bool, is_last_move_dest: bool) -> impl IntoView {
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

            // Animation class for slam effect
            let anim_class = if is_last_move_dest { "piece-slam" } else { "" };

            view! {
                <style>
                    "
                    .piece-hover {
                        transition: transform 0.2s cubic-bezier(0.34, 1.56, 0.64, 1);
                    }
                    .piece-hover:hover {
                        transform: translateY(-5px) scale(1.1) !important;
                        box-shadow: 0 15px 25px rgba(0,0,0,0.4) !important;
                        z-index: 100 !important;
                    }
                    
                    @keyframes slam {
                        0% { transform: scale(1.5); opacity: 0; }
                        50% { transform: scale(1.1); opacity: 1; }
                        100% { transform: scale(1); }
                    }
                    
                    .piece-slam {
                        animation: slam 0.4s cubic-bezier(0.175, 0.885, 0.32, 1.275);
                    }
                    "
                </style>
                <div
                    class=format!("piece-hover {}", anim_class)
                    style=format!("
                    width: 90%; 
                    height: 90%; 
                    border-radius: 50%; 
                    background-color: {}; 
                    color: {}; 
                    border: 2px solid {};
                    display: grid; 
                    place-items: center; 
                    line-height: 1; 
                    font-size: clamp(14px, 4.5cqw, 45px); 
                    container-type: size;
                    font-family: 'KaiTi', '楷体', serif;
                    font-weight: bold;
                    z-index: 20;
                    {};
                    {}
                ", bg_color, color, border_color, shadow, scale)>
                    <span style="font-size: 60cqw;">{symbol}</span>
                </div>
            }
            .into_view()
        }
        None => view! {
            <div style=format!("
                width: 90%; 
                height: 90%; 
                border-radius: 50%; 
                opacity: 0.3;
                background-color: {};
            ", if is_selected { "#a8e6cf" } else { "transparent" })></div> 
        }
        .into_view(),
    }
}
