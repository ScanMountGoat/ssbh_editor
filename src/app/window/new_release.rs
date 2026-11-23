use egui::{CentralPanel, Context, Modal, Panel, ScrollArea, Sides};
use egui_commonmark::{CommonMarkCache, CommonMarkViewer};
use tracing::error;

use crate::{horizontal_separator_empty, update::LatestReleaseInfo};

pub fn new_release_window(
    ctx: &Context,
    release_info: &mut LatestReleaseInfo,
    cache: &mut CommonMarkCache,
) {
    // The show update flag will be permanently false once closed.
    if let Some(new_release) = &release_info.new_release
        && release_info.should_show_update
    {
        let modal = Modal::new(egui::Id::unique("new_release_modal")).show(ctx, |ui| {
            Panel::bottom("new_release_bottom").show(ui, |ui| {
                Sides::new().show(
                    ui,
                    |_| {},
                    |ui| {
                        if ui.button("Update").clicked() {
                            // TODO: show progress indication
                            // TODO: restart when installation finishes?
                            if let Some(update) = &new_release.update {
                                if let Err(e) = update.update() {
                                    error!("Failed to update to version {}: {e}", new_release.tag);
                                }
                            }
                            ui.close();
                        }
                        if ui.button("Remind Me Later").clicked() {
                            ui.close();
                        }
                    },
                )
            });

            CentralPanel::default().show(ui, |ui| {
                ui.heading("New Release Available");

                ui.label("A new release of SSBH Editor is available!");
                ui.label(format!(
                    "The latest version is {}. The current version is {}.",
                    new_release.tag,
                    env!("CARGO_PKG_VERSION")
                ));
                horizontal_separator_empty(ui);

                ScrollArea::vertical().show(ui, |ui| {
                    if let Some(release_notes) = &new_release.release_notes {
                        CommonMarkViewer::new().show(ui, cache, release_notes);
                    }
                })
            });
        });

        if modal.should_close() {
            release_info.should_show_update = false;
        }
    }
}
