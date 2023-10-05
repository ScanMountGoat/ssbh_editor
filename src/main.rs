// Disable the console on Windows in release mode.
#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use egui::ecolor::linear_f32_from_gamma_u8;
use egui::Visuals;
use egui_commonmark::CommonMarkCache;
use log::error;
use nutexb_wgpu::TextureRenderer;
use pollster::FutureExt;
use ssbh_data::prelude::*;
use ssbh_editor::app::{Icons, ItemsToUpdate, SsbhApp, UiState};
use ssbh_editor::capture::{render_animation_sequence, render_screenshot};
use ssbh_editor::material::load_material_presets;
use ssbh_editor::preferences::{AppPreferences, GraphicsBackend};
use ssbh_editor::presets::default_presets;
use ssbh_editor::update::{check_for_updates, LatestReleaseInfo};
use ssbh_editor::{
    animate_models, checkerboard_texture, default_fonts, default_text_styles,
    generate_default_thumbnails, generate_model_thumbnails,
    path::{presets_file, PROJECT_DIR},
    widgets_dark, widgets_light, AnimationState, CameraInputState, RenderState, TexturePainter,
};
use ssbh_editor::{LightingData, SwingState, Thumbnail};
use ssbh_wgpu::animation::camera::animate_camera;
use ssbh_wgpu::{next_frame, BoneNameRenderer, CameraTransforms, RenderModel, SsbhRenderer};
use winit::{dpi::PhysicalSize, event::*, event_loop::ControlFlow};

// TODO: Make these configurable?
const NEAR_CLIP: f32 = 1.0;
const FAR_CLIP: f32 = 400000.0;

// TODO: Split up this file into modules.

fn main() {
    let mut args = pico_args::Arguments::from_env();

    // Initialize logging first in case app startup has warnings.
    // TODO: Also log to a file?
    log::set_logger(&*ssbh_editor::app::LOGGER)
        .map(|()| log::set_max_level(log::LevelFilter::Info))
        .unwrap();

    let icon = image::load_from_memory_with_format(
        include_bytes!("icons/ssbh_editor32.png"),
        image::ImageFormat::Png,
    )
    .unwrap();

    let event_loop = winit::event_loop::EventLoopBuilder::with_user_event().build();
    let window = build_window(icon, &event_loop);

    let mut preferences = AppPreferences::load_from_file();

    // Some Windows systems don't properly support Vulkan.
    // Add an option to force a backend so the application can open.
    if let Some(backend_arg) = args.opt_value_from_str::<_, String>("--backend").unwrap() {
        match backend_arg.to_lowercase().as_str() {
            "vulkan" => preferences.graphics_backend = GraphicsBackend::Vulkan,
            "metal" => preferences.graphics_backend = GraphicsBackend::Metal,
            "dx12" => preferences.graphics_backend = GraphicsBackend::Dx12,
            _ => (),
        }
    }

    let (surface, device, queue, adapter) = initialize_wgpu(preferences.graphics_backend, &window);

    create_app_data_directory();

    let release_info = check_for_updates();

    let mut size = window.inner_size();

    // Use the ssbh_wgpu format to ensure compatibility.
    let surface_format = ssbh_wgpu::RGBA_COLOR_FORMAT;
    let mut surface_config = wgpu::SurfaceConfiguration {
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        format: surface_format,
        width: size.width,
        height: size.height,
        present_mode: wgpu::PresentMode::Fifo,
        alpha_mode: wgpu::CompositeAlphaMode::Auto,
        view_formats: Vec::new(),
    };
    surface.configure(&device, &surface_config);

    // Initialize egui and winit state.
    let ctx = egui::Context::default();
    let mut egui_renderer = egui_wgpu::renderer::Renderer::new(&device, surface_format, None, 1);
    let mut winit_state = egui_winit::State::new(&event_loop);
    winit_state.set_max_texture_side(device.limits().max_texture_dimension_2d as usize);

    let mut current_scale_factor = if preferences.use_custom_scale_factor {
        preferences.scale_factor
    } else {
        window.scale_factor()
    };
    winit_state.set_pixels_per_point(current_scale_factor as f32);

    ctx.set_style(egui::style::Style {
        text_styles: default_text_styles(),
        visuals: egui::style::Visuals {
            widgets: widgets_dark(),
            ..Default::default()
        },
        ..Default::default()
    });
    ctx.set_fonts(default_fonts());

    let mut renderer = SsbhRenderer::new(
        &device,
        &queue,
        size.width,
        size.height,
        current_scale_factor,
        [0.0; 3],
    );
    let mut bone_name_renderer =
        BoneNameRenderer::new(&device, &queue, Some(ssbh_editor::FONT_BYTES.to_vec()));

    // TODO: Use this to generate thumbnails for cube maps and 3d textures.
    let texture_renderer = TextureRenderer::new(&device, &queue, surface_format);
    // Make sure the texture preview is ready for accessed by the app.
    // State is stored in a type map because of lifetime requirements.
    // https://github.com/emilk/egui/blob/master/egui_demo_app/src/apps/custom3d_wgpu.rs
    egui_renderer
        .paint_callback_resources
        .insert(TexturePainter {
            renderer: texture_renderer,
            texture: None,
        });

    // TODO: Camera framing?
    let mut camera_state = CameraInputState::default();

    update_camera(
        &queue,
        &mut renderer,
        &mut camera_state,
        size,
        current_scale_factor,
    );

    let presets_file = presets_file();
    let material_presets = load_material_presets(presets_file);

    let red_checkerboard =
        checkerboard_texture(&device, &queue, &mut egui_renderer, [255, 0, 0, 255]);
    let yellow_checkerboard =
        checkerboard_texture(&device, &queue, &mut egui_renderer, [255, 255, 0, 255]);

    let render_state = RenderState::new(device, queue, adapter.get_info());
    let default_thumbnails = generate_default_thumbnails(
        &mut egui_renderer,
        render_state.shared_data.default_textures(),
        &render_state.device,
        &render_state.queue,
    );

    let mut app = create_app(
        default_thumbnails,
        release_info,
        material_presets,
        red_checkerboard,
        yellow_checkerboard,
        render_state,
        camera_state,
        preferences,
    );

    // Make sure the theme updates if changed from preferences.
    // TODO: This is redundant with the initialization above?
    update_color_theme(&app, &ctx);
    let mut previous_dark_mode = false;

    // TODO: Does the T in the the event type matter here?
    event_loop.run(
        move |event: winit::event::Event<'_, usize>, _, control_flow| {
            match event {
                winit::event::Event::RedrawRequested(..) => {
                    // Don't render if the application window is minimized.
                    if window.inner_size().width > 0 && window.inner_size().height > 0 {
                        update_and_render_app(
                            &mut app,
                            &mut winit_state,
                            &mut renderer,
                            &mut bone_name_renderer,
                            &mut egui_renderer,
                            &mut previous_dark_mode,
                            &window,
                            &ctx,
                            &surface,
                            size,
                            &surface_config,
                            current_scale_factor,
                        );
                    }
                }
                winit::event::Event::WindowEvent { event, .. } => {
                    if !winit_state.on_event(&ctx, &event).consumed {
                        match event {
                            winit::event::WindowEvent::Resized(_) => {
                                // The dimensions must both be non-zero before resizing.
                                if window.inner_size().width > 0 && window.inner_size().height > 0 {
                                    // Use the window size to avoid a potential error from size mismatches.
                                    size = window.inner_size();

                                    resize(
                                        &mut app,
                                        &mut renderer,
                                        &mut surface_config,
                                        &size,
                                        current_scale_factor,
                                        &surface,
                                    );
                                }
                            }
                            winit::event::WindowEvent::CloseRequested => {
                                app.write_state_to_disk();
                                *control_flow = ControlFlow::Exit;
                            }
                            winit::event::WindowEvent::ScaleFactorChanged {
                                scale_factor, ..
                            } => {
                                current_scale_factor = if app.preferences.use_custom_scale_factor {
                                    app.preferences.scale_factor
                                } else {
                                    scale_factor
                                };
                                winit_state.set_pixels_per_point(current_scale_factor as f32);
                            }
                            _ => {
                                if ctx.wants_keyboard_input() || ctx.wants_pointer_input() {
                                    // It's possible to interact with the UI with the mouse over the viewport.
                                    // Disable tracking the mouse in this case to prevent unwanted camera rotations.
                                    // This mostly affects resizing the left and right side panels.
                                    app.camera_state.is_mouse_left_clicked = false;
                                    app.camera_state.is_mouse_right_clicked = false;
                                } else {
                                    // Only update the viewport camera if the user isn't interacting with the UI.
                                    if handle_input(&mut app.camera_state, &event, size) {
                                        update_camera(
                                            &app.render_state.queue,
                                            &mut renderer,
                                            &mut app.camera_state,
                                            size,
                                            current_scale_factor,
                                        );
                                    }
                                }
                            }
                        }
                    }
                }
                winit::event::Event::MainEventsCleared => {
                    window.request_redraw();
                }
                // TODO: Why does this need to be here?
                winit::event::Event::UserEvent(_) => (),
                _ => (),
            }
        },
    );
}

fn initialize_wgpu(
    backend: GraphicsBackend,
    window: &winit::window::Window,
) -> (wgpu::Surface, wgpu::Device, wgpu::Queue, wgpu::Adapter) {
    let start = std::time::Instant::now();

    let preferred_backends = match backend {
        GraphicsBackend::Auto => wgpu::Backends::PRIMARY,
        GraphicsBackend::Vulkan => wgpu::Backends::VULKAN,
        GraphicsBackend::Metal => wgpu::Backends::METAL,
        GraphicsBackend::Dx12 => wgpu::Backends::DX12,
    };

    // Try the other backends in case the user sets the wrong preferred backend.
    // Try DX12 last to fix Vulkan not working on some Windows systems.
    // TODO: Get adapter info and display in GPU preferences?
    // TODO: Enumerate available adapters?
    let (surface, adapter) = request_adapter(window, preferred_backends)
        .or_else(|| request_adapter(window, wgpu::Backends::VULKAN | wgpu::Backends::METAL))
        .or_else(|| request_adapter(window, wgpu::Backends::DX12))
        .unwrap();

    println!("Request compatible adapter: {:?}", start.elapsed());
    println!("Adapter: {:?}", adapter.get_info());

    let (device, queue) = adapter
        .request_device(
            &wgpu::DeviceDescriptor {
                features: wgpu::Features::default() | ssbh_wgpu::REQUIRED_FEATURES,
                limits: wgpu::Limits::default(),
                label: None,
            },
            None,
        )
        .block_on()
        .unwrap();

    (surface, device, queue, adapter)
}

// TODO: Separate module for camera + input handling?
fn calculate_mvp(
    size: winit::dpi::PhysicalSize<u32>,
    translation_xyz: glam::Vec3,
    rotation_xyz_radians: glam::Vec3,
    fov_y_radians: f32,
) -> (glam::Vec4, glam::Mat4, glam::Mat4) {
    let aspect = size.width as f32 / size.height as f32;

    let rotation = glam::Mat4::from_euler(
        glam::EulerRot::XYZ,
        rotation_xyz_radians.x,
        rotation_xyz_radians.y,
        rotation_xyz_radians.z,
    );
    let model_view_matrix = glam::Mat4::from_translation(translation_xyz) * rotation;
    let perspective_matrix = glam::Mat4::perspective_rh(fov_y_radians, aspect, NEAR_CLIP, FAR_CLIP);

    let camera_pos = model_view_matrix.inverse().col(3);

    (
        camera_pos,
        model_view_matrix,
        perspective_matrix * model_view_matrix,
    )
}

// TODO: Make this a method.
fn create_app(
    default_thumbnails: Vec<Thumbnail>,
    release_info: LatestReleaseInfo,
    material_presets: Vec<ssbh_data::matl_data::MatlEntryData>,
    red_checkerboard: egui::TextureId,
    yellow_checkerboard: egui::TextureId,
    render_state: RenderState,
    camera_state: CameraInputState,
    preferences: AppPreferences,
) -> SsbhApp {
    SsbhApp {
        models: Vec::new(),
        render_models: Vec::new(),
        default_thumbnails,
        models_to_update: ItemsToUpdate::None,
        release_info,
        should_update_lighting: false,
        should_refresh_render_settings: false,
        should_update_camera: false,
        should_validate_models: false,
        should_update_clear_color: true,
        should_update_thumbnails: false,
        material_presets,
        default_presets: default_presets(),
        red_checkerboard,
        yellow_checkerboard,
        draw_bone_names: false,
        ui_state: UiState::default(),
        render_state,
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
        icons: Icons::new(),
        markdown_cache: CommonMarkCache::default(),
    }
}

fn build_window(
    icon: image::DynamicImage,
    event_loop: &winit::event_loop::EventLoop<usize>,
) -> winit::window::Window {
    winit::window::WindowBuilder::new()
        .with_decorations(true)
        .with_resizable(true)
        .with_transparent(false)
        .with_title(concat!("SSBH Editor ", env!("CARGO_PKG_VERSION")))
        .with_window_icon(Some(
            winit::window::Icon::from_rgba(icon.into_bytes(), 32, 32).unwrap(),
        ))
        .with_inner_size(winit::dpi::Size::Logical(winit::dpi::LogicalSize::new(
            // Set a small initial size so the window doesn't overflow the screen.
            1280.0, 720.0,
        )))
        .build(event_loop)
        .unwrap()
}

fn update_and_render_app(
    app: &mut SsbhApp,
    winit_state: &mut egui_winit::State,
    renderer: &mut SsbhRenderer,
    bone_name_renderer: &mut BoneNameRenderer,
    egui_renderer: &mut egui_wgpu::Renderer,
    previous_dark_mode: &mut bool,
    window: &winit::window::Window,
    ctx: &egui::Context,
    surface: &wgpu::Surface,
    size: PhysicalSize<u32>,
    surface_config: &wgpu::SurfaceConfiguration,
    scale_factor: f64,
) {
    // Always update the frame times even if no animation is playing.
    // This avoids skipping when resuming playback.
    let current_frame_start = std::time::Instant::now();
    let final_frame_index = app.max_final_frame_index();

    let raw_input = winit_state.take_egui_input(window);

    // Allow users to drag and drop folders or files.
    for file in &raw_input.dropped_files {
        if let Some(path) = file.path.as_ref() {
            if path.is_file() {
                // Load the parent folder for files.
                if let Some(parent) = path.parent() {
                    app.add_folder_to_workspace(parent, false);
                }
            } else {
                app.add_folder_to_workspace(path, false);
            }
        }
    }

    if app.should_update_clear_color {
        // Assume an sRGB framebuffer, so convert sRGB to linear.
        let clear_color = app
            .preferences
            .viewport_color
            .map(|c| linear_f32_from_gamma_u8(c) as f64);
        renderer.set_clear_color(clear_color);
        app.should_update_clear_color = false;
    }

    if *previous_dark_mode != app.preferences.dark_mode {
        update_color_theme(app, ctx);
        *previous_dark_mode = app.preferences.dark_mode;
    }

    if app.animation_state.is_playing {
        app.animation_state.current_frame = next_frame(
            app.animation_state.current_frame,
            current_frame_start.duration_since(app.animation_state.previous_frame_start),
            final_frame_index,
            app.animation_state.playback_speed,
            app.animation_state.should_loop,
        );
    }
    app.animation_state.previous_frame_start = current_frame_start;

    let output_frame = match surface.get_current_texture() {
        Ok(frame) => frame,
        Err(e) => {
            // TODO: Return an error instead?
            eprintln!("Dropped frame with error: {e}");
            return;
        }
    };

    let output_view = output_frame
        .texture
        .create_view(&wgpu::TextureViewDescriptor::default());

    // Update the app to set the viewport rect before setting the scissor rect.
    // This prevents black bars when resizing the window.
    let full_output = ctx.run(raw_input, |ctx| {
        app.update(ctx);
    });

    let scissor_rect = app.viewport_rect(size.width, size.height, scale_factor as f32);
    renderer.set_scissor_rect(scissor_rect);

    refresh_render_state(app, renderer, egui_renderer, size, scale_factor);

    // Prepare the nutexb for rendering.
    // TODO: Avoid doing this each frame.
    let painter: &mut TexturePainter = egui_renderer.paint_callback_resources.get_mut().unwrap();
    if let Some((texture, dimension, size)) = get_nutexb_to_render(app) {
        painter.renderer.update(
            &app.render_state.device,
            &app.render_state.queue,
            texture,
            *dimension,
            size,
            &app.render_state.texture_render_settings,
        );
    }

    let mut encoder =
        app.render_state
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

    // TODO: Support opening editors from more than one folder?

    // First we draw the 3D viewport.
    // TODO: This doesn't need to be updated every frame?
    // TODO: Rework these fields to use Option<T>.
    let mask_model_index = app.ui_state.selected_folder_index.unwrap_or(0);
    app.render_state.model_render_options.mask_model_index = mask_model_index;
    app.render_state.model_render_options.mask_material_label =
        get_hovered_material_label(app, mask_model_index)
            .unwrap_or("")
            .to_owned();

    let mut final_pass = renderer.render_models(
        &mut encoder,
        &output_view,
        &app.render_models,
        app.render_state.shared_data.database(),
        &app.render_state.model_render_options,
    );

    for (render_model, hidden_collisions) in app
        .render_models
        .iter()
        .zip(app.swing_state.hidden_collisions.iter())
    {
        renderer.render_swing(&mut final_pass, render_model, hidden_collisions);
    }

    // TODO: Avoid calculating the MVP matrix every frame.
    // Overlay egui on the final pass to avoid a costly LoadOp::Load.
    // This improves performance on weak integrated graphics.
    // TODO: Why does this need a command encoder?
    let mut egui_encoder =
        app.render_state
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("egui Encoder"),
            });

    // TODO: Make the font size configurable.
    // TODO: Fix bone names appearing on top of the UI.
    if app.render_state.model_render_options.draw_bones && app.draw_bone_names {
        bone_name_renderer.render_bone_names(
            &app.render_state.device,
            &app.render_state.queue,
            &mut final_pass,
            &app.render_models,
            size.width,
            size.height,
            app.camera_state.mvp_matrix,
            18.0 * scale_factor as f32,
        );
    }

    egui_render_pass(
        ctx,
        full_output,
        &mut egui_encoder,
        winit_state,
        window,
        surface_config,
        egui_renderer,
        app,
        &mut final_pass,
        scale_factor,
    );

    drop(final_pass);

    // Submit the commands.
    app.render_state
        .queue
        .submit([encoder.finish(), egui_encoder.finish()]);

    // Present the final rendered image.
    output_frame.present();
    if let Some(file) = &app.screenshot_to_render {
        let rect = app.viewport_rect(size.width, size.height, scale_factor as f32);
        let image = render_screenshot(renderer, app, size.width, size.height, rect);
        if let Err(e) = image.save(file) {
            error!("Error saving screenshot to {:?}: {}", file, e);
        }
        app.screenshot_to_render = None;
    }

    // TODO: Avoid clone?
    if let Some(file) = app.animation_gif_to_render.clone() {
        // TODO: Run this on another thread?
        render_animation_to_gif(app, size, renderer, file, scale_factor);
        app.animation_gif_to_render = None;
    }

    if let Some(file) = app.animation_image_sequence_to_render.clone() {
        // TODO: Run this on another thread?
        render_animation_to_image_sequence(app, size, renderer, file, scale_factor);
        app.animation_image_sequence_to_render = None;
    }
}

fn refresh_render_state(
    app: &mut SsbhApp,
    renderer: &mut SsbhRenderer,
    egui_renderer: &mut egui_wgpu::Renderer,
    size: PhysicalSize<u32>,
    scale_factor: f64,
) {
    if app.should_update_lighting {
        update_lighting(renderer, app);
        app.should_update_lighting = false;
    }

    // TODO: Load models on a separate thread to avoid freezing the UI.
    reload_render_models(app, egui_renderer);

    if app.should_refresh_render_settings {
        renderer.update_render_settings(&app.render_state.queue, &app.render_state.render_settings);
        renderer
            .update_skinning_settings(&app.render_state.queue, &app.render_state.skinning_settings);
        app.should_refresh_render_settings = false;
    }

    if app.should_update_camera {
        app.render_state.camera_anim = app.camera_state.anim_path.as_ref().and_then(|path| {
            AnimData::from_file(path)
                .map_err(|e| {
                    error!("Error reading {:?}: {}", path, e);
                    e
                })
                .ok()
        });

        update_camera(
            &app.render_state.queue,
            renderer,
            &mut app.camera_state,
            size,
            scale_factor,
        );
        app.should_update_camera = false;
    }

    if app.should_validate_models {
        // Folders can be validated independently from one another.
        for model in &mut app.models {
            model.validate(&app.render_state.shared_data)
        }
        app.should_validate_models = false;
    }

    if app.swing_state.should_update_swing {
        for ((render_model, prc_index), model) in app
            .render_models
            .iter_mut()
            .zip(app.swing_state.selected_swing_folders.iter())
            .zip(app.models.iter())
        {
            if let Some(swing_prc) = prc_index
                .and_then(|prc_index| app.models.get(prc_index))
                .and_then(|m| m.swing_prc.as_ref())
            {
                render_model.recreate_swing_collisions(
                    &app.render_state.device,
                    swing_prc,
                    model.model.find_skel(),
                );
            }
        }
        app.swing_state.should_update_swing = false;
    }

    if app.animation_state.is_playing || app.animation_state.should_update_animations {
        animate_lighting(renderer, app);
        animate_viewport_camera(renderer, app, size, scale_factor);
        animate_models(app);
        app.animation_state.should_update_animations = false;
    }
}

fn create_app_data_directory() {
    let app_data_dir = PROJECT_DIR.data_local_dir();
    if let Err(e) = std::fs::create_dir_all(app_data_dir) {
        error!("Failed to create application directory at {app_data_dir:?}: {e}")
    }
}

fn render_animation_to_gif(
    app: &mut SsbhApp,
    size: PhysicalSize<u32>,
    renderer: &mut SsbhRenderer,
    file: std::path::PathBuf,
    scale_factor: f64,
) {
    // TODO: Rendering modifies the app, so this needs to be on the UI thread for now.
    let rect = app.viewport_rect(size.width, size.height, scale_factor as f32);
    let images = render_animation_sequence(renderer, app, size.width, size.height, rect);

    // TODO: Add progress indication.
    std::thread::spawn(move || match std::fs::File::create(&file) {
        Ok(file_out) => {
            let mut encoder = image::codecs::gif::GifEncoder::new(file_out);
            if let Err(e) = encoder.encode_frames(images.into_iter().map(image::Frame::new)) {
                error!("Error saving GIF to {file:?}: {e}");
            }
        }
        Err(e) => error!("Error creating file {file:?}: {e}"),
    });
}

fn render_animation_to_image_sequence(
    app: &mut SsbhApp,
    size: PhysicalSize<u32>,
    renderer: &mut SsbhRenderer,
    file: std::path::PathBuf,
    scale_factor: f64,
) {
    // TODO: Rendering modifies the app, so this needs to be on the UI thread for now.
    let rect = app.viewport_rect(size.width, size.height, scale_factor as f32);
    let images = render_animation_sequence(renderer, app, size.width, size.height, rect);

    // TODO: Add progress indication.
    std::thread::spawn(move || {
        for (i, image) in images.iter().enumerate() {
            // TODO: Find a simpler way to do this.
            let file_name = file
                .with_extension("")
                .file_name()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_else(|| "img".to_owned());
            let extension = file
                .extension()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_else(|| "png".to_owned());
            let output = file
                .with_file_name(file_name + &i.to_string())
                .with_extension(extension);
            if let Err(e) = image.save(output) {
                error!("Error saving image to {:?}: {}", file, e);
            }
        }
    });
}

fn update_lighting(renderer: &mut SsbhRenderer, app: &mut SsbhApp) {
    app.render_state.lighting_data = LightingData::from_ui(&app.ui_state.stage_lighting);

    // light00.nuamb
    animate_lighting(renderer, app);

    // color_grading_lut.nutexb
    match &app.render_state.lighting_data.color_grading_lut {
        Some(lut) => {
            renderer.update_color_lut(&app.render_state.device, &app.render_state.queue, lut)
        }
        None => renderer.reset_color_lut(&app.render_state.device, &app.render_state.queue),
    };

    // reflection_cubemap.nutexb
    match &app.render_state.lighting_data.reflection_cube_map {
        Some(cube) => app.render_state.shared_data.update_stage_cube_map(
            &app.render_state.device,
            &app.render_state.queue,
            cube,
        ),
        None => {
            app.render_state
                .shared_data
                .reset_stage_cube_map(&app.render_state.device, &app.render_state.queue);
        }
    }

    // Updating the cube map requires reassigning model textures.
    for (render_model, model) in app.render_models.iter_mut().zip(app.models.iter()) {
        if let Some(matl) = model.model.find_matl() {
            render_model.recreate_materials(
                &app.render_state.device,
                &matl.entries,
                &app.render_state.shared_data,
            );
        }
    }
}

fn animate_lighting(renderer: &mut SsbhRenderer, app: &SsbhApp) {
    // Only the light00.nuanmb needs to animate.
    match &app.render_state.lighting_data.light {
        Some(light) => renderer.update_stage_uniforms(
            &app.render_state.queue,
            light,
            app.animation_state.current_frame,
        ),
        None => renderer.reset_stage_uniforms(&app.render_state.queue),
    }
}

fn animate_viewport_camera(
    renderer: &mut SsbhRenderer,
    app: &mut SsbhApp,
    size: PhysicalSize<u32>,
    scale_factor: f64,
) {
    if let Some(anim) = &app.render_state.camera_anim {
        if let Some(values) = animate_camera(
            anim,
            app.animation_state.current_frame,
            app.camera_state.fov_y_radians,
            NEAR_CLIP,
            FAR_CLIP,
        ) {
            let transforms = values.to_transforms(size.width, size.height, scale_factor);
            renderer.update_camera(&app.render_state.queue, transforms);

            // Apply the animation values to the viewport camera.
            // This reduces "snapping" when moving the camera while paused.
            // These changes won't take effect unless the user actually moves the camera.
            // Decomposition is necessary to account for different transform orders.
            let (_, r, t) = transforms.model_view_matrix.to_scale_rotation_translation();
            app.camera_state.translation = t;
            app.camera_state.rotation_radians = r.to_euler(glam::EulerRot::XYZ).into();
            app.camera_state.fov_y_radians = values.fov_y_radians;
            app.camera_state.mvp_matrix = transforms.mvp_matrix;
        }
    }
}

fn reload_render_models(app: &mut SsbhApp, egui_renderer: &mut egui_wgpu::renderer::Renderer) {
    // Only load render models that need to change to improve performance.
    // Attempt to preserve the model and mesh visibility if possible.
    match app.models_to_update {
        ItemsToUpdate::None => (),
        ItemsToUpdate::One(i) => {
            if let (Some(render_model), Some(model)) =
                (app.render_models.get_mut(i), app.models.get(i))
            {
                let mut new_render_model = RenderModel::from_folder(
                    &app.render_state.device,
                    &app.render_state.queue,
                    &model.model,
                    &app.render_state.shared_data,
                );
                copy_visibility(&mut new_render_model, render_model);

                *render_model = new_render_model;
            }
        }
        ItemsToUpdate::All => {
            let mut new_render_models = ssbh_wgpu::load_render_models(
                &app.render_state.device,
                &app.render_state.queue,
                app.models.iter().map(|m| &m.model),
                &app.render_state.shared_data,
            );

            for (new_render_model, old_render_model) in
                new_render_models.iter_mut().zip(app.render_models.iter())
            {
                copy_visibility(new_render_model, old_render_model);
            }

            app.render_models = new_render_models;
        }
    }

    // TODO: Move this out of this function.
    if app.should_update_thumbnails {
        for (model, render_model) in app.models.iter_mut().zip(app.render_models.iter()) {
            model.thumbnails = generate_model_thumbnails(
                egui_renderer,
                &model.model,
                render_model,
                &app.render_state.device,
                &app.render_state.queue,
            );
        }
        app.should_update_thumbnails = false;
    }

    app.models_to_update = ItemsToUpdate::None;
}

fn copy_visibility(new_render_model: &mut RenderModel, render_model: &RenderModel) {
    // Preserve the visibility from the old render model.
    new_render_model.is_visible = render_model.is_visible;

    // The new render meshes may be renamed, added, or deleted.
    // This makes it impossible to always find the old mesh in general.
    // Assume the mesh ordering remains the same for simplicity.
    for (new_mesh, old_mesh) in new_render_model
        .meshes
        .iter_mut()
        .zip(render_model.meshes.iter())
    {
        new_mesh.is_visible = old_mesh.is_visible;
    }
}

fn get_hovered_material_label(app: &SsbhApp, folder_index: usize) -> Option<&str> {
    Some(
        app.models
            .get(folder_index)?
            .model
            .find_matl()?
            .entries
            .get(app.ui_state.matl_editor.hovered_material_index?)?
            .material_label
            .as_str(),
    )
}

fn update_color_theme(app: &SsbhApp, ctx: &egui::Context) {
    if app.preferences.dark_mode {
        ctx.set_visuals(Visuals {
            widgets: widgets_dark(),
            ..Default::default()
        });
    } else {
        ctx.set_visuals(Visuals {
            widgets: widgets_light(),
            ..Visuals::light()
        });
    }
}

fn resize(
    app: &mut SsbhApp,
    renderer: &mut SsbhRenderer,
    surface_config: &mut wgpu::SurfaceConfiguration,
    size: &PhysicalSize<u32>,
    scale_factor: f64,
    surface: &wgpu::Surface,
) {
    surface_config.width = size.width;
    surface_config.height = size.height;
    surface.configure(&app.render_state.device, surface_config);

    let scissor_rect = app.viewport_rect(size.width, size.height, scale_factor as f32);
    renderer.resize(
        &app.render_state.device,
        size.width,
        size.height,
        scale_factor,
        scissor_rect,
    );

    update_camera(
        &app.render_state.queue,
        renderer,
        &mut app.camera_state,
        *size,
        scale_factor,
    );
}

fn get_nutexb_to_render(
    app: &SsbhApp,
) -> Option<(&wgpu::Texture, &wgpu::TextureViewDimension, (u32, u32, u32))> {
    let folder_index = app.ui_state.selected_folder_index?;
    let model = app.models.get(folder_index)?;
    let render_model = app.render_models.get(folder_index)?;

    // Assume file names are unique, so use the name instead of the index.
    let (name, nutexb) = model.model.nutexbs.get(app.ui_state.open_nutexb?)?;
    let nutexb = nutexb.as_ref().ok()?;

    render_model.get_texture(name).map(|(texture, dim)| {
        (
            texture,
            dim,
            (
                nutexb.footer.width,
                nutexb.footer.height,
                nutexb.footer.depth,
            ),
        )
    })
}

fn egui_render_pass<'a>(
    ctx: &egui::Context,
    full_output: egui::FullOutput,
    encoder: &mut wgpu::CommandEncoder,
    winit_state: &mut egui_winit::State,
    window: &winit::window::Window,
    surface_config: &wgpu::SurfaceConfiguration,
    egui_renderer: &'a mut egui_wgpu::renderer::Renderer,
    app: &SsbhApp,
    rpass: &mut wgpu::RenderPass<'a>,
    scale_factor: f64,
) {
    // The UI is layered on top.
    // Based on the egui_wgpu source found here:
    // https://github.com/emilk/egui/blob/master/egui-wgpu/src/winit.rs
    winit_state.handle_platform_output(window, ctx, full_output.platform_output);
    let clipped_primitives = ctx.tessellate(full_output.shapes);
    let screen_descriptor = egui_wgpu::renderer::ScreenDescriptor {
        size_in_pixels: [surface_config.width, surface_config.height],
        pixels_per_point: scale_factor as f32,
    };
    for (id, image_delta) in &full_output.textures_delta.set {
        egui_renderer.update_texture(
            &app.render_state.device,
            &app.render_state.queue,
            *id,
            image_delta,
        );
    }
    egui_renderer.update_buffers(
        &app.render_state.device,
        &app.render_state.queue,
        encoder,
        &clipped_primitives,
        &screen_descriptor,
    );

    // TODO: This should technically go after painting.
    for id in &full_output.textures_delta.free {
        egui_renderer.free_texture(id);
    }

    // Record all render passes.
    egui_renderer.render(rpass, &clipped_primitives, &screen_descriptor);
}

fn request_adapter(
    window: &winit::window::Window,
    backends: wgpu::Backends,
) -> Option<(wgpu::Surface, wgpu::Adapter)> {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends,
        ..Default::default()
    });
    let surface = unsafe { instance.create_surface(window).unwrap() };

    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        })
        .block_on()?;
    Some((surface, adapter))
}

fn update_camera(
    queue: &wgpu::Queue,
    renderer: &mut SsbhRenderer,
    camera_state: &mut CameraInputState,
    size: PhysicalSize<u32>,
    scale_factor: f64,
) {
    let (camera_pos, model_view_matrix, mvp_matrix) = calculate_mvp(
        size,
        camera_state.translation,
        camera_state.rotation_radians,
        camera_state.fov_y_radians,
    );
    let transforms = CameraTransforms {
        model_view_matrix,
        mvp_matrix,
        mvp_inv_matrix: mvp_matrix.inverse(),
        camera_pos,
        screen_dimensions: glam::Vec4::new(
            size.width as f32,
            size.height as f32,
            scale_factor as f32,
            0.0,
        ),
    };
    renderer.update_camera(queue, transforms);

    // Needed for bone name rendering.
    camera_state.mvp_matrix = mvp_matrix;
}

// TODO: Create a separate module for input handling?
fn handle_input(
    input_state: &mut CameraInputState,
    event: &WindowEvent,
    size: PhysicalSize<u32>,
) -> bool {
    // Return true if the camera should update.
    let mut changed = false;

    // TODO: Input handling can be it's own module.
    match event {
        WindowEvent::MouseInput { button, state, .. } => {
            // Track mouse clicks to only rotate when dragging while clicked.
            match button {
                MouseButton::Left => {
                    input_state.is_mouse_left_clicked = *state == ElementState::Pressed;
                    changed = true;
                }
                MouseButton::Right => {
                    input_state.is_mouse_right_clicked = *state == ElementState::Pressed;
                    changed = true;
                }
                _ => (),
            }
        }
        WindowEvent::CursorMoved { position, .. } => {
            if input_state.is_mouse_left_clicked {
                let delta_x = position.x - input_state.previous_cursor_position.x;
                let delta_y = position.y - input_state.previous_cursor_position.y;

                // Swap XY so that dragging left right rotates left right.
                input_state.rotation_radians.x += (delta_y * 0.01) as f32;
                input_state.rotation_radians.y += (delta_x * 0.01) as f32;

                changed = true;
            } else if input_state.is_mouse_right_clicked {
                let delta_x = position.x - input_state.previous_cursor_position.x;
                let delta_y = position.y - input_state.previous_cursor_position.y;

                // Translate an equivalent distance in screen space based on the camera.
                // The viewport height and vertical field of view define the conversion.
                let fac = input_state.fov_y_radians.sin() * input_state.translation.z.abs()
                    / size.height as f32;

                // Negate y so that dragging up "drags" the model up.
                input_state.translation.x += delta_x as f32 * fac;
                input_state.translation.y -= delta_y as f32 * fac;

                changed = true;
            }
            // Always update the position to avoid jumps when moving between clicks.
            input_state.previous_cursor_position = *position;
        }
        WindowEvent::MouseWheel { delta, .. } => {
            // Scale zoom speed with distance to make it easier to zoom out large scenes.
            let delta_z = match delta {
                MouseScrollDelta::LineDelta(_x, y) => *y * input_state.translation.z.abs() * 0.1,
                MouseScrollDelta::PixelDelta(p) => {
                    p.y as f32 * input_state.translation.z.abs() * 0.005
                }
            };

            // Clamp to prevent the user from zooming through the origin.
            input_state.translation.z = (input_state.translation.z + delta_z).min(-1.0);

            changed = true;
        }
        WindowEvent::KeyboardInput { input, .. } => {
            if let Some(keycode) = input.virtual_keycode {
                match keycode {
                    VirtualKeyCode::Left => input_state.translation.x += 0.25,
                    VirtualKeyCode::Right => input_state.translation.x -= 0.25,
                    VirtualKeyCode::Up => input_state.translation.y += 0.25,
                    VirtualKeyCode::Down => input_state.translation.y -= 0.25,
                    _ => (),
                }

                changed = true;
            }
        }
        _ => (),
    }

    changed
}
