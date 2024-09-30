use std::str::FromStr;

use crate::{
    path::application_dir,
    preferences::{AppPreferences, GraphicsBackend},
};

use egui::{
    special_emojis::{OS_APPLE, OS_LINUX, OS_WINDOWS},
    DragValue, TextWrapMode, Ui, Window,
};
use strum::VariantNames;

pub fn preferences_window(
    ctx: &egui::Context,
    preferences: &mut AppPreferences,
    open: &mut bool,
) -> bool {
    let mut changed = false;

    Window::new("Preferences")
        .open(open)
        .resizable(false)
        .show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui
                        .add(
                            egui::Button::new("Open Preferences Directory...")
                                .wrap_mode(TextWrapMode::Extend),
                        )
                        .clicked()
                    {
                        ui.close_menu();

                        let path = application_dir();
                        if let Err(e) = open::that(path) {
                            log::error!("Failed to open {path:?}: {e}");
                        }
                    }
                });

                ui.menu_button("Help", |ui| {
                    if ui.button("Preferences Wiki").clicked() {
                        ui.close_menu();

                        let link = "https://github.com/ScanMountGoat/ssbh_editor/wiki/Preferences";
                        if let Err(e) = open::that(link) {
                            log::error!("Failed to open {link}: {e}");
                        }
                    }
                });
            });
            ui.separator();

            changed |= edit_preferences(ui, preferences);
        });
    changed
}

fn edit_preferences(ui: &mut Ui, preferences: &mut AppPreferences) -> bool {
    let mut changed = false;
    // TODO: Add a toggle widget instead.
    changed |= ui
        .checkbox(&mut preferences.dark_mode, "Dark Mode")
        .changed();
    ui.horizontal(|ui| {
        changed |= ui
            .color_edit_button_srgb(&mut preferences.viewport_color)
            .changed();
        ui.label("Viewport Background");
    });
    changed |= ui
        .checkbox(
            &mut preferences.autohide_expressions,
            "Automatically Hide Expressions",
        )
        .changed();
    changed |= ui
        .checkbox(
            &mut preferences.autohide_ink_meshes,
            "Automatically Hide Ink Meshes",
        )
        .changed();
    ui.horizontal(|ui| {
        ui.label("Graphics Backend").on_hover_text(
            "The preferred graphics backend. Requires an application restart to take effect.",
        );

        changed |= edit_graphics_backend(&mut preferences.graphics_backend, ui);
    });

    ui.horizontal(|ui| {
        ui.label("UI Scale");
        changed |= ui
            .add(
                DragValue::new(&mut preferences.scale_factor)
                    .update_while_editing(false)
                    .speed(0.05)
                    .range(0.5..=2.0),
            )
            .changed();
    });

    if ui.button("Reset Preferences").clicked() {
        *preferences = AppPreferences::default();
        changed = true;
    }

    changed
}

fn edit_graphics_backend(graphics_backend: &mut GraphicsBackend, ui: &mut Ui) -> bool {
    let backend_label = |b: &GraphicsBackend| match b {
        GraphicsBackend::Auto => "Auto".to_owned(),
        GraphicsBackend::Vulkan => format!("{OS_WINDOWS} {OS_LINUX} Vulkan"),
        GraphicsBackend::Metal => format!("{OS_APPLE} Metal"),
        GraphicsBackend::Dx12 => format!("{OS_WINDOWS} DX12"),
    };

    let mut changed = false;

    // TODO: Create a helper function for custom variant labels on enums?
    // TODO: Limit backends based on the current platform.
    egui::ComboBox::from_id_salt("graphics_backend")
        .width(200.0)
        .selected_text(backend_label(graphics_backend))
        .show_ui(ui, |ui| {
            for v in GraphicsBackend::VARIANTS {
                let variant = GraphicsBackend::from_str(v).unwrap();
                let label = backend_label(&variant);
                changed |= ui
                    .selectable_value(graphics_backend, variant, label)
                    .changed();
            }
        });

    changed
}
