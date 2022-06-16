use chrono::{DateTime, Utc};
use nutexb_wgpu::TextureRenderer;
use octocrab::models::repos::Release;
use pollster::FutureExt; // TODO: is this redundant with tokio?
use ssbh_editor::app::{AnimationIndex, SsbhApp};
use ssbh_editor::app::{AnimationState, RenderState, UiState};
use ssbh_editor::material::load_material_presets;
use ssbh_editor::{
    checkerboard_texture, default_text_styles, generate_default_thumbnails,
    generate_model_thumbnails, widgets_dark,
};
use ssbh_wgpu::{
    create_default_textures, CameraTransforms, PipelineData, RenderSettings, SsbhRenderer,
};
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
    state: &CameraInputState,
) -> (glam::Vec4, glam::Mat4, glam::Mat4) {
    let aspect = size.width as f32 / size.height as f32;
    let model_view_matrix = glam::Mat4::from_translation(state.translation_xyz)
        * glam::Mat4::from_rotation_x(state.rotation_xyz.x)
        * glam::Mat4::from_rotation_y(state.rotation_xyz.y);
    let perspective_matrix = glam::Mat4::perspective_rh_gl(0.5, aspect, 1.0, 400000.0);

    // TODO: Is this correct for the camera position?
    let (_, _, camera_pos) = (model_view_matrix)
        .inverse()
        .to_scale_rotation_translation();

    (
        glam::Vec4::from((camera_pos, 1.0)),
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

    let instance = wgpu::Instance::new(wgpu::Backends::PRIMARY);
    let surface = unsafe { instance.create_surface(&window) };

    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        })
        .block_on()
        .unwrap();

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

    // TODO: Is there an easier way to track changing window size?
    let mut size = window.inner_size();

    // Use the ssbh_wgpu format to ensure compatibility.
    let surface_format = ssbh_wgpu::RGBA_COLOR_FORMAT;
    let mut surface_config = wgpu::SurfaceConfiguration {
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        format: surface_format,
        width: size.width as u32,
        height: size.height as u32,
        present_mode: wgpu::PresentMode::Mailbox, // TODO: FIFO?
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

    let mut renderer = SsbhRenderer::new(
        &device,
        &queue,
        size.width,
        size.height,
        window.scale_factor(),
        wgpu::Color {
            // TODO: This doesn't match exactly due to gamma correction.
            r: (27.0f64 / 255.0f64).powf(2.2f64),
            g: (27.0f64 / 255.0f64).powf(2.2f64),
            b: (27.0f64 / 255.0f64).powf(2.2f64),
            a: 1.0,
        },
    );

    let texture_renderer = TextureRenderer::new(&device, surface_format);

    // TODO: How to organize the resources needed for viewport rendering?
    let default_textures = create_default_textures(&device, &queue);
    let stage_cube = ssbh_wgpu::load_default_cube(&device, &queue);

    // TODO: Should some of this state be moved to SsbhRenderer?
    // This would eliminate redundant shader loads.
    let pipeline_data = PipelineData::new(&device, surface_format);

    // TODO: Camera framing?
    let mut camera_state = CameraInputState {
        previous_cursor_position: PhysicalPosition { x: 0.0, y: 0.0 },
        is_mouse_left_clicked: false,
        is_mouse_right_clicked: false,
        translation_xyz: glam::Vec3::new(0.0, -5.0, -45.0),
        rotation_xyz: glam::Vec3::new(0.0, 0.0, 0.0),
    };

    update_camera(&mut renderer, &queue, size, &camera_state);

    // TODO: How to cache/store the thumbnails for nutexb textures?
    // TODO: How to ensure this cache remains up to date?
    // TODO: Should RenderModel expose its wgpu textures?
    let default_thumbnails = generate_default_thumbnails(
        &texture_renderer,
        &default_textures,
        &device,
        &queue,
        &mut egui_rpass,
    );

    let shader_database = ssbh_wgpu::create_database();

    // TODO: Log missing presets?
    let material_presets = load_material_presets("presets.json").unwrap_or_default();

    let red_checkerboard = checkerboard_texture(&device, &queue, &mut egui_rpass, [255, 0, 0, 255]);
    let yellow_checkerboard =
        checkerboard_texture(&device, &queue, &mut egui_rpass, [255, 255, 0, 255]);

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
        ui_state: UiState {
            material_editor_open: false,
            render_settings_open: false,
            modl_editor_advanced_mode: false,
            mesh_editor_advanced_mode: false,
            preset_window_open: false,
            selected_material_preset_index: 0,
            selected_folder_index: None,
            selected_skel_index: None,
            selected_matl_index: None,
            selected_modl_index: None,
            selected_mesh_index: None,
            selected_hlpb_index: None,
            selected_material_index: 0,
            right_panel_tab: ssbh_editor::app::PanelTab::MeshList,
            matl_editor_advanced_mode: false,
        },
        render_state: RenderState {
            device,
            queue,
            default_textures,
            stage_cube,
            pipeline_data,
            render_settings: RenderSettings::default(),
            shader_database,
        },
        animation_state: AnimationState {
            animations: Vec::new(),
            is_playing: false,
            current_frame: 0.0,
            previous_frame_start: std::time::Instant::now(),
            animation_frame_was_changed: false,
            selected_slot: 0,
        },
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
                            &texture_renderer,
                            &app.models,
                            &app.render_state.device,
                            &app.render_state.queue,
                            &mut egui_rpass,
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

                    if app.animation_state.is_playing
                        || app.animation_state.animation_frame_was_changed
                    {
                        for anim_index in &app.animation_state.animations {
                            let animation =
                                AnimationIndex::get_animation(anim_index.as_ref(), &app.models)
                                    .map(|(_, a)| a);

                            for (render_model, model) in
                                app.render_models.iter_mut().zip(app.models.iter())
                            {
                                // TODO: Why does this need an option?
                                // TODO: Make frame timing logic in ssbh_wgpu public?
                                // TODO: Modify ssbh_wgpu to take multiple anims to avoid clearing.
                                render_model.apply_anim(
                                    &app.render_state.device,
                                    &app.render_state.queue,
                                    animation,
                                    model
                                        .skels
                                        .iter()
                                        .find(|(f, _)| f == "model.nusktb")
                                        .map(|h| &h.1),
                                    model
                                        .matls
                                        .iter()
                                        .find(|(f, _)| f == "model.numatb")
                                        .map(|h| &h.1),
                                    model
                                        .hlpbs
                                        .iter()
                                        .find(|(f, _)| f == "model.nuhlpb")
                                        .map(|h| &h.1),
                                    app.animation_state.current_frame,
                                    &app.render_state.pipeline_data,
                                    &app.render_state.default_textures,
                                    &app.render_state.stage_cube,
                                    &app.render_state.shader_database,
                                );
                            }
                        }

                        app.animation_state.animation_frame_was_changed = false;
                    }

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

                    // TODO: Make a function for this.
                    // The UI is layered on top.
                    // Based on the egui_wgpu source found here:
                    // https://github.com/emilk/egui/blob/master/egui-wgpu/src/winit.rs
                    let full_output = ctx.run(raw_input, |ctx| {
                        app.update(ctx);
                    });

                    winit_state.handle_platform_output(&window, &ctx, full_output.platform_output);

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
                        &mut encoder,
                        &output_view,
                        &clipped_primitives,
                        &screen_descriptor,
                        None,
                    );

                    for id in &full_output.textures_delta.free {
                        egui_rpass.free_texture(id);
                    }
                    // end egui

                    // Submit the commands.
                    app.render_state.queue.submit(iter::once(encoder.finish()));

                    // Present the final rendered image.
                    output_frame.present();
                }
                winit::event::Event::WindowEvent { event, .. } => {
                    winit_state.on_event(&ctx, &event);

                    match event {
                        winit::event::WindowEvent::Resized(new_size) => {
                            // TODO: Is there an easier way to track changing window size?
                            size = new_size;

                            surface_config.width = size.width;
                            surface_config.height = size.height;
                            surface.configure(&app.render_state.device, &surface_config);

                            renderer.resize(
                                &app.render_state.device,
                                &app.render_state.queue,
                                size.width,
                                size.height,
                                window.scale_factor(),
                            );
                            update_camera(
                                &mut renderer,
                                &app.render_state.queue,
                                size,
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
                        _ => {
                            if ctx.wants_keyboard_input() || ctx.wants_pointer_input() {
                                // It's possible to interact with the UI with the mouse over the viewport.
                                // Disable tracking the mouse in this case to prevent unwanted camera rotations.
                                // This mostly affects resizing the left and right side panels.
                                camera_state.is_mouse_left_clicked = false;
                                camera_state.is_mouse_right_clicked = false;
                            } else {
                                // Only update the viewport camera if the user isn't interacting with the UI.
                                if handle_input(&mut camera_state, &event) {
                                    update_camera(
                                        &mut renderer,
                                        &app.render_state.queue,
                                        size,
                                        &camera_state,
                                    );
                                    // TODO: How to only execute this when settings change?
                                    renderer.update_render_settings(
                                        &app.render_state.queue,
                                        &app.render_state.render_settings,
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

fn update_camera(
    renderer: &mut SsbhRenderer,
    queue: &wgpu::Queue,
    size: PhysicalSize<u32>,
    camera_state: &CameraInputState,
) {
    let (camera_pos, model_view_matrix, mvp_matrix) = calculate_mvp(size, camera_state);
    let transforms = CameraTransforms {
        model_view_matrix,
        mvp_matrix,
        camera_pos: camera_pos.to_array(),
    };
    renderer.update_camera(queue, transforms);
}

// TODO: Create a separate module for input handling?
fn handle_input(input_state: &mut CameraInputState, event: &WindowEvent) -> bool {
    // Return true if this function handled the event.
    // TODO: Input handling can be it's own module with proper tests.
    // Just test if the WindowEvent object is handled correctly.
    // Test that some_fn(event, state) returns new state?
    match event {
        WindowEvent::MouseInput { button, state, .. } => {
            // Keep track mouse clicks to only rotate when dragging while clicked.
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
            // TODO: This isn't recommended for 3d camera control?
            // TODO: How to only adjust camera if the viewport is focused?
            if input_state.is_mouse_left_clicked {
                let delta_x = position.x - input_state.previous_cursor_position.x;
                let delta_y = position.y - input_state.previous_cursor_position.y;

                // Swap XY so that dragging left right rotates left right.
                input_state.rotation_xyz.x += (delta_y * 0.01) as f32;
                input_state.rotation_xyz.y += (delta_x * 0.01) as f32;
            } else if input_state.is_mouse_right_clicked {
                // TODO: Adjust speed based on camera distance and handle 0 distance.
                let delta_x = position.x - input_state.previous_cursor_position.x;
                let delta_y = position.y - input_state.previous_cursor_position.y;

                // Negate y so that dragging up "drags" the model up.
                input_state.translation_xyz.x += (delta_x * 0.1) as f32;
                input_state.translation_xyz.y -= (delta_y * 0.1) as f32;
            }
            // Always update the position to avoid jumps when moving between clicks.
            input_state.previous_cursor_position = *position;
            true
        }
        WindowEvent::MouseWheel { delta, .. } => {
            // TODO: Add tests for handling scroll events properly?
            input_state.translation_xyz.z += match delta {
                MouseScrollDelta::LineDelta(_x, y) => *y * 5.0,
                MouseScrollDelta::PixelDelta(p) => p.y as f32,
            };
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
