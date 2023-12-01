use crate::{horizontal_separator_empty, TexturePainter};
use egui::{special_emojis::GITHUB, DragValue};
use egui_wgpu::CallbackTrait;
use nutexb::{NutexbFile, NutexbFormat};
use nutexb_wgpu::RenderSettings;

pub fn nutexb_viewer(
    ctx: &egui::Context,
    title: &str,
    nutexb: &NutexbFile,
    settings: &mut RenderSettings,
) -> bool {
    let mut open = true;
    egui::Window::new(format!("Nutexb Viewer ({title})"))
        .open(&mut open)
        .resizable(false)
        .show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("Help", |ui| {
                    if ui.button(format!("{GITHUB} Nutexb Editor Wiki")).clicked() {
                        ui.close_menu();

                        let link =
                            "https://github.com/ScanMountGoat/ssbh_editor/wiki/Nutexb-Editor";
                        if let Err(e) = open::that(link) {
                            log::error!("Failed to open {link}: {e}");
                        }
                    }
                });
            });
            ui.separator();

            ui.heading("Footer");
            egui::Grid::new("nutexb_grid").show(ui, |ui| {
                ui.label("Name");
                ui.label(nutexb.footer.string.to_string());
                ui.end_row();

                ui.label("Width");
                ui.label(nutexb.footer.width.to_string());
                ui.end_row();

                ui.label("Height");
                ui.label(nutexb.footer.height.to_string());
                ui.end_row();

                ui.label("Depth");
                ui.label(nutexb.footer.depth.to_string());
                ui.end_row();

                ui.label("Image Format");
                ui.label(format_name(nutexb.footer.image_format));
                ui.end_row();

                ui.label("Mipmap Count");
                ui.label(nutexb.footer.mipmap_count.to_string());
                ui.end_row();

                ui.label("Layer Count");
                ui.label(nutexb.footer.layer_count.to_string());
                ui.end_row();

                // TODO: Show an error if this doesn't match the actual data?
                // TODO: Show an error if this doesn't match the expected size?
                ui.label("Data Size");
                ui.label(nutexb.footer.data_size.to_string());
                ui.end_row();
            });
            horizontal_separator_empty(ui);

            ui.heading("Image Data");
            ui.horizontal(|ui| {
                ui.checkbox(&mut settings.render_rgba[0], "R");
                ui.checkbox(&mut settings.render_rgba[1], "G");
                ui.checkbox(&mut settings.render_rgba[2], "B");
                ui.checkbox(&mut settings.render_rgba[3], "A");

                // TODO: Show a pixel grid in screen space?
                // TODO: Composite with a background color for alpha?

                if nutexb.footer.mipmap_count > 0 {
                    ui.label("Mipmap");
                    // TODO: Easier to use a slider?
                    ui.add(
                        DragValue::new(&mut settings.mipmap)
                            .clamp_range(0..=nutexb.footer.mipmap_count - 1),
                    );
                }

                // TODO: Disable UI if there are no mips/layers?
                if nutexb.footer.layer_count > 1 {
                    // TODO: Show cube map labels like X+?
                    ui.label("Layer");
                    ui.add(
                        DragValue::new(&mut settings.layer)
                            .clamp_range(0..=nutexb.footer.layer_count - 1),
                    );
                } else if nutexb.footer.depth > 1 {
                    // TODO: Should this be "Slice" instead?
                    ui.label("Depth");
                    ui.add(
                        DragValue::new(&mut settings.layer)
                            .clamp_range(0..=nutexb.footer.depth - 1),
                    );
                }
            });

            egui::Frame::canvas(ui.style()).show(ui, |ui| {
                // Preserve the aspect ratio of the texture.
                // TODO: Make the window resizable?
                let dimensions = if nutexb.footer.width > nutexb.footer.height {
                    egui::Vec2::new(
                        512.0,
                        512.0 * (nutexb.footer.height as f32 / nutexb.footer.width as f32),
                    )
                } else {
                    egui::Vec2::new(
                        512.0 * (nutexb.footer.width as f32 / nutexb.footer.height as f32),
                        512.0,
                    )
                };

                let (_, rect) = ui.allocate_space(dimensions);

                let cb = egui_wgpu::Callback::new_paint_callback(rect, PaintTextureCallback);
                ui.painter().add(cb);
            });
        });
    open
}

struct PaintTextureCallback;

impl CallbackTrait for PaintTextureCallback {
    fn paint<'a>(
        &'a self,
        _info: egui::PaintCallbackInfo,
        render_pass: &mut wgpu::RenderPass<'a>,
        callback_resources: &'a egui_wgpu::CallbackResources,
    ) {
        let painter: &TexturePainter = callback_resources.get().unwrap();
        painter.paint(render_pass);
    }
}

fn format_name(format: NutexbFormat) -> &'static str {
    match format {
        NutexbFormat::R8Unorm => "R8Unorm",
        NutexbFormat::R8G8B8A8Unorm => "R8G8B8A8Unorm",
        NutexbFormat::R8G8B8A8Srgb => "R8G8B8A8Srgb",
        NutexbFormat::R32G32B32A32Float => "R32G32B32A32Float",
        NutexbFormat::B8G8R8A8Unorm => "B8G8R8A8Unorm",
        NutexbFormat::B8G8R8A8Srgb => "B8G8R8A8Srgb",
        NutexbFormat::BC1Unorm => "BC1Unorm",
        NutexbFormat::BC1Srgb => "BC1Srgb",
        NutexbFormat::BC2Unorm => "BC2Unorm",
        NutexbFormat::BC2Srgb => "BC2Srgb",
        NutexbFormat::BC3Unorm => "BC3Unorm",
        NutexbFormat::BC3Srgb => "BC3Srgb",
        NutexbFormat::BC4Unorm => "BC4Unorm",
        NutexbFormat::BC4Snorm => "BC4Snorm",
        NutexbFormat::BC5Unorm => "BC5Unorm",
        NutexbFormat::BC5Snorm => "BC5Snorm",
        NutexbFormat::BC6Ufloat => "BC6Ufloat",
        NutexbFormat::BC6Sfloat => "BC6Sfloat",
        NutexbFormat::BC7Unorm => "BC7Unorm",
        NutexbFormat::BC7Srgb => "BC7Srgb",
    }
}
