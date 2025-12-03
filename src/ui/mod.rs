use bevy::prelude::*;

mod board;
mod panels;

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(board::BoardPlugin)
            .add_plugins(panels::PanelsPlugin);
    }
}
