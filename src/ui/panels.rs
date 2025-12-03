use crate::resources::{AppConfig, GameStateWrapper};
use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};

use cotuong_core::logic::game::{GameState, GameStatus};

pub struct PanelsPlugin;

impl Plugin for PanelsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, ui_panels);
    }
}

fn ui_panels(
    mut contexts: EguiContexts,
    mut game_state: ResMut<GameStateWrapper>,
    mut app_config: ResMut<AppConfig>,
) {
    egui::SidePanel::right("right_panel")
        .resizable(true)
        .default_width(300.0)
        .show(contexts.ctx_mut(), |ui| {
            ui.heading("Game Controls");
            ui.separator();

            // Game Status
            let status_text = match game_state.0.status {
                GameStatus::Playing => format!("Turn: {:?}", game_state.0.turn),
                GameStatus::Checkmate(winner) => format!("Checkmate! {winner:?} Wins!"),
                GameStatus::Stalemate => "Stalemate!".to_string(),
            };
            ui.label(status_text);

            if ui.button("New Game").clicked() {
                game_state.0 = GameState::new();
            }

            ui.separator();
            ui.heading("Move History");

            egui::ScrollArea::vertical()
                .max_height(300.0)
                .show(ui, |ui| {
                    for (i, record) in game_state.0.history.iter().enumerate() {
                        let turn_text = if i % 2 == 0 { "Red" } else { "Black" };
                        let from = record.from;
                        let to = record.to;
                        let move_str = format!(
                            "{}. {} ({}, {}) -> ({}, {})",
                            i + 1,
                            turn_text,
                            from.0,
                            from.1,
                            to.0,
                            to.1
                        );
                        ui.label(move_str);
                        if let Some(note) = &record.note {
                            ui.label(
                                egui::RichText::new(note)
                                    .size(10.0)
                                    .color(egui::Color32::GRAY),
                            );
                        }
                    }
                });

            ui.separator();
            ui.checkbox(&mut app_config.show_config, "Show Configuration");

            if app_config.show_config {
                ui.heading("Configuration");
                ui.label("Engine config not fully implemented in UI yet.");
                // TODO: Add sliders for engine config
            }
        });
}
