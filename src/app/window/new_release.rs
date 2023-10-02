use egui::{Context, ScrollArea, Window};
use egui_commonmark::{CommonMarkCache, CommonMarkViewer};

use crate::{horizontal_separator_empty, update::LatestReleaseInfo};

pub fn new_release_window(
    ctx: &Context,
    release_info: &mut LatestReleaseInfo,
    cache: &mut CommonMarkCache,
) {
    // The show update flag will be permanently false once closed.
    if let Some(new_release_tag) = &release_info.new_release_tag {
        Window::new("New Release Available")
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .resizable(false)
            .collapsible(false)
            .open(&mut release_info.should_show_update)
            .show(ctx, |ui| {
                ui.label("A new release of SSBH Editor is available!");
                ui.label(format!(
                    "The latest version is {}. The current version is {}.",
                    new_release_tag,
                    env!("CARGO_PKG_VERSION")
                ));
                ui.label("Download the new version from here:");
                let release_link = "https://github.com/ScanMountGoat/ssbh_editor/releases";
                if ui.hyperlink(release_link).clicked() {
                    if let Err(e) = open::that(release_link) {
                        log::error!("Failed to open {release_link}: {e}");
                    }
                }
                horizontal_separator_empty(ui);

                ScrollArea::vertical().show(ui, |ui| {
                    if let Some(release_notes) = &release_info.release_notes {
                        CommonMarkViewer::new("release_markdown").show(ui, cache, release_notes);
                    }
                });
            });
    }
}
