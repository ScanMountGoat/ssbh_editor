use chrono::{DateTime, Utc};
use egui::color::linear_f32_from_gamma_u8;
use nutexb_wgpu::TextureRenderer;
use octocrab::models::repos::Release;
use pollster::FutureExt; // TODO: is this redundant with tokio?
use ssbh_editor::app::{SsbhApp, UiState};
use ssbh_editor::material::load_material_presets;
use ssbh_editor::{
    checkerboard_texture, default_fonts, default_text_styles, generate_default_thumbnails,
    generate_model_thumbnails, widgets_dark, AnimationIndex, AnimationState, RenderState,
    TexturePainter,
};
use ssbh_wgpu::{create_default_textures, CameraTransforms, SsbhRenderer};
use std::iter;
use winit::{
    dpi::{PhysicalPosition, PhysicalSize},
    event::*,
    event_loop::ControlFlow,
};

struct CameraInputState {
    previous_cursor_position: PhysicalPosition<f64>,
    is_mouse_left_clicked: bool,
    is_mouse_right_clicked: bool,
    translation_xyz: glam::Vec3,
    rotation_xyz: glam::Vec3,
}

// TODO: Separate project for camera + input handling?
fn calculate_mvp(
    size: winit::dpi::PhysicalSize<u32>,
    translation_xyz: glam::Vec3,
    rotation_xyz: glam::Vec3,
) -> (glam::Vec4, glam::Mat4, glam::Mat4) {
    let aspect = size.width as f32 / size.height as f32;
    let model_view_matrix = glam::Mat4::from_translation(translation_xyz)
        * glam::Mat4::from_rotation_x(rotation_xyz.x)
        * glam::Mat4::from_rotation_y(rotation_xyz.y);
    let perspective_matrix = glam::Mat4::perspective_rh(0.5, aspect, 1.0, 400000.0);

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
) -> f32 {
    // Animate at 60 fps regardless of the rendering framerate.
    // This relies on interpolation or frame skipping.
    // TODO: How robust is this timing implementation?
    // TODO: Create a module/tests for this?
    let delta_t = current.duration_since(previous);

    let millis_per_frame = 1000.0f64 / 60.0f64;
    let delta_t_frames = delta_t.as_millis() as f64 / millis_per_frame;
    let playback_speed = 1.0;

    let mut next_frame = current_frame + (delta_t_frames * playback_speed) as f32;

    // TODO: Add behaviors other than looping.
    if next_frame > final_frame_index {
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

// TODO: Only check once per day.
// TODO: Display a changelog from the repository.
// TODO: Display update information once in the UI.
fn get_latest_release() -> Option<Release> {
    // TODO: Compare versions using the current version and tag.
    // TODO: Assume the tags use identical versioning to cargo.
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
    let event_loop = winit::event_loop::EventLoop::with_user_event();
    let window = winit::window::WindowBuilder::new()
        .with_decorations(true)
        .with_resizable(true)
        .with_transparent(false)
        .with_title(concat!("SSBH Editor ", env!("CARGO_PKG_VERSION")))
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

    let previous_update_check_time: Option<DateTime<Utc>> =
        std::fs::read_to_string("ssbh_editor_config.txt")
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
        // Assume an sRGB framebuffer, so convert sRGB to linear.
        [linear_f32_from_gamma_u8(27) as f64; 3],
        ssbh_editor::FONT_BYTES,
    );

    let texture_renderer = TextureRenderer::new(&device, surface_format);

    // TODO: Camera framing?
    let mut camera_state = CameraInputState {
        previous_cursor_position: PhysicalPosition { x: 0.0, y: 0.0 },
        is_mouse_left_clicked: false,
        is_mouse_right_clicked: false,
        translation_xyz: glam::Vec3::new(0.0, -8.0, -60.0),
        rotation_xyz: glam::Vec3::new(0.0, 0.0, 0.0),
    };

    update_camera(
        &mut renderer,
        &queue,
        size,
        &camera_state,
        window.scale_factor(),
    );

    let default_textures = create_default_textures(&device, &queue);

    // TODO: How to ensure this cache remains up to date?
    // TODO: Should RenderModel expose its wgpu textures?
    let default_thumbnails = generate_default_thumbnails(
        &texture_renderer,
        &default_textures,
        &device,
        &queue,
        &mut egui_rpass,
    );

    // TODO: Log missing presets?
    let material_presets = load_material_presets("presets.json").unwrap_or_default();

    let red_checkerboard = checkerboard_texture(&device, &queue, &mut egui_rpass, [255, 0, 0, 255]);
    let yellow_checkerboard =
        checkerboard_texture(&device, &queue, &mut egui_rpass, [255, 255, 0, 255]);

    // Make sure the texture preview is ready for accessed by the app.
    // State is stored in a type map because of lifetime requirements.
    // https://github.com/emilk/egui/blob/master/egui_demo_app/src/apps/custom3d_wgpu.rs
    egui_rpass.paint_callback_resources.insert(TexturePainter {
        renderer: texture_renderer,
        bind_group: None,
    });

    // Track if keys like ctrl or alt are being pressed.
    let mut modifiers = ModifiersState::default();

    let mut app = SsbhApp {
        models: Vec::new(),
        render_models: Vec::new(),
        thumbnails: Vec::new(),
        default_thumbnails,
        should_refresh_meshes: false,
        should_show_update,
        new_release_tag,
        should_refresh_render_settings: false,
        material_presets,
        red_checkerboard,
        yellow_checkerboard,
        draw_skeletons: false,
        draw_bone_names: false,
        ui_state: UiState::default(),
        render_state: RenderState::new(device, queue, surface_format, default_textures),
        animation_state: AnimationState::new(),
    };

    // Initialize logging.
    log::set_logger(&*ssbh_editor::app::LOGGER)
        .map(|()| log::set_max_level(log::LevelFilter::Info))
        .unwrap();

    // TODO: Does the T in the the event type matter here?
    event_loop.run(
        move |event: winit::event::Event<'_, usize>, _, control_flow| {
            match event {
                winit::event::Event::RedrawRequested(..) => {
                    let raw_input = winit_state.take_egui_input(&window);

                    // Always update the frame times even if no animation is playing.
                    // This avoids skipping when resuming playback.
                    let current_frame_start = std::time::Instant::now();

                    let final_frame_index = app.max_final_frame_index();

                    if app.animation_state.is_playing {
                        app.animation_state.current_frame = next_frame(
                            app.animation_state.current_frame,
                            app.animation_state.previous_frame_start,
                            current_frame_start,
                            final_frame_index,
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

                    // TODO: Load models on a separate thread to avoid freezing the UI.
                    if app.should_refresh_meshes {
                        app.render_models = ssbh_wgpu::load_render_models(
                            &app.render_state.device,
                            &app.render_state.queue,
                            &app.render_state.pipeline_data,
                            &app.models,
                            &app.render_state.default_textures,
                            &app.render_state.stage_cube,
                            &app.render_state.shader_database,
                        );

                        app.thumbnails = generate_model_thumbnails(
                            &mut egui_rpass,
                            &app.models,
                            &app.render_state.device,
                            &app.render_state.queue,
                        );
                        app.should_refresh_meshes = false;
                    }

                    if app.should_refresh_render_settings {
                        renderer.update_render_settings(
                            &app.render_state.queue,
                            &app.render_state.render_settings,
                        );
                        app.should_refresh_render_settings = false;
                    }

                    // TODO: How to handle model.nuanmb?
                    if app.animation_state.is_playing
                        || app.animation_state.animation_frame_was_changed
                    {
                        for (render_model, model) in
                            app.render_models.iter_mut().zip(app.models.iter())
                        {
                            let animations =
                                app.animation_state
                                    .animations
                                    .iter()
                                    .filter_map(|anim_index| {
                                        AnimationIndex::get_animation(
                                            anim_index.as_ref(),
                                            &app.models,
                                        )
                                        .and_then(|(_, a)| a.as_ref().ok())
                                    });

                            // TODO: Make frame timing logic in ssbh_wgpu public?
                            render_model.apply_anim(
                                &app.render_state.device,
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
                                &app.render_state.pipeline_data,
                                &app.render_state.default_textures,
                                &app.render_state.stage_cube,
                                &app.render_state.shader_database,
                            );
                        }

                        app.animation_state.animation_frame_was_changed = false;
                    }

                    // Prepare the nutexb for rendering.
                    // TODO: Avoid doing this each frame.
                    // TODO: Get textures from the RenderModel?
                    let painter: &mut TexturePainter =
                        egui_rpass.paint_callback_resources.get_mut().unwrap();
                    painter.bind_group = get_nutexb_bind_group(&app, painter);

                    let mut encoder = app.render_state.device.create_command_encoder(
                        &wgpu::CommandEncoderDescriptor {
                            label: Some("Render Encoder"),
                        },
                    );

                    // First we draw the 3D viewport and main background color.
                    // TODO: Customizeable background color for the renderer?
                    renderer.render_ssbh_passes(
                        &mut encoder,
                        &output_view,
                        &app.render_models,
                        &app.render_state.shader_database,
                    );

                    // TODO: Avoid calculating the MVP matrix every frame.
                    let (_, _, mvp) = calculate_mvp(
                        size,
                        camera_state.translation_xyz,
                        camera_state.rotation_xyz,
                    );

                    // TODO: Make the font size configurable.
                    let skels: Vec<_> = app.models.iter().map(|m| m.find_skel()).collect();
                    let bone_text_commands = if app.draw_skeletons {
                        renderer.render_skeleton(
                            &app.render_state.device,
                            &app.render_state.queue,
                            &mut encoder,
                            &output_view,
                            &app.render_models,
                            &skels,
                            size.width,
                            size.height,
                            mvp,
                            if app.draw_bone_names {
                                Some(18.0 * window.scale_factor() as f32)
                            } else {
                                None
                            },
                        )
                    } else {
                        None
                    };

                    // TODO: Find a better way to avoid drawing bone names over the UI.
                    let mut egui_encoder = app.render_state.device.create_command_encoder(
                        &wgpu::CommandEncoderDescriptor {
                            label: Some("egui Render Encoder"),
                        },
                    );

                    egui_render_pass(
                        &ctx,
                        raw_input,
                        &mut winit_state,
                        &window,
                        &surface_config,
                        &mut egui_rpass,
                        &mut app,
                        &mut egui_encoder,
                        output_view,
                    );

                    // Submit the commands.
                    app.render_state.queue.submit(iter::once(encoder.finish()));
                    if let Some(bone_text_commands) = bone_text_commands {
                        app.render_state
                            .queue
                            .submit(iter::once(bone_text_commands));
                    }
                    app.render_state.queue.submit(iter::once(egui_encoder.finish()));

                    // Present the final rendered image.
                    output_frame.present();
                }
                winit::event::Event::WindowEvent { event, .. } => {
                    winit_state.on_event(&ctx, &event);

                    match event {
                        winit::event::WindowEvent::Resized(_) => {
                            // Use the window size to avoid a potential error from size mismatches.
                            size = window.inner_size();

                            resize(
                                &mut renderer,
                                &mut surface_config,
                                &size,
                                window.scale_factor(),
                                &surface,
                                &app,
                                &camera_state,
                            );
                        }
                        winit::event::WindowEvent::CloseRequested => {
                            // TODO: Create an app.exit() method?
                            // TODO: Use json to support more settings.
                            // TODO: Where to store this on mac/linux?
                            std::fs::write("ssbh_editor_config.txt", update_check_time.to_string())
                                .unwrap();

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
                                camera_state.is_mouse_left_clicked = false;
                                camera_state.is_mouse_right_clicked = false;
                            } else {
                                // Only update the viewport camera if the user isn't interacting with the UI.
                                if handle_input(&mut camera_state, &event, size) {
                                    update_camera(
                                        &mut renderer,
                                        &app.render_state.queue,
                                        size,
                                        &camera_state,
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
    surface.configure(&app.render_state.device, &surface_config);

    renderer.resize(
        &app.render_state.device,
        &app.render_state.queue,
        size.width,
        size.height,
        scale_factor,
    );

    update_camera(
        renderer,
        &app.render_state.queue,
        *size,
        camera_state,
        scale_factor,
    );
}

fn get_nutexb_bind_group(
    app: &SsbhApp,
    painter: &TexturePainter,
) -> Option<nutexb_wgpu::BindGroup0> {
    let folder_index = app.ui_state.selected_folder_index?;
    let model = app.models.get(folder_index)?;
    let render_model = app.render_models.get(folder_index)?;

    // Assume file names are unique, so use the name instead of the index.
    let (name, nutexb) = model.nutexbs.get(app.ui_state.selected_nutexb_index?)?;
    let nutexb = nutexb.as_ref().ok()?;

    // Prevent a potential crash when trying to render cube maps.
    // TODO: Add support for cube maps to nutexb_wgpu.
    if nutexb.footer.layer_count > 1 {
        None
    } else {
        let texture = render_model.get_texture(name)?;

        let bind_group = painter.renderer.create_bind_group(
            &app.render_state.device,
            &texture,
            &app.render_state.texture_render_settings,
        );
        Some(bind_group)
    }
}

fn egui_render_pass(
    ctx: &egui::Context,
    raw_input: egui::RawInput,
    winit_state: &mut egui_winit::State,
    window: &winit::window::Window,
    surface_config: &wgpu::SurfaceConfiguration,
    egui_rpass: &mut egui_wgpu::renderer::RenderPass,
    app: &mut SsbhApp,
    encoder: &mut wgpu::CommandEncoder,
    output_view: wgpu::TextureView,
) {
    // The UI is layered on top.
    // Based on the egui_wgpu source found here:
    // https://github.com/emilk/egui/blob/master/egui-wgpu/src/winit.rs
    let full_output = ctx.run(raw_input, |ctx| {
        app.update(ctx);
    });
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
    egui_rpass.execute(
        encoder,
        &output_view,
        &clipped_primitives,
        &screen_descriptor,
        None,
    );
    for id in &full_output.textures_delta.free {
        egui_rpass.free_texture(id);
    }
    // end egui
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
                    (ModifiersState::CTRL, VirtualKeyCode::O) => app.open_folder(),
                    (ModifiersState::CTRL, VirtualKeyCode::R) => app.reload_workspace(),
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
        camera_state.rotation_xyz,
    );
    let transforms = CameraTransforms {
        model_view_matrix,
        mvp_matrix,
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
    // TODO: Input handling can be it's own module with proper tests.
    // Just test if the WindowEvent object is handled correctly.
    // Test that some_fn(event, state) returns new state?
    match event {
        WindowEvent::MouseInput { button, state, .. } => {
            // Track mouse clicks to only rotate when dragging while clicked.
            match (button, state) {
                (MouseButton::Left, ElementState::Pressed) => {
                    input_state.is_mouse_left_clicked = true
                }
                (MouseButton::Left, ElementState::Released) => {
                    input_state.is_mouse_left_clicked = false
                }
                (MouseButton::Right, ElementState::Pressed) => {
                    input_state.is_mouse_right_clicked = true
                }
                (MouseButton::Right, ElementState::Released) => {
                    input_state.is_mouse_right_clicked = false
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
                input_state.rotation_xyz.x += (delta_y * 0.01) as f32;
                input_state.rotation_xyz.y += (delta_x * 0.01) as f32;
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
            // TODO: Add tests for handling scroll events properly?
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
        input_state.rotation_xyz * 0.0,
    );
    let world = mvp.inverse() * glam::Vec4::new(x_clip as f32, y_clip as f32, 0.0, 1.0);

    let world_x = world.x * world.z;
    let world_y = world.y * world.z;
    (world_x, world_y)
}
