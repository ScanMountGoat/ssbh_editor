use nutexb::NutexbFile;
use ssbh_data::matl_data::ParamId;

use crate::RenderState;

// TODO: Create a dedicated struct for this?
pub type Thumbnail = (String, egui::TextureId, TextureDimension);

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum TextureDimension {
    Texture1d,
    Texture2d,
    Texture3d,
    TextureCube,
}

impl TextureDimension {
    pub fn from_nutexb(nutexb: &NutexbFile) -> TextureDimension {
        // Assume no array layers for depth and cube maps.
        if nutexb.footer.depth > 1 {
            TextureDimension::Texture3d
        } else if nutexb.footer.layer_count == 6 {
            TextureDimension::TextureCube
        } else {
            TextureDimension::Texture2d
        }
    }

    pub fn from_param(param: ParamId) -> TextureDimension {
        match param {
            ParamId::Texture2 | ParamId::Texture7 | ParamId::Texture8 => {
                TextureDimension::TextureCube
            }
            _ => TextureDimension::Texture2d,
        }
    }
}

impl From<&wgpu::TextureViewDimension> for TextureDimension {
    fn from(d: &wgpu::TextureViewDimension) -> Self {
        // TODO: Worry about array textures?
        match d {
            wgpu::TextureViewDimension::D1 => Self::Texture1d,
            wgpu::TextureViewDimension::D2 => Self::Texture2d,
            wgpu::TextureViewDimension::D2Array => Self::Texture2d,
            wgpu::TextureViewDimension::Cube => Self::TextureCube,
            wgpu::TextureViewDimension::CubeArray => Self::TextureCube,
            wgpu::TextureViewDimension::D3 => Self::Texture3d,
        }
    }
}

pub fn generate_model_thumbnails(
    egui_renderer: &egui_wgpu::Renderer,
    model: &ssbh_wgpu::ModelFolder,
    render_model: &ssbh_wgpu::RenderModel,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
) -> Vec<(String, wgpu::TextureView, TextureDimension)> {
    model
        .nutexbs
        .iter()
        .filter_map(|(f, n)| Some((f, n.as_ref()?)))
        .filter_map(|(file_name, n)| {
            // TODO: Will this correctly handle missing thumbnails?
            let (texture, dimension) = render_model.get_texture(file_name)?;

            let view = create_thumbnail_texture_view(
                egui_renderer,
                device,
                queue,
                texture,
                dimension,
                n.footer.width,
                n.footer.height,
                n.footer.depth,
            );

            Some((file_name.clone(), view, dimension.into()))
        })
        .collect()
}

pub fn generate_default_thumbnails(
    egui_renderer: &mut egui_wgpu::Renderer,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
) -> Vec<Thumbnail> {
    // Split into two steps to avoid mutably and immutably borrowing egui renderer.
    let render_state: &RenderState = egui_renderer.callback_resources.get().unwrap();
    let thumbnails: Vec<_> = render_state
        .shared_data
        .default_textures()
        .iter()
        .map(|(name, texture, dimension)| {
            // Assume the textures have the appropriate usage to work with egui.
            // TODO: Are there other default cube textures?
            let view = if name == "#replace_cubemap" {
                create_thumbnail_texture_view(
                    egui_renderer,
                    device,
                    queue,
                    texture,
                    &wgpu::TextureViewDimension::Cube,
                    64,
                    64,
                    1,
                )
            } else {
                texture.create_view(&wgpu::TextureViewDescriptor::default())
            };

            (name.clone(), view, dimension.into())
        })
        .collect();

    thumbnails
        .into_iter()
        .map(|(name, view, dimension)| {
            let id =
                egui_renderer.register_native_texture(device, &view, wgpu::FilterMode::Nearest);
            (name, id, dimension)
        })
        .collect()
}

fn create_thumbnail_texture_view(
    egui_renderer: &egui_wgpu::Renderer,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    texture: &wgpu::Texture,
    dimension: &wgpu::TextureViewDimension,
    width: u32,
    height: u32,
    depth: u32,
) -> wgpu::TextureView {
    // Assume the textures have the appropriate usage to work with egui.
    match dimension {
        // Don't render color textures with any gamma correction applied.
        wgpu::TextureViewDimension::D2 => texture.create_view(&wgpu::TextureViewDescriptor {
            format: Some(texture.format().remove_srgb_suffix()),
            ..Default::default()
        }),
        _ => {
            // egui is expecting a 2D RGBA texture.
            let render_state: &RenderState = egui_renderer.callback_resources.get().unwrap();

            // Convert cube maps and 3d textures to 2D.
            let new_texture = render_state.texture_renderer.render_to_texture_2d_rgba(
                device,
                queue,
                texture,
                *dimension,
                (width, height, depth),
                64,
                64,
                &nutexb_wgpu::RenderSettings::default(),
            );

            // We forced 2D above, so we don't need to set the descriptor dimensions.
            new_texture.create_view(&wgpu::TextureViewDescriptor::default())
        }
    }
}
