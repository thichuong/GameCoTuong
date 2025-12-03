use crate::resources::GameStateWrapper;
use bevy::prelude::*;
use cotuong_core::logic::board::{Color, PieceType};

pub struct BoardPlugin;

impl Plugin for BoardPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_camera)
            .add_systems(Update, (draw_board, spawn_pieces, handle_input));
    }
}

// Constants for board layout
const CELL_SIZE: f32 = 60.0;
const BOARD_WIDTH: f32 = CELL_SIZE * 8.0;
const BOARD_HEIGHT: f32 = CELL_SIZE * 9.0;
const HALF_CELL: f32 = CELL_SIZE / 2.0;
const PIECE_SIZE: f32 = 50.0;

#[derive(Component)]
struct PieceComponent {
    #[allow(dead_code)]
    row: usize,
    #[allow(dead_code)]
    col: usize,
}

fn setup_camera(mut commands: Commands) {
    commands.spawn(Camera2dBundle::default());
}

fn draw_board(mut gizmos: Gizmos) {
    let start_x = -BOARD_WIDTH / 2.0;
    let start_y = -BOARD_HEIGHT / 2.0;
    let color = bevy::prelude::Color::srgb(0.36, 0.23, 0.12); // #5c3a1e

    // Horizontal lines
    for r in 0..10 {
        let y = start_y + r as f32 * CELL_SIZE;
        gizmos.line_2d(
            Vec2::new(start_x, y),
            Vec2::new(start_x + BOARD_WIDTH, y),
            color,
        );
    }

    // Vertical lines
    for c in 0..9 {
        let x = start_x + c as f32 * CELL_SIZE;
        // Top half
        gizmos.line_2d(
            Vec2::new(x, start_y + 5.0 * CELL_SIZE),
            Vec2::new(x, start_y + 9.0 * CELL_SIZE),
            color,
        );
        // Bottom half
        gizmos.line_2d(
            Vec2::new(x, start_y),
            Vec2::new(x, start_y + 4.0 * CELL_SIZE),
            color,
        );

        // Connect sides completely
        if c == 0 || c == 8 {
            gizmos.line_2d(
                Vec2::new(x, start_y + 4.0 * CELL_SIZE),
                Vec2::new(x, start_y + 5.0 * CELL_SIZE),
                color,
            );
        }
    }

    // Palaces (X shapes)
    // Bottom (Red)
    gizmos.line_2d(
        Vec2::new(start_x + 3.0 * CELL_SIZE, start_y),
        Vec2::new(start_x + 5.0 * CELL_SIZE, start_y + 2.0 * CELL_SIZE),
        color,
    );
    gizmos.line_2d(
        Vec2::new(start_x + 5.0 * CELL_SIZE, start_y),
        Vec2::new(start_x + 3.0 * CELL_SIZE, start_y + 2.0 * CELL_SIZE),
        color,
    );

    // Top (Black)
    gizmos.line_2d(
        Vec2::new(start_x + 3.0 * CELL_SIZE, start_y + 7.0 * CELL_SIZE),
        Vec2::new(start_x + 5.0 * CELL_SIZE, start_y + 9.0 * CELL_SIZE),
        color,
    );
    gizmos.line_2d(
        Vec2::new(start_x + 5.0 * CELL_SIZE, start_y + 7.0 * CELL_SIZE),
        Vec2::new(start_x + 3.0 * CELL_SIZE, start_y + 9.0 * CELL_SIZE),
        color,
    );
}

fn spawn_pieces(
    mut commands: Commands,
    game_state: Res<GameStateWrapper>,
    query: Query<Entity, With<PieceComponent>>,
    asset_server: Res<AssetServer>,
) {
    // Ideally, we only update when state changes. For now, simple re-render.
    // Optimization: Diffing or event-based updates.
    // Since we don't have change detection yet, let's just clear and redraw for MVP
    // but this is inefficient.
    // Better: Check if GameState is changed. `Res<GameState>` change detection.
    if !game_state.is_changed() {
        return;
    }

    for entity in &query {
        commands.entity(entity).despawn_recursive();
    }

    let font = asset_server.load("fonts/KaiTi.ttf"); // We need to ensure this font exists or use default
                                                     // Since I don't have the font file, I should use a system font or just basic text.
                                                     // Bevy's default font is FiraMono.

    let start_x = -BOARD_WIDTH / 2.0;
    let start_y = -BOARD_HEIGHT / 2.0;

    for r in 0..10 {
        for c in 0..9 {
            if let Some(piece) = game_state.0.board.get_piece(r, c) {
                let x = start_x + c as f32 * CELL_SIZE;
                let y = start_y + r as f32 * CELL_SIZE;

                let color = match piece.color {
                    Color::Red => bevy::prelude::Color::srgb(0.8, 0.0, 0.0),
                    Color::Black => bevy::prelude::Color::BLACK,
                };

                let symbol = get_piece_symbol(piece.piece_type, piece.color);

                commands
                    .spawn((
                        SpriteBundle {
                            sprite: Sprite {
                                color: bevy::prelude::Color::srgb(0.94, 0.85, 0.71), // #f0d9b5
                                custom_size: Some(Vec2::new(PIECE_SIZE, PIECE_SIZE)),
                                ..default()
                            },
                            transform: Transform::from_xyz(x, y, 1.0),
                            ..default()
                        },
                        PieceComponent { row: r, col: c },
                    ))
                    .with_children(|parent| {
                        parent.spawn(Text2dBundle {
                            text: Text::from_section(
                                symbol,
                                TextStyle {
                                    font: font.clone(),
                                    font_size: 40.0,
                                    color,
                                },
                            ),
                            transform: Transform::from_xyz(0.0, 0.0, 1.0),
                            ..default()
                        });
                    });
            }
        }
    }
}

fn handle_input(
    // mut commands: Commands,
    windows: Query<&Window>,
    camera: Query<(&Camera, &GlobalTransform)>,
    buttons: Res<ButtonInput<MouseButton>>,
    mut game_state: ResMut<GameStateWrapper>,
    mut selected: Local<Option<(usize, usize)>>,
) {
    if buttons.just_pressed(MouseButton::Left) {
        let (camera, camera_transform) = camera.single();
        let window = windows.single();

        if let Some(cursor_position) = window.cursor_position() {
            if let Some(point) = camera.viewport_to_world_2d(camera_transform, cursor_position) {
                let start_x = -BOARD_WIDTH / 2.0;
                let start_y = -BOARD_HEIGHT / 2.0;

                // Convert world pos to grid pos
                // x = start_x + c * CELL_SIZE => c = (x - start_x) / CELL_SIZE
                let c = ((point.x - start_x + HALF_CELL) / CELL_SIZE).floor() as i32;
                let r = ((point.y - start_y + HALF_CELL) / CELL_SIZE).floor() as i32;

                if c >= 0 && c < 9 && r >= 0 && r < 10 {
                    let r = r as usize;
                    let c = c as usize;

                    let current_turn = game_state.0.turn;
                    let clicked_piece = game_state.0.board.get_piece(r, c);

                    if let Some((from_r, from_c)) = *selected {
                        if from_r == r && from_c == c {
                            *selected = None;
                        } else if let Some(p) = clicked_piece {
                            if p.color == current_turn {
                                *selected = Some((r, c));
                            } else {
                                // Capture
                                if game_state.0.make_move(from_r, from_c, r, c).is_ok() {
                                    *selected = None;
                                }
                            }
                        } else {
                            // Move to empty
                            if game_state.0.make_move(from_r, from_c, r, c).is_ok() {
                                *selected = None;
                            }
                        }
                    } else if let Some(p) = clicked_piece {
                        if p.color == current_turn {
                            *selected = Some((r, c));
                        }
                    }
                }
            }
        }
    }
}

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
