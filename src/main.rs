use bevy::prelude::*;
use bevy::render::settings::{Backends, RenderCreation, WgpuSettings};
use bevy::render::RenderPlugin;
use bevy_egui::EguiPlugin;
// use cotuong_core::logic::game::GameState;

mod resources;
mod systems;
mod ui;

use resources::AppConfig;
use systems::game::GamePlugin;
use ui::UiPlugin;

fn main() {
    App::new()
        .add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "Co Tuong (Xiangqi)".into(),
                        resolution: (1280.0, 720.0).into(),
                        ..default()
                    }),
                    ..default()
                })
                .set(RenderPlugin {
                    render_creation: RenderCreation::Automatic(WgpuSettings {
                        // Prioritize Vulkan on Linux to avoid EGL errors
                        backends: Some(Backends::VULKAN),
                        ..default()
                    }),
                    ..default()
                }),
        )
        .add_plugins(EguiPlugin)
        .add_plugins(GamePlugin)
        .add_plugins(UiPlugin)
        .init_resource::<AppConfig>()
        .init_resource::<resources::GameStateWrapper>()
        .run();
}
