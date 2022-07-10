use egui::ScrollArea;
use log::error;
use rfd::FileDialog;
use ssbh_data::prelude::*;

pub fn adj_editor(ctx: &egui::Context, title: &str, adj: &mut AdjData) -> bool {
    let mut open = true;

    egui::Window::new(format!("Adj Editor ({title})"))
        .open(&mut open)
        .resizable(true)
        .show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                egui::menu::menu_button(ui, "File", |ui| {
                    if ui.button("Save").clicked() {
                        ui.close_menu();

                        if let Some(file) =
                            FileDialog::new().add_filter("Adj", &["adjb"]).save_file()
                        {
                            if let Err(e) = adj.write_to_file(file) {
                                error!("Failed to save Adj (.adjb): {}", e);
                            }
                        }
                    }
                });
            });
            ui.add(egui::Separator::default().horizontal());

            ScrollArea::vertical()
                .auto_shrink([false; 2])
                .show(ui, |ui| {
                    egui::Grid::new("adj_grid").show(ui, |ui| {
                        // TODO: How to best display adjacency data?
                        // TODO: Show info from mesh if present?
                        ui.heading("Mesh Object Index");
                        ui.heading("Vertex Adjacency Count");
                        ui.end_row();

                        for entry in &adj.entries {
                            ui.label(entry.mesh_object_index.to_string());
                            ui.label(entry.vertex_adjacency.len().to_string());
                            ui.end_row();
                        }
                    });
                });
        });

    open
}
