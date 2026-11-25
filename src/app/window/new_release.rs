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
                        if let Some(version) = &release_info.installed_update_version {
                            if ui
                                .button("Restart")
                                .on_hover_text("Restart the newly updated application")
                                .clicked()
                            {
                                // TODO: restart with the open folders?
                                let Err(e) = self_update::restart::restart();
                                error!("Failed to restart after update: {e}");
                                release_info.finished_update = false;
                                ui.close();
                            }
                            if ui
                                .button("Close")
                                .on_hover_text("Close this window to restart manually later")
                                .clicked()
                            {
                                release_info.finished_update = false;
                                ui.close();
                            }

                            ui.label(format!(
                                "Updated to version {version}. Restart for this to take effect."
                            ));
                        } else {
                            if ui
                                .button("Update")
                                .on_hover_text(
                                    "Download and replace this executable with the latest release",
                                )
                                .clicked()
                            {
                                // TODO: show progress indication
                                if let Some(update) = &new_release.update {
                                    match update.update() {
                                        Ok(status) => {
                                            if status.is_updated() {
                                                release_info.finished_update = true;
                                                release_info.installed_update_version =
                                                    Some(status.version().to_string());
                                            }
                                        }
                                        Err(e) => {
                                            error!(
                                                "Failed to update to version {}: {e}",
                                                new_release.tag
                                            );
                                        }
                                    }
                                }
                            }
                            if ui
                                .button("Remind Me Later")
                                .on_hover_text("Cancel the update until the next update check")
                                .clicked()
                            {
                                ui.close();
                            }
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
