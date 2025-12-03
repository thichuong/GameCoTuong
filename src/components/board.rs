use cotuong_core::logic::board::{Color, Piece, PieceType};
use cotuong_core::logic::game::GameState;
use leptos::{
    component, create_effect, create_node_ref, create_signal, html::Canvas, view, IntoView,
    NodeRef, ReadSignal, SignalGet, SignalSet, SignalWith, WriteSignal,
};
use std::ops::Deref;
use wasm_bindgen::JsCast;
use web_sys::CanvasRenderingContext2d;

// Board constants
const CELL_SIZE: f64 = 50.0;
const PADDING: f64 = 25.0;
const BOARD_WIDTH: f64 = 450.0;
const BOARD_HEIGHT: f64 = 500.0;

// Helper to get symbol
fn get_piece_symbol(p: PieceType, c: Color) -> &'static str {
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
}

#[allow(deprecated)]
fn draw_piece(
    ctx: &CanvasRenderingContext2d,
    row: usize,
    col: usize,
    piece: Piece,
    is_selected: bool,
) {
    // Map logic row to visual y (row 0 is bottom)
    #[allow(clippy::cast_possible_truncation)]
    let x = f64::from(col as u32).mul_add(CELL_SIZE, PADDING);
    #[allow(clippy::cast_possible_truncation)]
    let y = f64::from((9 - row) as u32).mul_add(CELL_SIZE, PADDING);

    let radius = 23.0; // Increased from 20.0

    // Shadow/Selection
    if is_selected {
        ctx.set_shadow_blur(15.0);
        ctx.set_shadow_color("#ffeb3b");
    } else {
        ctx.set_shadow_blur(5.0); // Slightly increased shadow
        ctx.set_shadow_color("rgba(0,0,0,0.5)");
    }

    // Body
    ctx.begin_path();
    let _ = ctx.arc(x, y, radius, 0.0, std::f64::consts::PI * 2.0);
    ctx.set_fill_style(&"#f0d9b5".into());
    ctx.fill();

    // Reset shadow
    ctx.set_shadow_blur(0.0);

    // Border
    let color_str = if piece.color == Color::Red {
        "#c00"
    } else {
        "#000"
    };
    ctx.set_stroke_style(&color_str.into());
    ctx.set_line_width(2.0);
    ctx.stroke();

    // Inner Ring Detail
    ctx.begin_path();
    let _ = ctx.arc(x, y, radius - 4.0, 0.0, std::f64::consts::PI * 2.0);
    ctx.set_line_width(1.0);
    ctx.stroke();

    // Text
    ctx.set_fill_style(&color_str.into());
    ctx.set_font("bold 32px KaiTi, serif"); // Increased from 24px
    ctx.set_text_align("center");
    ctx.set_text_baseline("middle");
    // Adjust baseline slightly for visual centering if needed, but middle is usually good

    let symbol = get_piece_symbol(piece.piece_type, piece.color);
    // Small vertical adjustment for font rendering
    let _ = ctx.fill_text(symbol, x, y + 2.0);
}

#[component]
#[allow(clippy::too_many_lines)]
#[allow(deprecated)]
pub fn BoardView(
    game_state: ReadSignal<GameState>,
    set_game_state: WriteSignal<GameState>,
) -> impl IntoView {
    let (selected, set_selected) = create_signal(Option::<(usize, usize)>::None);
    let canvas_ref: NodeRef<Canvas> = create_node_ref();

    // Draw function
    let draw = move || {
        let Some(canvas) = canvas_ref.get() else {
            return;
        };
        let Some(window) = web_sys::window() else {
            return;
        };
        let ratio = window.device_pixel_ratio();

        let width = BOARD_WIDTH;
        let height = BOARD_HEIGHT;

        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        canvas.set_width((width * ratio) as u32);
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        canvas.set_height((height * ratio) as u32);

        let Ok(Some(ctx)) = canvas
            .get_context("2d")
            .map(|res| res.and_then(|o| o.dyn_into::<CanvasRenderingContext2d>().ok()))
        else {
            return;
        };
        let ctx: CanvasRenderingContext2d = ctx;

        let _ = ctx.scale(ratio, ratio);

        // Clear canvas
        ctx.set_fill_style(&"#eecfa1".into());
        ctx.fill_rect(0.0, 0.0, BOARD_WIDTH, BOARD_HEIGHT);

        // Draw Grid
        ctx.set_stroke_style(&"#5c3a1e".into());
        ctx.set_line_width(2.0);

        // Horizontal lines
        for r in 0..10 {
            let y = f64::from(r).mul_add(CELL_SIZE, PADDING);
            ctx.begin_path();
            ctx.move_to(PADDING, y);
            ctx.line_to(BOARD_WIDTH - PADDING, y);
            ctx.stroke();
        }

        // Vertical lines
        for c in 0..9 {
            let x = f64::from(c).mul_add(CELL_SIZE, PADDING);

            // Top half
            ctx.begin_path();
            ctx.move_to(x, PADDING);
            ctx.line_to(x, 4.0f64.mul_add(CELL_SIZE, PADDING));
            ctx.stroke();

            // Bottom half
            ctx.begin_path();
            ctx.move_to(x, 5.0f64.mul_add(CELL_SIZE, PADDING));
            ctx.line_to(x, BOARD_HEIGHT - PADDING);
            ctx.stroke();
        }

        // Vertical lines sides (connect top and bottom)
        ctx.begin_path();
        ctx.move_to(PADDING, 4.0f64.mul_add(CELL_SIZE, PADDING));
        ctx.line_to(PADDING, 5.0f64.mul_add(CELL_SIZE, PADDING));
        ctx.stroke();

        ctx.begin_path();
        ctx.move_to(BOARD_WIDTH - PADDING, 4.0f64.mul_add(CELL_SIZE, PADDING));
        ctx.line_to(BOARD_WIDTH - PADDING, 5.0f64.mul_add(CELL_SIZE, PADDING));
        ctx.stroke();

        // Palaces
        // Top
        ctx.begin_path();
        ctx.move_to(3.0f64.mul_add(CELL_SIZE, PADDING), PADDING);
        ctx.line_to(
            5.0f64.mul_add(CELL_SIZE, PADDING),
            2.0f64.mul_add(CELL_SIZE, PADDING),
        );
        ctx.stroke();

        ctx.begin_path();
        ctx.move_to(5.0f64.mul_add(CELL_SIZE, PADDING), PADDING);
        ctx.line_to(
            3.0f64.mul_add(CELL_SIZE, PADDING),
            2.0f64.mul_add(CELL_SIZE, PADDING),
        );
        ctx.stroke();

        // Bottom
        ctx.begin_path();
        ctx.move_to(
            3.0f64.mul_add(CELL_SIZE, PADDING),
            7.0f64.mul_add(CELL_SIZE, PADDING),
        );
        ctx.line_to(
            5.0f64.mul_add(CELL_SIZE, PADDING),
            9.0f64.mul_add(CELL_SIZE, PADDING),
        );
        ctx.stroke();

        ctx.begin_path();
        ctx.move_to(
            5.0f64.mul_add(CELL_SIZE, PADDING),
            7.0f64.mul_add(CELL_SIZE, PADDING),
        );
        ctx.line_to(
            3.0f64.mul_add(CELL_SIZE, PADDING),
            9.0f64.mul_add(CELL_SIZE, PADDING),
        );
        ctx.stroke();

        // River Text
        ctx.set_font("24px serif");
        ctx.set_fill_style(&"#5c3a1e".into());
        ctx.set_text_align("center");
        let _ = ctx.fill_text("楚 河", 112.5 + PADDING, 250.0 + 8.0);
        let _ = ctx.fill_text("漢 界", 337.5 - PADDING, 250.0 + 8.0);

        // Draw Pieces
        let state = game_state.get();
        for r in 0..10 {
            for c in 0..9 {
                if let Some(piece) = state.board.get_piece(r, c) {
                    draw_piece(&ctx, r, c, piece, selected.get() == Some((r, c)));
                }
            }
        }

        // Draw Last Move
        if let Some(((fr, fc), (tr, tc))) = state.last_move {
            #[allow(clippy::cast_possible_truncation)]
            let x1 = f64::from(fc as u32).mul_add(CELL_SIZE, PADDING);
            #[allow(clippy::cast_possible_truncation)]
            let y1 = f64::from((9 - fr) as u32).mul_add(CELL_SIZE, PADDING);
            #[allow(clippy::cast_possible_truncation)]
            let x2 = f64::from(tc as u32).mul_add(CELL_SIZE, PADDING);
            #[allow(clippy::cast_possible_truncation)]
            let y2 = f64::from((9 - tr) as u32).mul_add(CELL_SIZE, PADDING);

            ctx.set_stroke_style(&"rgba(255, 165, 0, 0.6)".into());
            ctx.set_line_width(6.0);
            ctx.set_line_cap("round");
            ctx.begin_path();
            ctx.move_to(x1, y1);
            ctx.line_to(x2, y2);
            ctx.stroke();

            // Dots
            ctx.set_fill_style(&"rgba(255, 165, 0, 0.6)".into());
            ctx.begin_path();
            let _ = ctx.arc(x1, y1, 5.0, 0.0, std::f64::consts::PI * 2.0);
            ctx.fill();
            ctx.begin_path();
            let _ = ctx.arc(x2, y2, 5.0, 0.0, std::f64::consts::PI * 2.0);
            ctx.fill();
        }
    };

    // Effect to redraw when game state changes
    create_effect(move |_| {
        game_state.track(); // Track changes
        selected.track();
        draw();
    });

    let on_click = move |ev: web_sys::MouseEvent| {
        let Some(canvas) = canvas_ref.get() else {
            return;
        };
        let rect = canvas.deref().get_bounding_client_rect();
        let scale_x = BOARD_WIDTH / rect.width();
        let scale_y = BOARD_HEIGHT / rect.height();

        let click_x = (f64::from(ev.client_x()) - rect.left()) * scale_x;
        let click_y = (f64::from(ev.client_y()) - rect.top()) * scale_y;

        // Convert to board coords
        // x = col * CELL + PADDING => col = (x - PADDING) / CELL
        // y = (9 - row) * CELL + PADDING => row = 9 - (y - PADDING) / CELL

        #[allow(clippy::cast_possible_truncation)]
        let col = ((click_x - PADDING + CELL_SIZE / 2.0) / CELL_SIZE).floor() as isize;
        #[allow(clippy::cast_possible_truncation)]
        let row_visual = ((click_y - PADDING + CELL_SIZE / 2.0) / CELL_SIZE).floor() as isize;
        let row = 9 - row_visual;

        if (0..9).contains(&col) && (0..10).contains(&row) {
            #[allow(clippy::cast_sign_loss)]
            let r = row as usize;
            #[allow(clippy::cast_sign_loss)]
            let c = col as usize;

            // Handle move logic (same as before)
            let state = game_state.get();
            let current_turn = state.turn;
            let clicked_piece = state.board.get_piece(r, c);

            if let Some((from_row, from_col)) = selected.get() {
                if from_row == r && from_col == c {
                    set_selected.set(None);
                } else if let Some(p) = clicked_piece {
                    if p.color == current_turn {
                        set_selected.set(Some((r, c)));
                    } else {
                        let mut new_state = state;
                        match new_state.make_move(from_row, from_col, r, c) {
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
                    let mut new_state = state;
                    match new_state.make_move(from_row, from_col, r, c) {
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
                    set_selected.set(Some((r, c)));
                }
            }
        }
    };

    let captured_row_style = "
        display: flex;
        justify-content: center;
        gap: 5px;
        width: 100%;
        min-height: 30px;
        margin: 5px 0;
        flex-wrap: wrap;
    ";

    let captured_piece_style = |c: Color| {
        format!(
            "
        width: 28px;
        height: 28px;
        border-radius: 50%;
        background-color: #f0d9b5;
        color: {};
        border: 1px solid {};
        display: flex;
        justify-content: center;
        align-items: center;
        font-family: 'KaiTi', '楷体', serif;
        font-weight: bold;
        font-size: 18px;
        line-height: 1;
        box-shadow: 1px 1px 2px rgba(0,0,0,0.3);
    ",
            if c == Color::Red { "#c00" } else { "#000" },
            if c == Color::Red { "#c00" } else { "#000" }
        )
    };

    view! {
        <div style="display: flex; flex-direction: column; align-items: center; width: 100%; padding: 5px; box-sizing: border-box;">
            // Black's Lost Pieces (Top)
            <div style=captured_row_style>
                {move || {
                    let state = game_state.get();
                    let mut black_lost = Vec::new();
                    for record in &state.history {
                        if let Some(p) = record.captured {
                            if p.color == Color::Black {
                                black_lost.push(p);
                            }
                        }
                    }
                    black_lost.iter().map(|p| {
                        view! {
                            <div style=captured_piece_style(Color::Black)>
                                {get_piece_symbol(p.piece_type, Color::Black)}
                            </div>
                        }
                    }).collect::<Vec<_>>()
                }}
            </div>

            <canvas
                _ref=canvas_ref
                width=450
                height=500
                style="width: min(96vw, 72vh); aspect-ratio: 9/10; background-color: #eecfa1; border: 2px solid #5c3a1e; box-shadow: 0 5px 15px rgba(0,0,0,0.3); cursor: pointer; -webkit-tap-highlight-color: transparent;"
                on:click=on_click
            />

            // Red's Lost Pieces (Bottom)
            <div style=captured_row_style>
                {move || {
                    let state = game_state.get();
                    let mut red_lost = Vec::new();
                    for record in &state.history {
                        if let Some(p) = record.captured {
                            if p.color == Color::Red {
                                red_lost.push(p);
                            }
                        }
                    }
                    red_lost.iter().map(|p| {
                        view! {
                            <div style=captured_piece_style(Color::Red)>
                                {get_piece_symbol(p.piece_type, Color::Red)}
                            </div>
                        }
                    }).collect::<Vec<_>>()
                }}
            </div>

            <div class="status" style="margin-top: 10px; font-size: 1.2em; color: #eee;">
                {move || {
                    let state = game_state.get();
                    match state.status {
                        cotuong_core::logic::game::GameStatus::Playing => format!("Turn: {:?}", state.turn),
                        cotuong_core::logic::game::GameStatus::Checkmate(winner) => format!("Checkmate! {winner:?} Wins!"),
                        cotuong_core::logic::game::GameStatus::Stalemate => "Stalemate!".to_string(),
                    }
                }}
            </div>
        </div>
    }
}
