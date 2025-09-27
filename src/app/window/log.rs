use egui::{Context, Label, ScrollArea, Window};

use crate::app::{LOGGER, log_level_icon};

pub fn log_window(ctx: &Context, open: &mut bool) {
    Window::new("Application Log")
        .open(open)
        .resizable(true)
        .show(ctx, |ui| {
            ScrollArea::vertical()
                .auto_shrink([false; 2])
                .show(ui, |ui| {
                    for (level, message) in LOGGER.messages.lock().unwrap().iter() {
                        ui.horizontal(|ui| {
                            log_level_icon(ui, level);
                            // binrw formats backtraces, which isn't supported by egui font rendering.
                            let clean_message = strip_ansi_escapes::strip(message);
                            let clean_message = String::from_utf8_lossy(&clean_message);
                            ui.add(Label::new(clean_message).wrap());
                        });
                    }
                });
        });
}
