#![windows_subsystem = "windows"]

use chrono::{DateTime, Utc};
use egui::color::linear_f32_from_gamma_u8;
use egui::Visuals;
use log::error;
use nutexb::NutexbFile;
use nutexb_wgpu::TextureRenderer;
use octocrab::models::repos::Release;
use pollster::FutureExt;
use ssbh_data::prelude::*;
use ssbh_editor::Thumbnail;
// TODO: is this redundant with tokio?
use ssbh_editor::app::{ItemsToUpdate, SsbhApp, UiState};
use ssbh_editor::capture::{render_animation_sequence, render_screenshot};
use ssbh_editor::material::load_material_presets;
use ssbh_editor::preferences::AppPreferences;
use ssbh_editor::{
    animate_models, checkerboard_texture, default_fonts, default_text_styles,
    generate_default_thumbnails, generate_model_thumbnails,
    path::{last_update_check_file, presets_file, PROJECT_DIR},
    widgets_dark, widgets_light, AnimationState, CameraInputState, RenderState, TexturePainter,
};
use ssbh_wgpu::{CameraTransforms, RenderModel, SsbhRenderer};
use std::iter;
use std::path::Path;
use winit::{
    dpi::{PhysicalPosition, PhysicalSize},
    event::*,
    event_loop::ControlFlow,
};

// TODO: Split up this file into modules.

fn main() {
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

    // Use DX12 as a separate fallback to fix initialization on some Windows systems.
    let start = std::time::Instant::now();
    let (surface, adapter) =
        request_adapter(&window, wgpu::Backends::VULKAN | wgpu::Backends::METAL)
            .or_else(|| request_adapter(&window, wgpu::Backends::DX12))
            .unwrap();
    println!("Request compatible adapter: {:?}", start.elapsed());

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

    create_app_data_directory();

    let (update_check_time, new_release_tag, should_show_update) = check_for_updates();

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
    };
    surface.configure(&device, &surface_config);

    // Initialize egui and winit state.
    let ctx = egui::Context::default();
    let mut egui_renderer = egui_wgpu::renderer::Renderer::new(&device, surface_format, None, 1);
    let mut winit_state = egui_winit::State::new(&event_loop);
    winit_state.set_max_texture_side(device.limits().max_texture_dimension_2d as usize);
    winit_state.set_pixels_per_point(window.scale_factor() as f32);

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
        window.scale_factor(),
        [0.0; 3],
        ssbh_editor::FONT_BYTES,
    );

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
    let camera_state = CameraInputState::default();

    update_camera(
        &mut renderer,
        &queue,
        size,
        &camera_state,
        window.scale_factor(),
    );

    let presets_file = presets_file();
    let material_presets = load_material_presets(presets_file);

    let red_checkerboard =
        checkerboard_texture(&device, &queue, &mut egui_renderer, [255, 0, 0, 255]);
    let yellow_checkerboard =
        checkerboard_texture(&device, &queue, &mut egui_renderer, [255, 255, 0, 255]);

    let render_state = RenderState::new(device, queue, surface_format);
    let default_thumbnails = generate_default_thumbnails(
        &mut egui_renderer,
        render_state.shared_data.default_textures(),
        &render_state.device,
        &render_state.queue,
    );

    let preferences = AppPreferences::load_from_file();

    let mut app = create_app(
        default_thumbnails,
        should_show_update,
        new_release_tag,
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
                            &mut egui_renderer,
                            &mut previous_dark_mode,
                            &window,
                            &ctx,
                            &surface,
                            size,
                            &surface_config,
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
                                        &mut renderer,
                                        &mut surface_config,
                                        &size,
                                        window.scale_factor(),
                                        &surface,
                                        &app,
                                        &app.camera_state,
                                    );
                                }
                            }
                            winit::event::WindowEvent::CloseRequested => {
                                app.write_state_to_disk(update_check_time);
                                *control_flow = ControlFlow::Exit;
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
                                            &mut renderer,
                                            &app.render_state.queue,
                                            size,
                                            &app.camera_state,
                                            window.scale_factor(),
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

// TODO: Separate module for camera + input handling?
fn calculate_mvp(
    size: winit::dpi::PhysicalSize<u32>,
    translation_xyz: glam::Vec3,
    rotation_xyz_radians: glam::Vec3,
    fov_y_radians: f32,
) -> (glam::Vec4, glam::Mat4, glam::Mat4) {
    // TODO: Rework this to use the camera format from Smash (quaternion).
    let aspect = size.width as f32 / size.height as f32;
    let model_view_matrix = glam::Mat4::from_translation(translation_xyz)
        * glam::Mat4::from_rotation_x(rotation_xyz_radians.x)
        * glam::Mat4::from_rotation_y(rotation_xyz_radians.y);
    let perspective_matrix = glam::Mat4::perspective_rh(fov_y_radians, aspect, 1.0, 400000.0);

    let camera_pos = model_view_matrix.inverse().col(3);

    (
        camera_pos,
        model_view_matrix,
        perspective_matrix * model_view_matrix,
    )
}

// TODO: Make this part of the public API for ssbh_wgpu?
pub fn next_frame(
    current_frame: f32,
    previous: std::time::Instant,
    current: std::time::Instant,
    final_frame_index: f32,
    playback_speed: f32,
    should_loop: bool,
) -> f32 {
    // Convert elapsed time to a delta in frames.
    // This relies on interpolation or frame skipping.
    // TODO: How robust is this timing implementation?
    // TODO: Create a module/tests for this?
    let delta_t = current.duration_since(previous);

    // TODO: Ensure 60hz monitors always advanced by exactly one frame per refresh?
    let millis_per_frame = 1000.0f64 / 60.0f64;
    let delta_t_frames = delta_t.as_millis() as f64 / millis_per_frame;

    let mut next_frame = current_frame + (delta_t_frames as f32 * playback_speed);

    if next_frame > final_frame_index {
        if should_loop {
            // Wrap around to loop the animation.
            // This may not be seamless if the animations have different lengths.
            next_frame = if final_frame_index > 0.0 {
                next_frame.rem_euclid(final_frame_index)
            } else {
                // Use 0.0 instead of NaN for empty animations.
                0.0
            };
        } else {
            // Reduce chances of overflow.
            next_frame = final_frame_index;
        }
    }

    next_frame
}

// TODO: Test this.
fn should_check_for_release(
    previous_update_check_time: Option<DateTime<Utc>>,
    current_time: DateTime<Utc>,
) -> bool {
    if let Some(previous_update_check_time) = previous_update_check_time {
        // Check at most once per day.
        current_time.date() > previous_update_check_time.date()
    } else {
        true
    }
}

// TODO: Display a changelog from the repository.
fn get_latest_release() -> Option<Release> {
    let octocrab = octocrab::instance();
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .ok()?
        .block_on(
            octocrab
                .repos("ScanMountGoat", "ssbh_editor")
                .releases()
                .get_latest(),
        )
        .ok()
}

// TODO: Create struct for return type.
fn check_for_updates() -> (DateTime<Utc>, Option<String>, bool) {
    let last_update_check_file = last_update_check_file();

    let previous_update_check_time: Option<DateTime<Utc>> =
        std::fs::read_to_string(last_update_check_file)
            .unwrap_or_default()
            .parse()
            .ok();

    let update_check_time = Utc::now();

    // TODO: Add logging for update check?
    let start = std::time::Instant::now();
    let should_check_for_update =
        should_check_for_release(previous_update_check_time, update_check_time);
    let new_release_tag = if should_check_for_update {
        get_latest_release().map(|r| r.tag_name)
    } else {
        None
    };

    let should_show_update = if let Some(new_release_tag) = &new_release_tag {
        new_release_tag.as_str() > env!("CARGO_PKG_VERSION")
    } else {
        false
    };
    println!("Check for new release: {:?}", start.elapsed());

    (update_check_time, new_release_tag, should_show_update)
}

// TODO: Make this a method.
fn create_app(
    default_thumbnails: Vec<Thumbnail>,
    should_show_update: bool,
    new_release_tag: Option<String>,
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
        should_show_update,
        new_release_tag,
        should_update_lighting: false,
        should_refresh_render_settings: false,
        should_refresh_camera_settings: false,
        should_validate_models: false,
        should_update_clear_color: true,
        should_update_thumbnails: false,
        material_presets,
        red_checkerboard,
        yellow_checkerboard,
        draw_bone_names: false,
        ui_state: UiState::default(),
        render_state,
        animation_state: AnimationState::new(),
        show_left_panel: true,
        show_right_panel: true,
        show_bottom_panel: true,
        camera_state,
        preferences,
        enable_helper_bones: true,
        screenshot_to_render: None,
        animation_gif_to_render: None,
        animation_image_sequence_to_render: None,
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
    egui_renderer: &mut egui_wgpu::Renderer,
    previous_dark_mode: &mut bool,
    window: &winit::window::Window,
    ctx: &egui::Context,
    surface: &wgpu::Surface,
    size: PhysicalSize<u32>,
    surface_config: &wgpu::SurfaceConfiguration,
) {
    let raw_input = winit_state.take_egui_input(window);

    // Always update the frame times even if no animation is playing.
    // This avoids skipping when resuming playback.
    let current_frame_start = std::time::Instant::now();
    let final_frame_index = app.max_final_frame_index();

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
            app.animation_state.previous_frame_start,
            current_frame_start,
            final_frame_index,
            app.animation_state.playback_speed,
            app.animation_state.should_loop,
        );
    }
    app.animation_state.previous_frame_start = current_frame_start;

    let output_frame = match surface.get_current_texture() {
        Ok(frame) => frame,
        Err(e) => {
            eprintln!("Dropped frame with error: {}", e);
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

    // TODO: Only update scale factor variable when changed?
    let scissor_rect = app.viewport_rect(size.width, size.height, window.scale_factor() as f32);
    renderer.set_scissor_rect(scissor_rect);

    if app.should_update_lighting {
        update_lighting(renderer, app);
        app.should_update_lighting = false;
    }

    // TODO: Load models on a separate thread to avoid freezing the UI.
    reload_models(app, egui_renderer);

    if app.should_refresh_render_settings {
        renderer.update_render_settings(&app.render_state.queue, &app.render_state.render_settings);
        renderer
            .update_skinning_settings(&app.render_state.queue, &app.render_state.skinning_settings);
        app.should_refresh_render_settings = false;
    }

    if app.should_refresh_camera_settings {
        update_camera(
            renderer,
            &app.render_state.queue,
            size,
            &app.camera_state,
            window.scale_factor(),
        );
        app.should_refresh_camera_settings = false;
    }

    if app.should_validate_models {
        // Folders can be validated independently from one another.
        for model in &mut app.models {
            model.validate(&app.render_state.shared_data)
        }
        app.should_validate_models = false;
    }

    if app.animation_state.is_playing || app.animation_state.should_update_animations {
        animate_models(app);
        app.animation_state.should_update_animations = false;
    }

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
    );
    drop(final_pass);

    let (_, _, mvp) = calculate_mvp(
        size,
        app.camera_state.translation_xyz,
        app.camera_state.rotation_xyz_radians,
        app.camera_state.fov_y_radians,
    );

    // TODO: Make the font size configurable.
    // TODO: Fix bone names appearing on top of the UI.
    let bone_text_commands =
        if app.render_state.model_render_options.draw_bones && app.draw_bone_names {
            renderer.render_skeleton_names(
                &app.render_state.device,
                &app.render_state.queue,
                &output_view,
                app.render_models.iter(),
                app.models.iter().map(|m| m.model.find_skel()),
                size.width,
                size.height,
                mvp,
                18.0 * window.scale_factor() as f32,
            )
        } else {
            None
        };

    // Submit the commands.
    app.render_state
        .queue
        .submit([encoder.finish(), egui_encoder.finish()]);
    if let Some(bone_text_commands) = bone_text_commands {
        app.render_state
            .queue
            .submit(iter::once(bone_text_commands));
    }

    // Present the final rendered image.
    output_frame.present();
    if let Some(file) = &app.screenshot_to_render {
        let rect = app.viewport_rect(size.width, size.height, window.scale_factor() as f32);
        let image = render_screenshot(renderer, app, size.width, size.height, rect);
        if let Err(e) = image.save(file) {
            error!("Error saving screenshot to {:?}: {}", file, e);
        }
        app.screenshot_to_render = None;
    }

    // TODO: Avoid clone?
    if let Some(file) = app.animation_gif_to_render.clone() {
        // TODO: Run this on another thread?
        render_animation_to_gif(app, size, window, renderer, file);
        app.animation_gif_to_render = None;
    }

    if let Some(file) = app.animation_image_sequence_to_render.clone() {
        // TODO: Run this on another thread?
        render_animation_to_image_sequence(app, size, window, renderer, file);
        app.animation_image_sequence_to_render = None;
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
    window: &winit::window::Window,
    renderer: &mut SsbhRenderer,
    file: std::path::PathBuf,
) {
    // TODO: Rendering modifies the app, so this needs to be on the UI thread for now.
    let rect = app.viewport_rect(size.width, size.height, window.scale_factor() as f32);
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
    window: &winit::window::Window,
    renderer: &mut SsbhRenderer,
    file: std::path::PathBuf,
) {
    // TODO: Rendering modifies the app, so this needs to be on the UI thread for now.
    let rect = app.viewport_rect(size.width, size.height, window.scale_factor() as f32);
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
    // light00.nuamb
    match &app.ui_state.stage_lighting.light {
        Some(path) => {
            update_stage_uniforms(renderer, app, path);
        }
        None => renderer.reset_stage_uniforms(&app.render_state.queue),
    }

    // color_grading_lut.nutexb
    match &app.ui_state.stage_lighting.color_grading_lut {
        Some(path) => update_color_lut(app, renderer, path),
        None => renderer.reset_color_lut(&app.render_state.device, &app.render_state.queue),
    };

    // reflection_cubemap.nutexb
    match &app.ui_state.stage_lighting.reflection_cube_map {
        Some(path) => update_stage_cube_map(&mut app.render_state, path),
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

fn update_color_lut(app: &SsbhApp, renderer: &mut SsbhRenderer, path: &Path) {
    match NutexbFile::read_from_file(path) {
        Ok(nutexb) => {
            renderer.update_color_lut(&app.render_state.device, &app.render_state.queue, &nutexb);
        }
        Err(e) => error!("Error reading {:?}: {}", path, e),
    }
}

fn update_stage_cube_map(render_state: &mut RenderState, path: &Path) {
    match NutexbFile::read_from_file(path) {
        Ok(nutexb) => {
            render_state.shared_data.update_stage_cube_map(
                &render_state.device,
                &render_state.queue,
                &nutexb,
            );
        }
        Err(e) => error!("Error reading {:?}: {}", path, e),
    }
}

fn update_stage_uniforms(renderer: &mut SsbhRenderer, app: &SsbhApp, path: &Path) {
    match AnimData::from_file(path) {
        Ok(data) => {
            renderer.update_stage_uniforms(&app.render_state.queue, &data);
        }
        Err(e) => error!("Error reading {:?}: {}", path, e),
    }
}

fn reload_models(app: &mut SsbhApp, egui_renderer: &mut egui_wgpu::renderer::Renderer) {
    // Only load render models that need to change to improve performance.
    match app.models_to_update {
        ItemsToUpdate::None => (),
        ItemsToUpdate::One(i) => {
            if let (Some(render_model), Some(model)) =
                (app.render_models.get_mut(i), app.models.get(i))
            {
                *render_model = RenderModel::from_folder(
                    &app.render_state.device,
                    &app.render_state.queue,
                    &model.model,
                    &app.render_state.shared_data,
                );
            }
        }
        ItemsToUpdate::All => {
            app.render_models = ssbh_wgpu::load_render_models(
                &app.render_state.device,
                &app.render_state.queue,
                app.models.iter().map(|m| &m.model),
                &app.render_state.shared_data,
            );
        }
    }

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

    if app.models_to_update != ItemsToUpdate::None && app.preferences.autohide_expressions {
        app.hide_expressions();
    }

    app.models_to_update = ItemsToUpdate::None;
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
    renderer: &mut SsbhRenderer,
    surface_config: &mut wgpu::SurfaceConfiguration,
    size: &PhysicalSize<u32>,
    scale_factor: f64,
    surface: &wgpu::Surface,
    app: &SsbhApp,
    camera_state: &CameraInputState,
) {
    surface_config.width = size.width;
    surface_config.height = size.height;
    surface.configure(&app.render_state.device, surface_config);

    let scissor_rect = app.viewport_rect(size.width, size.height, scale_factor as f32);
    renderer.resize(
        &app.render_state.device,
        &app.render_state.queue,
        size.width,
        size.height,
        scale_factor,
        scissor_rect,
    );

    update_camera(
        renderer,
        &app.render_state.queue,
        *size,
        camera_state,
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
    let (name, nutexb) = model
        .model
        .nutexbs
        .get(app.ui_state.selected_nutexb_index?)?;
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
) {
    // The UI is layered on top.
    // Based on the egui_wgpu source found here:
    // https://github.com/emilk/egui/blob/master/egui-wgpu/src/winit.rs
    winit_state.handle_platform_output(window, ctx, full_output.platform_output);
    let clipped_primitives = ctx.tessellate(full_output.shapes);
    let screen_descriptor = egui_wgpu::renderer::ScreenDescriptor {
        size_in_pixels: [surface_config.width, surface_config.height],
        pixels_per_point: window.scale_factor() as f32,
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
    let instance = wgpu::Instance::new(backends);
    let surface = unsafe { instance.create_surface(window) };
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
    renderer: &mut SsbhRenderer,
    queue: &wgpu::Queue,
    size: PhysicalSize<u32>,
    camera_state: &CameraInputState,
    scale_factor: f64,
) {
    let (camera_pos, model_view_matrix, mvp_matrix) = calculate_mvp(
        size,
        camera_state.translation_xyz,
        camera_state.rotation_xyz_radians,
        camera_state.fov_y_radians,
    );
    let transforms = CameraTransforms {
        model_view_matrix: model_view_matrix.to_cols_array_2d(),
        mvp_matrix: mvp_matrix.to_cols_array_2d(),
        camera_pos: camera_pos.to_array(),
        screen_dimensions: [
            size.width as f32,
            size.height as f32,
            scale_factor as f32,
            0.0,
        ],
    };
    renderer.update_camera(queue, transforms);
}

// TODO: Create a separate module for input handling?
fn handle_input(
    input_state: &mut CameraInputState,
    event: &WindowEvent,
    size: PhysicalSize<u32>,
) -> bool {
    // Return true if this function handled the event.
    // TODO: Input handling can be it's own module.
    match event {
        WindowEvent::MouseInput { button, state, .. } => {
            // Track mouse clicks to only rotate when dragging while clicked.
            match button {
                MouseButton::Left => {
                    input_state.is_mouse_left_clicked = *state == ElementState::Pressed;
                }
                MouseButton::Right => {
                    input_state.is_mouse_right_clicked = *state == ElementState::Pressed;
                }
                _ => (),
            }
            true
        }
        WindowEvent::CursorMoved { position, .. } => {
            if input_state.is_mouse_left_clicked {
                let delta_x = position.x - input_state.previous_cursor_position.x;
                let delta_y = position.y - input_state.previous_cursor_position.y;

                // Swap XY so that dragging left right rotates left right.
                input_state.rotation_xyz_radians.x += (delta_y * 0.01) as f32;
                input_state.rotation_xyz_radians.y += (delta_x * 0.01) as f32;
            } else if input_state.is_mouse_right_clicked {
                let (current_x_world, current_y_world) =
                    screen_to_world(input_state, *position, size);

                let (previous_x_world, previous_y_world) =
                    screen_to_world(input_state, input_state.previous_cursor_position, size);

                let delta_x_world = current_x_world - previous_x_world;
                let delta_y_world = current_y_world - previous_y_world;

                // Negate y so that dragging up "drags" the model up.
                input_state.translation_xyz.x += delta_x_world;
                input_state.translation_xyz.y -= delta_y_world;
            }
            // Always update the position to avoid jumps when moving between clicks.
            input_state.previous_cursor_position = *position;

            true
        }
        WindowEvent::MouseWheel { delta, .. } => {
            // Scale zoom speed with distance to make it easier to zoom out large scenes.
            let delta_z = match delta {
                MouseScrollDelta::LineDelta(_x, y) => {
                    *y * input_state.translation_xyz.z.abs() * 0.1
                }
                MouseScrollDelta::PixelDelta(p) => {
                    p.y as f32 * input_state.translation_xyz.z.abs() * 0.005
                }
            };

            // Clamp to prevent the user from zooming through the origin.
            input_state.translation_xyz.z = (input_state.translation_xyz.z + delta_z).min(-1.0);
            true
        }
        WindowEvent::KeyboardInput { input, .. } => {
            if let Some(keycode) = input.virtual_keycode {
                match keycode {
                    VirtualKeyCode::Left => input_state.translation_xyz.x += 0.25,
                    VirtualKeyCode::Right => input_state.translation_xyz.x -= 0.25,
                    VirtualKeyCode::Up => input_state.translation_xyz.y += 0.25,
                    VirtualKeyCode::Down => input_state.translation_xyz.y -= 0.25,
                    _ => (),
                }
            }

            true
        }
        _ => false,
    }
}

// TODO: Move this to ssbh_wgpu and make tests.
fn screen_to_world(
    input_state: &CameraInputState,
    position: PhysicalPosition<f64>,
    size: PhysicalSize<u32>,
) -> (f32, f32) {
    // The translation input is in pixels.
    let x_pixels = position.x;
    let y_pixels = position.y;

    // We want a world translation to move the scene origin that many pixels.
    // Map from screen space to clip space in the range [-1,1].
    let x_clip = 2.0 * x_pixels / size.width as f64 - 1.0;
    let y_clip = 2.0 * y_pixels / size.height as f64 - 1.0;

    // Map to world space using the model, view, and projection matrix.
    // TODO: Avoid recalculating the matrix?
    // Rotation is applied first, so always translate in XY.
    // TODO: Does ignoring rotation like this work in general?
    let (_, _, mvp) = calculate_mvp(
        size,
        input_state.translation_xyz,
        input_state.rotation_xyz_radians * 0.0,
        input_state.fov_y_radians,
    );
    let world = mvp.inverse() * glam::Vec4::new(x_clip as f32, y_clip as f32, 0.0, 1.0);

    let world_x = world.x * world.z;
    let world_y = world.y * world.z;
    (world_x, world_y)
}
