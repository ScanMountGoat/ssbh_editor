use crate::{app::SsbhApp, RenderState};
use futures::executor::block_on;
use log::error;

pub fn render_screenshot(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    render_state: &mut RenderState,
    width: u32,
    height: u32,
    surface_format: wgpu::TextureFormat,
) -> image::ImageBuffer<image::Rgba<u8>, Vec<u8>> {
    // Force transparent for screenshots.
    render_state.renderer.set_clear_color([0.0; 4]);

    // Round up to satisfy alignment requirements for texture copies.
    let round_up = |x: u32, n: u32| x.div_ceil(n) * n;
    let screenshot_width = round_up(width, 64);
    let screenshot_height = height;

    // Use a separate texture for drawing since the swapchain isn't COPY_SRC.
    let screenshot_texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("screenshot texture"),
        size: wgpu::Extent3d {
            width: screenshot_width,
            height: screenshot_height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: surface_format,
        usage: wgpu::TextureUsages::COPY_SRC | wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    let screenshot_view = screenshot_texture.create_view(&wgpu::TextureViewDescriptor::default());
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("Screenshot Render Encoder"),
    });
    let final_pass = render_state.renderer.render_models(
        &mut encoder,
        &screenshot_view,
        &render_state.render_models,
        render_state.shared_data.database(),
        &render_state.model_render_options,
    );
    drop(final_pass);

    read_texture_to_image(
        encoder,
        device,
        queue,
        &screenshot_texture,
        screenshot_width,
        screenshot_height,
        surface_format,
    )
}

fn read_texture_to_image(
    mut encoder: wgpu::CommandEncoder,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    output: &wgpu::Texture,
    width: u32,
    height: u32,
    surface_format: wgpu::TextureFormat,
) -> image::ImageBuffer<image::Rgba<u8>, Vec<u8>> {
    let output_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        size: width as u64 * height as u64 * 4,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        label: None,
        mapped_at_creation: false,
    });

    let texture_desc = wgpu::TextureDescriptor {
        label: None,
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: surface_format,
        usage: wgpu::TextureUsages::COPY_SRC | wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    };

    encoder.copy_texture_to_buffer(
        wgpu::TexelCopyTextureInfo {
            aspect: wgpu::TextureAspect::All,
            texture: output,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
        },
        wgpu::TexelCopyBufferInfo {
            buffer: &output_buffer,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                // TODO: This needs to be aligned to 256 bytes?
                bytes_per_row: Some(width * 4),
                rows_per_image: None,
            },
        },
        texture_desc.size,
    );

    queue.submit([encoder.finish()]);

    let image = read_buffer_to_image(&output_buffer, device, width, height);
    output_buffer.unmap();

    image
}

fn read_buffer_to_image(
    output_buffer: &wgpu::Buffer,
    device: &wgpu::Device,
    width: u32,
    height: u32,
) -> image::ImageBuffer<image::Rgba<u8>, Vec<u8>> {
    // Save the output texture.
    // Adapted from WGPU Example https://github.com/gfx-rs/wgpu/tree/master/wgpu/examples/capture
    // TODO: Find ways to optimize this?
    let buffer_slice = output_buffer.slice(..);
    // TODO: Do this without another crate?
    let (tx, rx) = futures_intrusive::channel::shared::oneshot_channel();
    buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
        tx.send(result).unwrap();
    });
    device.poll(wgpu::Maintain::Wait);
    block_on(rx.receive()).unwrap().unwrap();
    let data = buffer_slice.get_mapped_range();
    let mut buffer =
        image::ImageBuffer::<image::Rgba<u8>, _>::from_raw(width, height, data.to_owned()).unwrap();

    // Convert BGRA to RGBA.
    buffer.pixels_mut().for_each(|p| p.0.swap(0, 2));

    buffer
}

pub fn render_animation_to_gif(
    app: &mut SsbhApp,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    render_state: &mut RenderState,
    width: u32,
    height: u32,
    file: std::path::PathBuf,
    surface_format: wgpu::TextureFormat,
) {
    // TODO: Rendering modifies the app, so this needs to be on the UI thread for now.
    let images = render_animation_sequence(
        app,
        device,
        queue,
        render_state,
        width,
        height,
        surface_format,
    );

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

pub fn render_animation_to_image_sequence(
    app: &mut SsbhApp,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    render_state: &mut RenderState,
    width: u32,
    height: u32,
    file: std::path::PathBuf,
    surface_format: wgpu::TextureFormat,
) {
    // TODO: Rendering modifies the app, so this needs to be on the UI thread for now.
    let images = render_animation_sequence(
        app,
        device,
        queue,
        render_state,
        width,
        height,
        surface_format,
    );

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
                error!("Error saving image to {file:?}: {e}");
            }
        }
    });
}

fn render_animation_sequence(
    app: &mut SsbhApp,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    render_state: &mut RenderState,
    width: u32,
    height: u32,
    surface_format: wgpu::TextureFormat,
) -> Vec<image::ImageBuffer<image::Rgba<u8>, Vec<u8>>> {
    let saved_frame = app.animation_state.current_frame;

    let mut frames = Vec::new();

    // Render out an animation sequence using the loaded animations.
    let final_frame = app.max_final_frame_index(render_state);
    app.animation_state.current_frame = 0.0;
    while app.animation_state.current_frame <= final_frame {
        app.animate_models(queue, render_state);
        let frame = render_screenshot(device, queue, render_state, width, height, surface_format);
        frames.push(frame);

        app.animation_state.current_frame += 1.0;
    }

    // Restore any state we modified while animating.
    app.animation_state.current_frame = saved_frame;

    frames
}
