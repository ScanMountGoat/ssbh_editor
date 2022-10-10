use crate::{path::preferences_file, widgets_dark};
use log::error;
use serde::{Deserialize, Serialize};

// Use defaults for missing values to avoid most version conflicts.
#[derive(Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct AppPreferences {
    pub dark_mode: bool,
    pub autohide_expressions: bool,
    pub viewport_color: [u8; 3],
    pub recent_folders: Vec<String>
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
        let color = widgets_dark().noninteractive.bg_fill;
        Self {
            dark_mode: true,
            autohide_expressions: false,
            viewport_color: [color.r(), color.g(), color.b()],
            recent_folders: Vec::new()
        }
    }
}
