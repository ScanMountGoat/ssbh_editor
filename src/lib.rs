use std::{collections::BTreeMap, path::Path};

use egui::{
    style::{WidgetVisuals, Widgets},
    Color32, FontFamily, FontId, Rounding, Stroke, TextStyle,
};
use nutexb_wgpu::TextureRenderer;

use ssbh_wgpu::ModelFolder;

pub mod app;
mod editors;
pub mod material;
pub mod widgets;

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

// TODO: Include default textures.
pub fn generate_model_thumbnails(
    renderer: &TextureRenderer,
    models: &[ssbh_wgpu::ModelFolder],
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    egui_rpass: &mut egui_wgpu::renderer::RenderPass,
) -> Vec<Vec<(String, egui::TextureId)>> {
    models
        .iter()
        .map(|m| {
            m.nutexbs
                .iter()
                .filter(|(_, nutexb)| nutexb.footer.layer_count == 1) // TODO: How to handle 3d/array layers?
                .map(|(file_name, nutexb)| {
                    let texture = nutexb_wgpu::create_texture(nutexb, device, queue);
                    let rgba_texture =
                        renderer.render_to_texture_rgba(device, queue, &texture, 64, 64);
                    let rgba_view =
                        rgba_texture.create_view(&wgpu::TextureViewDescriptor::default());
                    // TODO: Does the filter mode here matter?
                    let egui_texture = egui_rpass.register_native_texture(
                        device,
                        &rgba_view,
                        wgpu::FilterMode::Linear,
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
    let mut thumbnails: Vec<_> = default_textures
        .iter()
        .map(|(name, texture)| {
            let rgba_texture = renderer.render_to_texture_rgba(device, queue, texture, 64, 64);
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

pub fn default_text_styles() -> BTreeMap<TextStyle, FontId> {
    // Modified from the default theme.
    let mut text_styles = BTreeMap::new();
    text_styles.insert(
        TextStyle::Small,
        FontId::new(12.0, FontFamily::Proportional),
    );
    text_styles.insert(TextStyle::Body, FontId::new(14.0, FontFamily::Proportional));
    text_styles.insert(
        TextStyle::Button,
        FontId::new(14.0, FontFamily::Proportional),
    );
    text_styles.insert(
        TextStyle::Heading,
        FontId::new(20.0, FontFamily::Proportional),
    );
    text_styles.insert(
        TextStyle::Monospace,
        FontId::new(14.0, FontFamily::Monospace),
    );
    text_styles
}

pub fn widgets_dark() -> Widgets {
    // Modified from the default theme to have higher text contrast.
    Widgets {
        noninteractive: WidgetVisuals {
            bg_fill: Color32::from_gray(27), // window background
            bg_stroke: Stroke::new(1.0, Color32::from_gray(60)), // separators, indentation lines, windows outlines
            fg_stroke: Stroke::new(1.0, Color32::from_gray(180)), // normal text color
            rounding: Rounding::same(2.0),
            expansion: 0.0,
        },
        inactive: WidgetVisuals {
            bg_fill: Color32::from_gray(60), // button background
            bg_stroke: Default::default(),
            fg_stroke: Stroke::new(1.0, Color32::from_gray(204)), // button text
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
            bg_stroke: Stroke::new(1.0, Color32::WHITE),
            fg_stroke: Stroke::new(2.0, Color32::WHITE),
            rounding: Rounding::same(2.0),
            expansion: 1.0,
        },
        open: WidgetVisuals {
            bg_fill: Color32::from_gray(27),
            bg_stroke: Stroke::new(1.0, Color32::from_gray(60)),
            fg_stroke: Stroke::new(1.0, Color32::from_gray(204)),
            rounding: Rounding::same(2.0),
            expansion: 0.0,
        },
    }
}