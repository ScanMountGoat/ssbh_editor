use std::{collections::BTreeMap, error::Error, path::Path};

use egui::{
    style::{WidgetVisuals, Widgets},
    Color32, FontFamily, FontId, Rounding, Stroke, TextStyle,
};
use nutexb_wgpu::TextureRenderer;

use ssbh_data::prelude::*;
use ssbh_wgpu::{ModelFolder, RenderSettings, SharedRenderData};
use winit::dpi::PhysicalPosition;

pub mod app;
mod editors;
pub mod material;
mod render_settings;
pub mod validation;
pub mod widgets;

pub static FONT_BYTES: &[u8] = include_bytes!("fonts/NotoSansSC-Regular.otf");

// TODO: Store the current nutexb to paint?
pub struct TexturePainter<'a> {
    pub renderer: TextureRenderer,
    pub texture: Option<(&'a wgpu::Texture, &'a wgpu::TextureViewDimension)>,
}

impl<'a> TexturePainter<'a> {
    pub fn paint<'rpass>(&'rpass self, render_pass: &mut wgpu::RenderPass<'rpass>) {
        self.renderer.render(render_pass);
    }
}

pub struct CameraInputState {
    pub previous_cursor_position: PhysicalPosition<f64>,
    pub is_mouse_left_clicked: bool,
    pub is_mouse_right_clicked: bool,
    pub translation_xyz: glam::Vec3,
    pub rotation_xyz: glam::Vec3,
}

impl Default for CameraInputState {
    fn default() -> Self {
        Self {
            previous_cursor_position: PhysicalPosition { x: 0.0, y: 0.0 },
            is_mouse_left_clicked: false,
            is_mouse_right_clicked: false,
            translation_xyz: glam::Vec3::new(0.0, -8.0, -60.0),
            rotation_xyz: glam::Vec3::new(0.0, 0.0, 0.0)
        }
    }
}

pub struct RenderState {
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub render_settings: RenderSettings,
    pub texture_render_settings: nutexb_wgpu::RenderSettings,
    pub shared_data: SharedRenderData,
}

impl RenderState {
    pub fn new(
        device: wgpu::Device,
        queue: wgpu::Queue,
        surface_format: wgpu::TextureFormat,
    ) -> Self {
        let shared_data = SharedRenderData::new(&device, &queue, surface_format);
        Self {
            device,
            queue,
            render_settings: RenderSettings::default(),
            texture_render_settings: nutexb_wgpu::RenderSettings::default(),
            shared_data,
        }
    }
}

pub struct AnimationState {
    pub current_frame: f32,
    pub is_playing: bool,
    pub animation_frame_was_changed: bool,
    pub selected_slot: usize,
    pub animations: Vec<Option<AnimationIndex>>,
    pub previous_frame_start: std::time::Instant,
}

impl AnimationState {
    pub fn new() -> Self {
        Self {
            animations: Vec::new(),
            is_playing: false,
            current_frame: 0.0,
            previous_frame_start: std::time::Instant::now(),
            animation_frame_was_changed: false,
            selected_slot: 0,
        }
    }
}

pub struct AnimationIndex {
    pub folder_index: usize,
    pub anim_index: usize,
}

impl AnimationIndex {
    pub fn get_animation<'a>(
        index: Option<&AnimationIndex>,
        models: &'a [ModelFolder],
    ) -> Option<&'a (String, Result<AnimData, Box<dyn Error>>)> {
        index.and_then(|index| {
            models
                .get(index.folder_index)
                .and_then(|m| m.anims.get(index.anim_index))
        })
    }
}

pub fn load_models_recursive<P: AsRef<Path>>(root: P) -> Vec<ModelFolder> {
    let mut models = ssbh_wgpu::load_model_folders(root);
    models.sort_by_key(|m| m.folder_name.to_string());
    for model in &mut models {
        sort_files(model);
    }
    models
}

pub fn load_model<P: AsRef<Path>>(root: P) -> ModelFolder {
    let mut model = ssbh_wgpu::ModelFolder::load_folder(root);
    sort_files(&mut model);
    model
}

fn sort_files(model: &mut ModelFolder) {
    // Sort by file name for consistent ordering in the UI.
    model.adjs.sort_by(|(n1, _), (n2, _)| n1.cmp(n2));
    model.anims.sort_by(|(n1, _), (n2, _)| n1.cmp(n2));
    model.matls.sort_by(|(n1, _), (n2, _)| n1.cmp(n2));
    model.meshes.sort_by(|(n1, _), (n2, _)| n1.cmp(n2));
    model.modls.sort_by(|(n1, _), (n2, _)| n1.cmp(n2));
    model.nutexbs.sort_by(|(n1, _), (n2, _)| n1.cmp(n2));
    model.skels.sort_by(|(n1, _), (n2, _)| n1.cmp(n2));
}

pub fn generate_model_thumbnails(
    egui_rpass: &mut egui_wgpu::renderer::RenderPass,
    models: &[ssbh_wgpu::ModelFolder],
    render_models: &[ssbh_wgpu::RenderModel],
    device: &wgpu::Device,
    queue: &wgpu::Queue,
) -> Vec<Vec<(String, egui::TextureId)>> {
    models
        .iter()
        .zip(render_models)
        .map(|(model, render_model)| {
            model
                .nutexbs
                .iter()
                .filter_map(|(f, n)| Some((f, n.as_ref().ok()?)))
                .filter_map(|(file_name, n)| {
                    // TODO: Will this correctly handle missing thumbnails?
                    let (texture, dimension) = render_model.get_texture(file_name)?;

                    // Assume the textures have the appropriate usage to work with egui.
                    // TODO: How to handle cube maps?
                    // egui is expecting a 2D RGBA texture.
                    let egui_texture = match dimension {
                        wgpu::TextureViewDimension::D2 => egui_rpass.register_native_texture(
                            device,
                            &texture.create_view(&wgpu::TextureViewDescriptor::default()),
                            wgpu::FilterMode::Nearest,
                        ),
                        _ => {
                            let painter: &TexturePainter =
                                egui_rpass.paint_callback_resources.get().unwrap();

                            // Convert cube maps and 3d textures to 2D.
                            let new_texture = painter.renderer.render_to_texture_2d_rgba(
                                device,
                                queue,
                                texture,
                                *dimension,
                                (n.footer.width, n.footer.height, n.footer.depth),
                                64,
                                64,
                                &nutexb_wgpu::RenderSettings::default(),
                            );

                            // We forced 2D above, so we don't need to set the descriptor dimensions.
                            egui_rpass.register_native_texture(
                                device,
                                &new_texture.create_view(&wgpu::TextureViewDescriptor::default()),
                                wgpu::FilterMode::Nearest,
                            )
                        }
                    };

                    Some((file_name.clone(), egui_texture))
                })
                .collect()
        })
        .collect()
}

pub fn checkerboard_texture(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    egui_rpass: &mut egui_wgpu::renderer::RenderPass,
    color: [u8; 4],
) -> egui::TextureId {
    let texture_size = wgpu::Extent3d {
        width: 2,
        height: 2,
        depth_or_array_layers: 1,
    };
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: None,
        size: texture_size,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::COPY_SRC
            | wgpu::TextureUsages::COPY_DST
            | wgpu::TextureUsages::TEXTURE_BINDING,
    });
    queue.write_texture(
        wgpu::ImageCopyTexture {
            texture: &texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        &[
            0, 0, 0, 255, color[0], color[1], color[2], color[3], color[0], color[1], color[2],
            color[3], 0, 0, 0, 255,
        ],
        wgpu::ImageDataLayout {
            offset: 0,
            bytes_per_row: std::num::NonZeroU32::new(8),
            rows_per_image: None,
        },
        texture_size,
    );

    egui_rpass.register_native_texture(
        device,
        &texture.create_view(&wgpu::TextureViewDescriptor::default()),
        wgpu::FilterMode::Nearest,
    )
}

pub fn generate_default_thumbnails(
    egui_rpass: &mut egui_wgpu::renderer::RenderPass,
    default_textures: &[(String, wgpu::Texture, wgpu::TextureViewDimension)],
    device: &wgpu::Device,
) -> Vec<(String, egui::TextureId)> {
    let mut thumbnails: Vec<_> = default_textures
        .iter()
        .map(|(name, texture, _)| {
            // Assume the textures have the appropriate usage to work with egui.
            // TODO: How to handle cube maps?
            let egui_texture = egui_rpass.register_native_texture(
                device,
                &texture.create_view(&wgpu::TextureViewDescriptor::default()),
                wgpu::FilterMode::Nearest,
            );

            (name.clone(), egui_texture)
        })
        .collect();
    // TODO: Add proper cube map thumbnails to nutexb_wgpu.
    thumbnails.push((
        "#replace_cubemap".to_string(),
        thumbnails
            .iter()
            .find(|(n, _)| n == "/common/shader/sfxpbs/default_black")
            .unwrap()
            .1,
    ));
    thumbnails
}

pub fn default_fonts() -> egui::FontDefinitions {
    // The default fonts don't support Japanese or Chinese characters.
    // These languages are required to display some user mods correctly.
    let mut fonts = egui::FontDefinitions::empty();
    fonts
        .font_data
        .insert("font".to_owned(), egui::FontData::from_static(FONT_BYTES));
    fonts.font_data.insert(
        "emoji".to_owned(),
        egui::FontData::from_static(include_bytes!("fonts/emoji.ttf")),
    );

    // Use the same font for all text for a consistent look for numeric digits.
    let monospace = fonts
        .families
        .get_mut(&egui::FontFamily::Monospace)
        .unwrap();
    monospace.insert(0, "font".to_owned());
    monospace.insert(1, "emoji".to_owned());

    let proportional = fonts
        .families
        .get_mut(&egui::FontFamily::Proportional)
        .unwrap();
    proportional.insert(0, "font".to_owned());
    proportional.insert(1, "emoji".to_owned());

    fonts
}

pub fn default_text_styles() -> BTreeMap<TextStyle, FontId> {
    // Modified from the default theme.
    let mut text_styles = BTreeMap::new();
    text_styles.insert(
        TextStyle::Small,
        FontId::new(12.0, FontFamily::Proportional),
    );
    text_styles.insert(TextStyle::Body, FontId::new(18.0, FontFamily::Proportional));
    text_styles.insert(
        TextStyle::Button,
        FontId::new(18.0, FontFamily::Proportional),
    );
    text_styles.insert(
        TextStyle::Heading,
        FontId::new(24.0, FontFamily::Proportional),
    );
    // Use a consistent font for sliders and drag values.
    text_styles.insert(
        TextStyle::Monospace,
        FontId::new(18.0, FontFamily::Proportional),
    );
    text_styles
}

pub fn widgets_dark() -> Widgets {
    // Modified from the default theme to have higher text contrast.
    Widgets {
        noninteractive: WidgetVisuals {
            bg_fill: Color32::from_gray(27), // window background
            bg_stroke: Stroke::new(1.0, Color32::from_gray(60)), // separators, indentation lines, windows outlines
            fg_stroke: Stroke::new(1.0, Color32::from_gray(200)), // normal text color
            rounding: Rounding::same(2.0),
            expansion: 0.0,
        },
        inactive: WidgetVisuals {
            bg_fill: Color32::from_gray(60), // button background
            bg_stroke: Default::default(),
            fg_stroke: Stroke::new(1.0, Color32::from_gray(200)), // button text
            rounding: Rounding::same(2.0),
            expansion: 0.0,
        },
        hovered: WidgetVisuals {
            bg_fill: Color32::from_gray(70),
            bg_stroke: Stroke::new(1.0, Color32::from_gray(150)), // e.g. hover over window edge or button
            fg_stroke: Stroke::new(1.5, Color32::from_gray(255)),
            rounding: Rounding::same(3.0),
            expansion: 1.0,
        },
        active: WidgetVisuals {
            bg_fill: Color32::from_gray(55),
            bg_stroke: Stroke::new(1.0, Color32::from_gray(255)),
            fg_stroke: Stroke::new(2.0, Color32::from_gray(255)),
            rounding: Rounding::same(2.0),
            expansion: 1.0,
        },
        open: WidgetVisuals {
            bg_fill: Color32::from_gray(27),
            bg_stroke: Stroke::new(1.0, Color32::from_gray(60)),
            fg_stroke: Stroke::new(1.0, Color32::from_gray(200)),
            rounding: Rounding::same(2.0),
            expansion: 0.0,
        },
    }
}

fn horizontal_separator_empty(ui: &mut egui::Ui) {
    let available_space = ui.available_size_before_wrap();
    ui.allocate_space(egui::vec2(available_space.x, 6.0));
}
