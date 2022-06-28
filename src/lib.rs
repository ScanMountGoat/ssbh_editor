use std::{collections::BTreeMap, error::Error, path::Path};

use egui::{
    style::{WidgetVisuals, Widgets},
    Color32, FontFamily, FontId, Rounding, Stroke, TextStyle,
};
use nutexb_wgpu::TextureRenderer;

use ssbh_data::prelude::*;
use ssbh_wgpu::{ModelFolder, PipelineData, RenderSettings, ShaderDatabase};

pub mod app;
mod editors;
pub mod material;
mod render_settings;
pub mod widgets;

pub static FONT_BYTES: &[u8] = include_bytes!("fonts/NotoSansSC-Regular.otf");

// TODO: Store the current nutexb to paint?
pub struct TexturePainter {
    pub renderer: TextureRenderer,
    pub bind_group: Option<nutexb_wgpu::BindGroup0>,
}

impl TexturePainter {
    pub fn paint<'rpass>(&'rpass self, render_pass: &mut wgpu::RenderPass<'rpass>) {
        if let Some(bind_group) = self.bind_group.as_ref() {
            self.renderer.render(render_pass, bind_group);
        }
    }
}

pub struct RenderState {
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub default_textures: Vec<(String, wgpu::Texture)>,
    pub stage_cube: (wgpu::TextureView, wgpu::Sampler),
    pub pipeline_data: PipelineData,
    pub render_settings: RenderSettings,
    pub texture_render_settings: nutexb_wgpu::RenderSettings,
    pub shader_database: ShaderDatabase,
}

impl RenderState {
    pub fn new(
        device: wgpu::Device,
        queue: wgpu::Queue,
        surface_format: wgpu::TextureFormat,
        default_textures: Vec<(String, wgpu::Texture)>,
    ) -> Self {
        // TODO: How to organize the resources needed for viewport rendering?
        let stage_cube = ssbh_wgpu::load_default_cube(&device, &queue);

        // TODO: Should some of this state be moved to SsbhRenderer?
        // This would eliminate redundant shader loads.
        let pipeline_data = PipelineData::new(&device, surface_format);

        let shader_database = ssbh_wgpu::create_database();

        Self {
            device,
            queue,
            default_textures,
            stage_cube,
            pipeline_data,
            render_settings: RenderSettings::default(),
            texture_render_settings: nutexb_wgpu::RenderSettings::default(),
            shader_database,
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
    device: &wgpu::Device,
    queue: &wgpu::Queue,
) -> Vec<Vec<(String, egui::TextureId)>> {
    models
        .iter()
        .map(|m| {
            m.nutexbs
                .iter()
                .filter_map(|(f, n)| Some((f, n.as_ref().ok()?)))
                .filter(|(_, nutexb)| nutexb.footer.layer_count == 1) // TODO: How to handle 3d/array layers?
                .map(|(file_name, nutexb)| {
                    let texture = nutexb_wgpu::create_texture(&nutexb, device, queue);

                    // Assume the textures have the appropriate usage to work with egui.
                    // TODO: How to handle cube maps?
                    let egui_texture = egui_rpass.register_native_texture(
                        device,
                        &texture.create_view(&wgpu::TextureViewDescriptor::default()),
                        wgpu::FilterMode::Nearest,
                    );

                    (file_name.clone(), egui_texture)
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
    renderer: &TextureRenderer,
    default_textures: &[(String, wgpu::Texture)],
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    egui_rpass: &mut egui_wgpu::renderer::RenderPass,
) -> Vec<(String, egui::TextureId)> {
    let settings = &nutexb_wgpu::RenderSettings::default();
    let mut thumbnails: Vec<_> = default_textures
        .iter()
        .map(|(name, texture)| {
            let rgba_texture =
                renderer.render_to_texture_rgba(device, queue, texture, 64, 64, &settings);
            let rgba_view = rgba_texture.create_view(&wgpu::TextureViewDescriptor::default());
            // TODO: Does the filter mode here matter?
            let egui_texture =
                egui_rpass.register_native_texture(device, &rgba_view, wgpu::FilterMode::Linear);

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
