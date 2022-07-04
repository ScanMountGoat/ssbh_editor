use egui::ScrollArea;
use log::error;
use rfd::FileDialog;
use ssbh_data::prelude::*;
use ssbh_data::skel_data::*;

pub fn skel_editor(ctx: &egui::Context, title: &str, skel: &mut SkelData) -> bool {
    let mut open = true;

    egui::Window::new(format!("Skel Editor ({title})"))
        .resizable(true)
        .open(&mut open)
        .show(ctx, |ui| {
            ScrollArea::vertical()
                .auto_shrink([false; 2])
                .show(ui, |ui| {
                    egui::menu::bar(ui, |ui| {
                        egui::menu::menu_button(ui, "File", |ui| {
                            if ui.button("Save").clicked() {
                                ui.close_menu();

                                if let Some(file) = FileDialog::new()
                                    .add_filter("Skel", &["nusktb"])
                                    .save_file()
                                {
                                    if let Err(e) = skel.write_to_file(file) {
                                        error!("Failed to save Skel (.nusktb): {}", e);
                                    }
                                }
                            }
                        });
                    });

                    ui.add(egui::Separator::default().horizontal());

                    // TODO: Add options to show a grid or tree based layout?
                    egui::Grid::new("some_unique_id").show(ui, |ui| {
                        // Header
                        ui.heading("Bone");
                        ui.heading("Parent");
                        ui.heading("Billboard Type");
                        ui.end_row();

                        // TODO: Do this without clone?
                        let other_bones = skel.bones.clone();

                        for (i, bone) in skel.bones.iter_mut().enumerate() {
                            ui.label(&bone.name);
                            let parent_bone_name = bone
                                .parent_index
                                .and_then(|i| other_bones.get(i))
                                .map(|p| p.name.as_str())
                                .unwrap_or("None");

                            egui::ComboBox::from_id_source(i)
                                .selected_text(parent_bone_name)
                                .show_ui(ui, |ui| {
                                    ui.selectable_value(&mut bone.parent_index, None, "None");
                                    ui.separator();
                                    // TODO: Is there a way to make this not O(N^2)?
                                    for (other_i, other_bone) in other_bones.iter().enumerate() {
                                        ui.selectable_value(
                                            &mut bone.parent_index,
                                            Some(other_i),
                                            &other_bone.name,
                                        );
                                    }
                                });

                                let billboard_type_str: &str = match bone.billboard_type {
                                    BillboardType::Disabled => "Disabled",
                                    BillboardType::XAxisViewPointAligned => "XAxisViewPointAligned",
                                    BillboardType::YAxisViewPointAligned => "YAxisViewPointAligned",
                                    BillboardType::Unk3 => "Unk3",
                                    BillboardType::XYAxisViewPointAligned => "XYAxisViewPointAligned",
                                    BillboardType::YAxisViewPlaneAligned => "YAxisViewPlaneAligned",
                                    BillboardType::XYAxisViewPlaneAligned => "XYAxisViewPlaneAligned",
                                    _ => "Disabled"
                                };
                            egui::ComboBox::from_id_source(i + 1024)
                                .selected_text(billboard_type_str)
                                .width(200.0)
                                .show_ui(ui, |ui| {
                                    ui.selectable_value(&mut bone.billboard_type, BillboardType::Disabled, "Disabled");
                                    ui.separator();
                                    ui.selectable_value(&mut bone.billboard_type, BillboardType::XAxisViewPointAligned, "XAxisViewPointAligned");
                                    ui.selectable_value(&mut bone.billboard_type, BillboardType::YAxisViewPointAligned, "YAxisViewPointAligned");
                                    ui.selectable_value(&mut bone.billboard_type, BillboardType::Unk3, "Unk3");
                                    ui.selectable_value(&mut bone.billboard_type, BillboardType::XYAxisViewPointAligned, "XYAxisViewPointAligned");
                                    ui.selectable_value(&mut bone.billboard_type, BillboardType::YAxisViewPlaneAligned, "YAxisViewPlaneAligned");
                                    ui.selectable_value(&mut bone.billboard_type, BillboardType::XYAxisViewPlaneAligned, "XYAxisViewPlaneAligned");
                                });
                            ui.end_row();
                        }
                    });
                });
        });

    open
}
