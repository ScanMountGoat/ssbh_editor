use crate::path::{application_dir, preferences_file};
use log::error;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct AppPreferences {
    pub dark_mode: bool,
    pub autohide_expressions: bool,
}

impl AppPreferences {
    pub fn load_from_file() -> Self {
        let path = preferences_file();
        let mut bytes = std::fs::read(&path);
        if bytes.is_err() {
            Self::default().write_to_file();

            // Read again to avoid showing an error after writing default preferences.
            bytes = std::fs::read(&path);
        }

        bytes
            .and_then(|data| Ok(serde_json::from_slice(&data)?))
            .map_err(|e| {
                error!("Failed to load preferences from {:?}: {}", &path, e);
                e
            })
            .unwrap_or_else(|_| AppPreferences::default())
    }

    pub fn write_to_file(&self) {
        let path = preferences_file();
        // TODO: Give a visual indication that the file saved?
        match serde_json::to_string_pretty(self) {
            Ok(json) => {
                if let Err(e) = std::fs::write(&path, json) {
                    error!("Failed to write preferences to {:?}: {}", &path, e);
                }
            }
            Err(e) => error!("Failed to serialize preferences: {}", e),
        }
    }
}

impl Default for AppPreferences {
    fn default() -> Self {
        Self {
            dark_mode: true,
            autohide_expressions: false,
        }
    }
}

pub fn preferences_window(ctx: &egui::Context, preferences: &mut AppPreferences, open: &mut bool) {
    egui::Window::new("Preferences")
        .open(open)
        .resizable(false)
        .show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                egui::menu::menu_button(ui, "File", |ui| {
                    if ui
                        .add(egui::Button::new("Open preferences directory...").wrap(false))
                        .clicked()
                    {
                        ui.close_menu();

                        let path = application_dir();
                        if let Err(e) = open::that(&path) {
                            log::error!("Failed to open {path:?}: {e}");
                        }
                    }
                });
            });
            ui.separator();

            // TODO: Add a toggle widget instead.
            ui.checkbox(&mut preferences.dark_mode, "Dark Mode");
            ui.checkbox(
                &mut preferences.autohide_expressions,
                "Automatically Hide Expressions",
            );
        });
}
