// Disable the console on Windows in release mode.
#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use std::sync::Arc;

use egui::ViewportBuilder;
use egui_commonmark::CommonMarkCache;
use egui_wgpu::WgpuConfiguration;
use log::error;
use nutexb_wgpu::TextureRenderer;
use ssbh_editor::app::{ItemsToUpdate, SsbhApp, UiState};
use ssbh_editor::material::load_material_presets;
use ssbh_editor::preferences::{AppPreferences, GraphicsBackend};
use ssbh_editor::presets::default_presets;
use ssbh_editor::update::{check_for_updates, LatestReleaseInfo};
use ssbh_editor::{
    checkerboard_texture, default_fonts, default_text_styles, generate_default_thumbnails,
    path::{presets_file, PROJECT_DIR},
    widgets_dark, AnimationState, CameraState, RenderState,
};
use ssbh_editor::{update_color_theme, SwingState, Thumbnail};
use ssbh_wgpu::{BoneNameRenderer, SsbhRenderer};

fn main() {
    let mut args = pico_args::Arguments::from_env();

    // Initialize logging first in case app startup has warnings.
    // TODO: Also log to a file?
    log::set_logger(&*ssbh_editor::app::LOGGER)
        .map(|()| log::set_max_level(log::LevelFilter::Info))
        .unwrap();

    let icon =
        eframe::icon_data::from_png_bytes(include_bytes!("icons/ssbh_editor32.png")).unwrap();

    let mut preferences = AppPreferences::load_from_file();

    // Some Windows systems don't properly support Vulkan.
    // This mostly affects dual GPU systems like laptops.
    // Add an option to force a backend so the application can open.
    if let Some(backend_arg) = args.opt_value_from_str::<_, String>("--backend").unwrap() {
        match backend_arg.to_lowercase().as_str() {
            "vulkan" => preferences.graphics_backend = GraphicsBackend::Vulkan,
            "metal" => preferences.graphics_backend = GraphicsBackend::Metal,
            "dx12" => preferences.graphics_backend = GraphicsBackend::Dx12,
            _ => (),
        }
    }

    create_app_data_directory();

    let release_info = check_for_updates();

    let presets_file = presets_file();
    let material_presets = load_material_presets(presets_file);

    let preferred_backends = match preferences.graphics_backend {
        GraphicsBackend::Auto => wgpu::Backends::PRIMARY,
        GraphicsBackend::Vulkan => wgpu::Backends::VULKAN,
        GraphicsBackend::Metal => wgpu::Backends::METAL,
        GraphicsBackend::Dx12 => wgpu::Backends::DX12,
    };

    eframe::run_native(
        concat!("SSBH Editor ", env!("CARGO_PKG_VERSION")),
        eframe::NativeOptions {
            renderer: eframe::Renderer::Wgpu,
            viewport: ViewportBuilder::default()
                .with_icon(icon)
                .with_inner_size([1280.0, 720.0]),
            wgpu_options: WgpuConfiguration {
                supported_backends: preferred_backends,
                device_descriptor: Arc::new(|_adapter| wgpu::DeviceDescriptor {
                    required_features: wgpu::Features::default() | ssbh_wgpu::REQUIRED_FEATURES,
                    required_limits: wgpu::Limits::default(),
                    label: None,
                }),
                present_mode: wgpu::PresentMode::Fifo,
                power_preference: wgpu::PowerPreference::HighPerformance,
                ..Default::default()
            },
            ..Default::default()
        },
        Box::new(|cc| {
            let ctx = &cc.egui_ctx;

            ctx.set_style(egui::style::Style {
                text_styles: default_text_styles(),
                visuals: egui::style::Visuals {
                    widgets: widgets_dark(),
                    ..Default::default()
                },
                ..Default::default()
            });
            ctx.set_fonts(default_fonts());

            egui_extras::install_image_loaders(ctx);

            // Make sure the theme updates if changed from preferences.
            update_color_theme(&preferences, ctx);

            let wgpu_state = cc.wgpu_render_state.as_ref().unwrap();
            let mut egui_renderer = wgpu_state.renderer.write();

            // TODO: Use this to generate thumbnails for cube maps and 3d textures.
            // Make sure the texture preview is ready to be accessed by the app.
            // State is stored in a type map because of lifetime requirements.
            // https://github.com/emilk/egui/blob/master/egui_demo_app/src/apps/custom3d_wgpu.rs
            let texture_renderer = TextureRenderer::new(
                &wgpu_state.device,
                &wgpu_state.queue,
                wgpu_state.target_format,
            );

            let red_checkerboard = checkerboard_texture(
                &wgpu_state.device,
                &wgpu_state.queue,
                &mut egui_renderer,
                [255, 0, 0, 255],
            );
            let yellow_checkerboard = checkerboard_texture(
                &wgpu_state.device,
                &wgpu_state.queue,
                &mut egui_renderer,
                [255, 255, 0, 255],
            );

            // TODO: What to use for the initial size?
            let scale_factor = ctx.native_pixels_per_point().unwrap_or(1.0).into();
            let renderer = SsbhRenderer::new(
                &wgpu_state.device,
                &wgpu_state.queue,
                512,
                512,
                scale_factor,
                [0.0, 0.0, 0.0, 1.0],
                wgpu_state.target_format,
            );

            let bone_name_renderer = BoneNameRenderer::new(
                &wgpu_state.device,
                &wgpu_state.queue,
                Some(ssbh_editor::FONT_BYTES.to_vec()),
                wgpu_state.target_format,
            );

            // TODO: Camera framing?
            let camera_state = CameraState {
                values: preferences.default_camera.clone(),
                ..Default::default()
            };

            let render_state = RenderState::new(
                &wgpu_state.device,
                &wgpu_state.queue,
                wgpu_state.adapter.get_info(),
                renderer,
                texture_renderer,
                bone_name_renderer,
            );

            egui_renderer.callback_resources.insert(render_state);

            let default_thumbnails = generate_default_thumbnails(
                &mut egui_renderer,
                &wgpu_state.device,
                &wgpu_state.queue,
            );

            let app = create_app(
                default_thumbnails,
                release_info,
                material_presets,
                red_checkerboard,
                yellow_checkerboard,
                camera_state,
                preferences,
            );

            Box::new(app)
        }),
    )
    .unwrap();

    // TODO: How to save state to disk?
}

// TODO: Make this a method.
fn create_app(
    default_thumbnails: Vec<Thumbnail>,
    release_info: LatestReleaseInfo,
    material_presets: Vec<ssbh_data::matl_data::MatlEntryData>,
    red_checkerboard: egui::TextureId,
    yellow_checkerboard: egui::TextureId,
    camera_state: CameraState,
    preferences: AppPreferences,
) -> SsbhApp {
    SsbhApp {
        models: Vec::new(),
        default_thumbnails,
        models_to_update: ItemsToUpdate::None,
        release_info,
        should_update_lighting: false,
        should_refresh_render_settings: false,
        should_update_camera: true,
        should_validate_models: false,
        should_update_clear_color: true,
        should_update_thumbnails: false,
        material_presets,
        default_presets: default_presets(),
        red_checkerboard,
        yellow_checkerboard,
        draw_bone_names: false,
        ui_state: UiState::default(),
        animation_state: AnimationState::default(),
        swing_state: SwingState::default(),
        show_left_panel: true,
        show_right_panel: true,
        show_bottom_panel: true,
        camera_state,
        preferences,
        enable_helper_bones: true,
        screenshot_to_render: None,
        animation_gif_to_render: None,
        animation_image_sequence_to_render: None,
        markdown_cache: CommonMarkCache::default(),
        previous_viewport_width: 512.0,
        previous_viewport_height: 512.0,
        has_initialized_zoom_factor: false,
    }
}

fn create_app_data_directory() {
    let app_data_dir = PROJECT_DIR.data_local_dir();
    if let Err(e) = std::fs::create_dir_all(app_data_dir) {
        error!("Failed to create application directory at {app_data_dir:?}: {e}")
    }
}
