use crate::{RenderState, app::NutexbViewerState, horizontal_separator_empty};
use egui::{ComboBox, DragValue, Scene, Slider, special_emojis::GITHUB};
use egui_wgpu::{Callback, CallbackTrait};
use nutexb::{NutexbFile, NutexbFormat};
use nutexb_wgpu::RenderSettings;

pub fn nutexb_viewer(
    ctx: &egui::Context,
    title: &str,
    state: &mut NutexbViewerState,
    nutexb: &NutexbFile,
    settings: &mut RenderSettings,
) -> bool {
    let mut open = true;
    egui::Window::new(format!("Nutexb Viewer ({title})"))
        .open(&mut open)
        .default_size((500.0, 600.0))
        .show(ctx, |ui| {
            egui::MenuBar::new().ui(ui, |ui| {
                ui.menu_button("Help", |ui| {
                    if ui.button(format!("{GITHUB} Nutexb Editor Wiki")).clicked() {
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

                ui.label("Dimensions");
                ui.label(format!(
                    "{} x {} x {}",
                    nutexb.footer.width, nutexb.footer.height, nutexb.footer.depth
                ));
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

                if nutexb.footer.mipmap_count > 1 {
                    ui.label("Mipmap");
                    let mut mip = settings.mipmap as u32;
                    if ui
                        .add(Slider::new(
                            &mut mip,
                            0..=nutexb.footer.mipmap_count.saturating_sub(1),
                        ))
                        .changed()
                    {
                        settings.mipmap = mip as f32;
                    }
                }

                if nutexb.footer.layer_count == 6 {
                    let layers = ["X+", "X-", "Y+", "Y-", "Z+", "Z-"];
                    ui.label("Layer");
                    ComboBox::from_id_salt("nutexb_layer")
                        .selected_text(
                            layers
                                .get(settings.layer as usize)
                                .copied()
                                .unwrap_or_default(),
                        )
                        .show_ui(ui, |ui| {
                            for (i, layer) in layers.into_iter().enumerate() {
                                ui.selectable_value(&mut settings.layer, i as u32, layer);
                            }
                        });
                } else if nutexb.footer.layer_count > 1 {
                    // This case won't be used for in game nutexb files.
                    ui.label("Layer");
                    ui.add(
                        DragValue::new(&mut settings.layer)
                            .range(0..=nutexb.footer.layer_count - 1),
                    );
                } else if nutexb.footer.depth > 1 {
                    // TODO: Should this be "Slice" instead?
                    ui.label("Depth");
                    ui.add(DragValue::new(&mut settings.layer).range(0..=nutexb.footer.depth - 1));
                }
            });

            // TODO: Show a pixel grid in screen space?
            // TODO: Composite with a background color for alpha?
            // TODO: show controls for pan+zoom?
            // TODO: This shouldn't stretch on the bottom and left sides.
            // TODO: Add option to reset the view.
            egui::Frame::group(ui.style())
                .inner_margin(0.0)
                .show(ui, |ui| {
                    Scene::new()
                        .zoom_range(0.0..=f32::INFINITY)
                        .show(ui, &mut state.rect, |ui| {
                            // Preserve the aspect ratio of the texture.
                            let image_rect = if nutexb.footer.width > nutexb.footer.height {
                                egui::Rect {
                                    min: egui::Pos2::ZERO,
                                    max: egui::Pos2::new(
                                        512.0,
                                        512.0
                                            * (nutexb.footer.height as f32
                                                / nutexb.footer.width as f32),
                                    ),
                                }
                            } else {
                                egui::Rect {
                                    min: egui::Pos2::ZERO,
                                    max: egui::Pos2::new(
                                        512.0
                                            * (nutexb.footer.width as f32
                                                / nutexb.footer.height as f32),
                                        512.0,
                                    ),
                                }
                            };
                            let cb = Callback::new_paint_callback(image_rect, PaintTextureCallback);
                            ui.painter().add(cb);
                        });
                });
        });

    open
}

struct PaintTextureCallback;

impl CallbackTrait for PaintTextureCallback {
    // TODO: Handle the size of the texture?
    fn paint(
        &self,
        _info: egui::PaintCallbackInfo,
        render_pass: &mut wgpu::RenderPass<'static>,
        callback_resources: &egui_wgpu::CallbackResources,
    ) {
        let render_state: &RenderState = callback_resources.get().unwrap();
        render_state.texture_renderer.render(render_pass);
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
