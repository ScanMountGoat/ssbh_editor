use std::{collections::BTreeMap, path::Path};

use egui::{FontFamily, FontId, TextStyle};
use egui_wgpu_backend::RenderPass;
use epi::*;
use nutexb_wgpu::TextureRenderer;
use ssbh_data::matl_data::ParamId;
use ssbh_wgpu::ModelFolder;

pub mod app;
mod material;
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
    egui_rpass: &mut RenderPass,
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
                    let egui_texture = egui_rpass.egui_texture_from_wgpu_texture(
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

pub fn generate_default_thumbnails(
    renderer: &TextureRenderer,
    default_textures: &[(String, wgpu::Texture)],
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    egui_rpass: &mut RenderPass,
) -> Vec<(String, egui::TextureId)> {
    default_textures
        .iter()
        .map(|(name, texture)| {
            let rgba_texture = renderer.render_to_texture_rgba(device, queue, &texture, 64, 64);
            let rgba_view = rgba_texture.create_view(&wgpu::TextureViewDescriptor::default());
            // TODO: Does the filter mode here matter?
            let egui_texture = egui_rpass.egui_texture_from_wgpu_texture(
                device,
                &rgba_view,
                wgpu::FilterMode::Linear,
            );

            (name.clone(), egui_texture)
        })
        .collect()
}

pub fn default_text_styles() -> BTreeMap<TextStyle, FontId> {
    // Modified from the default theme to be slightly larger.
    // Size 16.0 is common on the web and more legible than 14.0.
    let mut text_styles = BTreeMap::new();
    text_styles.insert(
        TextStyle::Small,
        FontId::new(12.0, FontFamily::Proportional),
    );
    text_styles.insert(TextStyle::Body, FontId::new(16.0, FontFamily::Proportional));
    text_styles.insert(
        TextStyle::Button,
        FontId::new(16.0, FontFamily::Proportional),
    );
    text_styles.insert(
        TextStyle::Heading,
        FontId::new(20.0, FontFamily::Proportional),
    );
    text_styles.insert(
        TextStyle::Monospace,
        FontId::new(16.0, FontFamily::Monospace),
    );
    text_styles
}
