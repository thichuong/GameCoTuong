#![allow(clippy::cast_precision_loss)]
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_sign_loss)]
use crate::resources::GameStateWrapper;
use bevy::prelude::*;

use cotuong_core::logic::board::{Color, PieceType};

pub struct BoardPlugin;

impl Plugin for BoardPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PieceAssets>()
            .add_systems(
                Startup,
                (
                    setup_camera,
                    load_piece_assets,
                    spawn_board.after(load_piece_assets),
                ),
            )
            .add_systems(Update, (draw_board, spawn_pieces, handle_input));
    }
}

#[derive(Resource, Default)]
struct PieceAssets {
    textures: std::collections::HashMap<(PieceType, Color), Handle<Image>>,
    board: Handle<Image>,
}

#[allow(clippy::needless_pass_by_value)]
fn load_piece_assets(mut piece_assets: ResMut<PieceAssets>, asset_server: Res<AssetServer>) {
    let pieces = [
        (PieceType::General, "general"),
        (PieceType::Advisor, "advisor"),
        (PieceType::Elephant, "elephant"),
        (PieceType::Horse, "horse"),
        (PieceType::Chariot, "chariot"),
        (PieceType::Cannon, "cannon"),
        (PieceType::Soldier, "soldier"),
    ];

    for (piece_type, name) in pieces {
        // Red
        let red_path = format!("textures/red_{name}.png");
        piece_assets
            .textures
            .insert((piece_type, Color::Red), asset_server.load(red_path));

        // Black
        let black_path = format!("textures/black_{name}.png");
        piece_assets
            .textures
            .insert((piece_type, Color::Black), asset_server.load(black_path));
    }
    piece_assets.board = asset_server.load("textures/board.png");
}

#[allow(clippy::needless_pass_by_value)]
fn spawn_board(mut commands: Commands, piece_assets: Res<PieceAssets>) {
    commands.spawn(SpriteBundle {
        texture: piece_assets.board.clone(),
        transform: Transform::from_xyz(0.0, 0.0, 0.0),
        sprite: Sprite {
            custom_size: Some(Vec2::new(BOARD_WIDTH + 100.0, BOARD_HEIGHT + 100.0)),
            ..default()
        },
        ..default()
    });
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
        let y = (r as f32).mul_add(CELL_SIZE, start_y);
        gizmos.line_2d(
            Vec2::new(start_x, y),
            Vec2::new(start_x + BOARD_WIDTH, y),
            color,
        );
    }

    // Vertical lines
    for c in 0..9 {
        let x = (c as f32).mul_add(CELL_SIZE, start_x);
        // Top half
        gizmos.line_2d(
            Vec2::new(x, 5.0f32.mul_add(CELL_SIZE, start_y)),
            Vec2::new(x, 9.0f32.mul_add(CELL_SIZE, start_y)),
            color,
        );
        // Bottom half
        gizmos.line_2d(
            Vec2::new(x, start_y),
            Vec2::new(x, 4.0f32.mul_add(CELL_SIZE, start_y)),
            color,
        );

        // Connect sides completely
        if c == 0 || c == 8 {
            gizmos.line_2d(
                Vec2::new(x, 4.0f32.mul_add(CELL_SIZE, start_y)),
                Vec2::new(x, 5.0f32.mul_add(CELL_SIZE, start_y)),
                color,
            );
        }
    }

    // Palaces (X shapes)
    // Bottom (Red)
    gizmos.line_2d(
        Vec2::new(3.0f32.mul_add(CELL_SIZE, start_x), start_y),
        Vec2::new(
            5.0f32.mul_add(CELL_SIZE, start_x),
            2.0f32.mul_add(CELL_SIZE, start_y),
        ),
        color,
    );
    gizmos.line_2d(
        Vec2::new(5.0f32.mul_add(CELL_SIZE, start_x), start_y),
        Vec2::new(
            3.0f32.mul_add(CELL_SIZE, start_x),
            2.0f32.mul_add(CELL_SIZE, start_y),
        ),
        color,
    );

    // Top (Black)
    gizmos.line_2d(
        Vec2::new(
            3.0f32.mul_add(CELL_SIZE, start_x),
            7.0f32.mul_add(CELL_SIZE, start_y),
        ),
        Vec2::new(
            5.0f32.mul_add(CELL_SIZE, start_x),
            9.0f32.mul_add(CELL_SIZE, start_y),
        ),
        color,
    );
    gizmos.line_2d(
        Vec2::new(
            5.0f32.mul_add(CELL_SIZE, start_x),
            7.0f32.mul_add(CELL_SIZE, start_y),
        ),
        Vec2::new(
            3.0f32.mul_add(CELL_SIZE, start_x),
            9.0f32.mul_add(CELL_SIZE, start_y),
        ),
        color,
    );
}

#[allow(clippy::needless_pass_by_value)]
fn spawn_pieces(
    mut commands: Commands,
    game_state: Res<GameStateWrapper>,
    query: Query<Entity, With<PieceComponent>>,
    piece_assets: Res<PieceAssets>,
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

    let start_x = -BOARD_WIDTH / 2.0;
    let start_y = -BOARD_HEIGHT / 2.0;

    for r in 0..10 {
        for c in 0..9 {
            if let Some(piece) = game_state.0.board.get_piece(r, c) {
                let x = (c as f32).mul_add(CELL_SIZE, start_x);
                let y = (r as f32).mul_add(CELL_SIZE, start_y);

                if let Some(texture_handle) =
                    piece_assets.textures.get(&(piece.piece_type, piece.color))
                {
                    commands.spawn((
                        SpriteBundle {
                            texture: texture_handle.clone(),
                            transform: Transform::from_xyz(x, y, 1.0),
                            sprite: Sprite {
                                custom_size: Some(Vec2::new(PIECE_SIZE, PIECE_SIZE)),
                                ..default()
                            },
                            ..default()
                        },
                        PieceComponent { row: r, col: c },
                    ));
                }
            }
        }
    }
}

#[allow(clippy::needless_pass_by_value)]
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

                if (0..9).contains(&c) && (0..10).contains(&r) {
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
