#![windows_subsystem = "windows"]

use chrono::{DateTime, Utc};
use egui::color::linear_f32_from_gamma_u8;
use egui::Visuals;
use nutexb_wgpu::TextureRenderer;
use octocrab::models::repos::Release;
use pollster::FutureExt; // TODO: is this redundant with tokio?
use ssbh_editor::app::{SsbhApp, UiState};
use ssbh_editor::material::load_material_presets;
use ssbh_editor::preferences::AppPreferences;
use ssbh_editor::validation::ModelValidationErrors;
use ssbh_editor::{
    checkerboard_texture, default_fonts, default_text_styles, generate_default_thumbnails,
    generate_model_thumbnails, last_update_check_file, presets_file, widgets_dark, widgets_light,
    AnimationState, CameraInputState, RenderState, TexturePainter, PROJECT_DIR,
};
use ssbh_wgpu::{CameraTransforms, SsbhRenderer};
use std::iter;
use winit::{
    dpi::{PhysicalPosition, PhysicalSize},
    event::*,
    event_loop::ControlFlow,
};

// TODO: Separate project for camera + input handling?
fn calculate_mvp(
    size: winit::dpi::PhysicalSize<u32>,
    translation_xyz: glam::Vec3,
    rotation_xyz_radians: glam::Vec3,
    fov_y_radians: f32,
) -> (glam::Vec4, glam::Mat4, glam::Mat4) {
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
    // Animate at 60 fps regardless of the rendering framerate.
    // This relies on interpolation or frame skipping.
    // TODO: How robust is this timing implementation?
    // TODO: Create a module/tests for this?
    let delta_t = current.duration_since(previous);

    let millis_per_frame = 1000.0f64 / 60.0f64;
    let delta_t_frames = delta_t.as_millis() as f64 / millis_per_frame;

    let mut next_frame = current_frame + (delta_t_frames as f32 * playback_speed);

    // TODO: Each animation should loop individually.
    if next_frame > final_frame_index && should_loop {
        next_frame = 0.0;
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
        .unwrap()
        .block_on(
            octocrab
                .repos("ScanMountGoat", "ssbh_editor")
                .releases()
                .get_latest(),
        )
        .ok()
}

fn main() {
    // Initialize logging first in case app startup has warnings.
    log::set_logger(&*ssbh_editor::app::LOGGER)
        .map(|()| log::set_max_level(log::LevelFilter::Info))
        .unwrap();

    let icon = image::load_from_memory_with_format(
        include_bytes!("icons/ssbh_editor32.png"),
        image::ImageFormat::Png,
    )
    .unwrap();

    let event_loop = winit::event_loop::EventLoop::with_user_event();
    let window = winit::window::WindowBuilder::new()
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
        .build(&event_loop)
        .unwrap();

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

    // TODO: Avoid unwrap.
    let app_data_dir = PROJECT_DIR.data_local_dir();
    std::fs::create_dir_all(&app_data_dir).unwrap();

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

    let mut size = window.inner_size();

    // Use the ssbh_wgpu format to ensure compatibility.
    let surface_format = ssbh_wgpu::RGBA_COLOR_FORMAT;
    let mut surface_config = wgpu::SurfaceConfiguration {
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        format: surface_format,
        width: size.width as u32,
        height: size.height as u32,
        present_mode: wgpu::PresentMode::Fifo,
    };
    surface.configure(&device, &surface_config);

    // Initialize egui and winit state.
    let ctx = egui::Context::default();
    let mut egui_rpass = egui_wgpu::renderer::RenderPass::new(&device, surface_format, 1);
    let mut winit_state = egui_winit::State::new(&event_loop);
    winit_state.set_max_texture_side(device.limits().max_texture_dimension_2d as usize);
    winit_state.set_pixels_per_point(window.scale_factor() as f32);

    // Assume an sRGB framebuffer, so convert sRGB to linear.
    let clear_dark = widgets_dark().noninteractive.bg_fill.r();
    let clear_dark = [linear_f32_from_gamma_u8(clear_dark) as f64; 3];

    let clear_light = widgets_light().noninteractive.bg_fill.r();
    let clear_light = [linear_f32_from_gamma_u8(clear_light) as f64; 3];

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
    egui_rpass.paint_callback_resources.insert(TexturePainter {
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

    let red_checkerboard = checkerboard_texture(&device, &queue, &mut egui_rpass, [255, 0, 0, 255]);
    let yellow_checkerboard =
        checkerboard_texture(&device, &queue, &mut egui_rpass, [255, 255, 0, 255]);

    let render_state = RenderState::new(device, queue, surface_format);
    let default_thumbnails = generate_default_thumbnails(
        &mut egui_rpass,
        render_state.shared_data.default_textures(),
        &render_state.device,
        &render_state.queue,
    );

    let preferences = AppPreferences::load_from_file();

    // Track if keys like ctrl or alt are being pressed.
    let mut modifiers = ModifiersState::default();

    // TODO: Move this to a function.
    let mut app = SsbhApp {
        models: Vec::new(),
        render_models: Vec::new(),
        thumbnails: Vec::new(),
        validation_errors: Vec::new(),
        default_thumbnails,
        should_reload_models: false,
        should_show_update,
        new_release_tag,
        should_refresh_render_settings: false,
        should_refresh_camera_settings: false,
        should_validate_models: false,
        material_presets,
        red_checkerboard,
        yellow_checkerboard,
        draw_skeletons: false,
        draw_bone_names: false,
        draw_bone_axes: false,
        ui_state: UiState::default(),
        render_state,
        animation_state: AnimationState::new(),
        show_left_panel: true,
        show_right_panel: true,
        show_bottom_panel: true,
        camera_state,
        preferences,
    };

    // Make sure the theme updates if changed from preferences.
    // TODO: This is redundant with the initialization above?
    update_color_theme(&app, &ctx, &mut renderer, clear_dark, clear_light);
    let mut previous_dark_mode = false;

    // TODO: Does the T in the the event type matter here?
    event_loop.run(
        move |event: winit::event::Event<'_, usize>, _, control_flow| {
            match event {
                winit::event::Event::RedrawRequested(..) => {
                    // Don't render if the application window is minimized.
                    if window.inner_size().width > 0 && window.inner_size().height > 0 {
                        let raw_input = winit_state.take_egui_input(&window);

                        // Always update the frame times even if no animation is playing.
                        // This avoids skipping when resuming playback.
                        let current_frame_start = std::time::Instant::now();

                        let final_frame_index = app.max_final_frame_index();

                        if previous_dark_mode != app.preferences.dark_mode {
                            update_color_theme(&app, &ctx, &mut renderer, clear_dark, clear_light);
                        }

                        previous_dark_mode = app.preferences.dark_mode;

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
                        let scissor_rect = app.viewport_rect(
                            size.width,
                            size.height,
                            window.scale_factor() as f32,
                        );
                        renderer.set_scissor_rect(scissor_rect);

                        // TODO: Load models on a separate thread to avoid freezing the UI.
                        if app.should_reload_models {
                            reload_models(&mut app, &mut egui_rpass);
                            app.should_reload_models = false;
                        }

                        if app.should_refresh_render_settings {
                            renderer.update_render_settings(
                                &app.render_state.queue,
                                &app.render_state.render_settings,
                            );
                            app.should_refresh_render_settings = false;
                        }

                        if app.should_refresh_camera_settings {
                            update_camera(
                                &mut renderer,
                                &app.render_state.queue,
                                size,
                                &app.camera_state,
                                window.scale_factor(),
                            );
                            app.should_refresh_camera_settings = false;
                        }

                        if app.should_validate_models {
                            for (model, validation) in
                                app.models.iter().zip(app.validation_errors.iter_mut())
                            {
                                *validation = ModelValidationErrors::from_model(
                                    model,
                                    app.render_state.shared_data.database(),
                                );
                            }
                            app.should_validate_models = false;
                        }

                        // TODO: How to handle model.nuanmb?
                        if app.animation_state.is_playing
                            || app.animation_state.should_update_animations
                        {
                            animate_models(&mut app);
                            app.animation_state.should_update_animations = false;
                        }

                        // Prepare the nutexb for rendering.
                        // TODO: Avoid doing this each frame.
                        let painter: &mut TexturePainter =
                            egui_rpass.paint_callback_resources.get_mut().unwrap();
                        if let Some((texture, dimension, size)) = get_nutexb_to_render(&app) {
                            painter.renderer.update(
                                &app.render_state.device,
                                &app.render_state.queue,
                                texture,
                                *dimension,
                                size,
                                &app.render_state.texture_render_settings,
                            );
                        }

                        let mut encoder = app.render_state.device.create_command_encoder(
                            &wgpu::CommandEncoderDescriptor {
                                label: Some("Render Encoder"),
                            },
                        );

                        // First we draw the 3D viewport.
                        // TODO: Simplify these parameters.
                        // TODO: Support opening editors from more than one folder?
                        let mask_model_index = app.ui_state.selected_folder_index.unwrap_or(0);
                        let mut final_pass = renderer.render_models(
                            &mut encoder,
                            &output_view,
                            &app.render_models,
                            app.models.iter().map(|m| {
                                // TODO: Find a cleaner way to disable bone rendering.
                                if app.draw_skeletons {
                                    m.find_skel()
                                } else {
                                    None
                                }
                            }),
                            app.render_state.shared_data.database(),
                            app.draw_bone_axes,
                            mask_model_index,
                            get_hovered_material_label(&app, mask_model_index).unwrap_or(""),
                        );

                        // TODO: Avoid calculating the MVP matrix every frame.
                        // Overlay egui on the final pass to avoid a costly LoadOp::Load.
                        // This improves performance on weak integrated graphics.
                        egui_render_pass(
                            &ctx,
                            full_output,
                            &mut winit_state,
                            &window,
                            &surface_config,
                            &mut egui_rpass,
                            &app,
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
                        let bone_text_commands = if app.draw_skeletons && app.draw_bone_names {
                            renderer.render_skeleton_names(
                                &app.render_state.device,
                                &app.render_state.queue,
                                &output_view,
                                &app.render_models,
                                app.models.iter().map(|m| m.find_skel()),
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
                            .submit(std::iter::once(encoder.finish()));
                        if let Some(bone_text_commands) = bone_text_commands {
                            app.render_state
                                .queue
                                .submit(iter::once(bone_text_commands));
                        }
                        // Present the final rendered image.
                        output_frame.present();
                    }
                }
                winit::event::Event::WindowEvent { event, .. } => {
                    winit_state.on_event(&ctx, &event);

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
                        winit::event::WindowEvent::ModifiersChanged(new_modifiers) => {
                            modifiers = new_modifiers;
                        }
                        _ => {
                            // TODO: Is this the best place to handle keyboard shortcuts?
                            hande_keyboard_shortcuts(&event, modifiers, &mut app);

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

fn reload_models(app: &mut SsbhApp, egui_rpass: &mut egui_wgpu::renderer::RenderPass) {
    app.render_models = ssbh_wgpu::load_render_models(
        &app.render_state.device,
        &app.render_state.queue,
        &app.models,
        &app.render_state.shared_data,
    );
    app.validation_errors = app
        .models
        .iter()
        .map(|_| ModelValidationErrors::default())
        .collect();
    app.thumbnails = generate_model_thumbnails(
        egui_rpass,
        &app.models,
        &app.render_models,
        &app.render_state.device,
        &app.render_state.queue,
    );
}

fn animate_models(app: &mut SsbhApp) {
    for ((render_model, model), model_animations) in app
        .render_models
        .iter_mut()
        .zip(app.models.iter())
        .zip(app.animation_state.animations.iter())
    {
        // Only render enabled animations.
        let animations = model_animations
            .iter()
            .filter(|anim_slot| anim_slot.is_enabled)
            .filter_map(|anim_slot| {
                anim_slot
                    .animation
                    .and_then(|anim_index| anim_index.get_animation(&app.models))
                    .and_then(|(_, a)| a.as_ref().ok())
            });

        // TODO: Should animations loop independently if some animations are longer than others?

        // TODO: Make frame timing logic in ssbh_wgpu public?
        render_model.apply_anim(
            &app.render_state.queue,
            animations,
            model
                .skels
                .iter()
                .find(|(f, _)| f == "model.nusktb")
                .and_then(|(_, m)| m.as_ref().ok()),
            model
                .matls
                .iter()
                .find(|(f, _)| f == "model.numatb")
                .and_then(|(_, m)| m.as_ref().ok()),
            model
                .hlpbs
                .iter()
                .find(|(f, _)| f == "model.nuhlpb")
                .and_then(|(_, m)| m.as_ref().ok()),
            app.animation_state.current_frame,
            &app.render_state.shared_data,
        );
    }
}

fn get_hovered_material_label(app: &SsbhApp, folder_index: usize) -> Option<&str> {
    Some(
        app.models
            .get(folder_index)?
            .find_matl()?
            .entries
            .get(app.ui_state.matl_editor.hovered_material_index?)?
            .material_label
            .as_str(),
    )
}

fn update_color_theme(
    app: &SsbhApp,
    ctx: &egui::Context,
    renderer: &mut SsbhRenderer,
    clear_dark: [f64; 3],
    clear_light: [f64; 3],
) {
    if app.preferences.dark_mode {
        ctx.set_visuals(Visuals {
            widgets: widgets_dark(),
            ..Default::default()
        });
        renderer.set_clear_color(clear_dark);
    } else {
        ctx.set_visuals(Visuals {
            widgets: widgets_light(),
            ..Visuals::light()
        });
        renderer.set_clear_color(clear_light);
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
    let (name, nutexb) = model.nutexbs.get(app.ui_state.selected_nutexb_index?)?;
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
    winit_state: &mut egui_winit::State,
    window: &winit::window::Window,
    surface_config: &wgpu::SurfaceConfiguration,
    egui_rpass: &'a mut egui_wgpu::renderer::RenderPass,
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
        egui_rpass.update_texture(
            &app.render_state.device,
            &app.render_state.queue,
            *id,
            image_delta,
        );
    }
    egui_rpass.update_buffers(
        &app.render_state.device,
        &app.render_state.queue,
        &clipped_primitives,
        &screen_descriptor,
    );
    // Record all render passes.
    egui_rpass.execute_with_renderpass(rpass, &clipped_primitives, &screen_descriptor);
    // for id in &full_output.textures_delta.free {
    //     egui_rpass.free_texture(id);
    // }
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

fn hande_keyboard_shortcuts(event: &WindowEvent, modifiers: ModifiersState, app: &mut SsbhApp) {
    // Use command instead of ctrl on MacOS.
    const CTRL: ModifiersState = if cfg!(target_os = "macos") {
        ModifiersState::LOGO
    } else {
        ModifiersState::CTRL
    };
    const CTRL_SHIFT: ModifiersState = CTRL.union(ModifiersState::SHIFT);

    if let WindowEvent::KeyboardInput {
        input,
        is_synthetic,
        ..
    } = event
    {
        // Check for synthetic keys to avoid triggering events twice.
        if !is_synthetic {
            if let Some(key) = input.virtual_keycode {
                match (modifiers, key) {
                    (CTRL, VirtualKeyCode::O) => app.open_folder(),
                    (CTRL_SHIFT, VirtualKeyCode::O) => app.add_folder_to_workspace(),
                    (CTRL, VirtualKeyCode::R) => app.reload_workspace(),
                    _ => (),
                }
            }
        }
    }
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
