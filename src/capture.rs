use crate::{animate_models, app::SsbhApp};
use futures::executor::block_on;
use ssbh_wgpu::SsbhRenderer;
use std::num::NonZeroU32;

pub fn render_animation_sequence(
    renderer: &mut SsbhRenderer,
    app: &mut SsbhApp,
    width: u32,
    height: u32,
    output_rect: [u32; 4],
) -> Vec<image::ImageBuffer<image::Rgba<u8>, Vec<u8>>> {
    let saved_frame = app.animation_state.current_frame;

    let mut frames = Vec::new();

    // Render out an animation sequence using the loaded animations.
    let final_frame = app.max_final_frame_index();
    app.animation_state.current_frame = 0.0;
    while app.animation_state.current_frame <= final_frame {
        animate_models(app);
        let frame = render_screenshot(renderer, app, width, height, output_rect);
        frames.push(frame);

        app.animation_state.current_frame += 1.0;
    }

    // Restore any state we modified while animating.
    app.animation_state.current_frame = saved_frame;

    frames
}

// TODO: Add an option to make the background transparent.
pub fn render_screenshot(
    renderer: &mut SsbhRenderer,
    app: &SsbhApp,
    width: u32,
    height: u32,
    output_rect: [u32; 4],
) -> image::ImageBuffer<image::Rgba<u8>, Vec<u8>> {
    // Round up to satisfy alignment requirements for texture copies.
    let round_up = |x, n| ((x + n - 1) / n) * n;
    let screenshot_width = round_up(width, 64);
    let screenshot_height = height;

    // Use a separate texture for drawing since the swapchain isn't COPY_SRC.
    let screenshot_texture = app
        .render_state
        .device
        .create_texture(&wgpu::TextureDescriptor {
            label: Some("screenshot texture"),
            size: wgpu::Extent3d {
                width: screenshot_width,
                height: screenshot_height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: ssbh_wgpu::RGBA_COLOR_FORMAT,
            usage: wgpu::TextureUsages::COPY_SRC | wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
    let screenshot_view = screenshot_texture.create_view(&wgpu::TextureViewDescriptor::default());
    let mut encoder =
        app.render_state
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Screenshot Render Encoder"),
            });
    let final_pass = renderer.render_models(
        &mut encoder,
        &screenshot_view,
        &app.render_models,
        app.render_state.shared_data.database(),
        &app.render_state.model_render_options,
    );
    drop(final_pass);

    read_texture_to_image(
        encoder,
        &app.render_state.device,
        &app.render_state.queue,
        &screenshot_texture,
        screenshot_width,
        screenshot_height,
        output_rect,
    )
}

fn read_texture_to_image(
    mut encoder: wgpu::CommandEncoder,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    output: &wgpu::Texture,
    width: u32,
    height: u32,
    output_rect: [u32; 4],
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
        format: ssbh_wgpu::RGBA_COLOR_FORMAT,
        usage: wgpu::TextureUsages::COPY_SRC | wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    };

    encoder.copy_texture_to_buffer(
        wgpu::ImageCopyTexture {
            aspect: wgpu::TextureAspect::All,
            texture: output,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
        },
        wgpu::ImageCopyBuffer {
            buffer: &output_buffer,
            layout: wgpu::ImageDataLayout {
                offset: 0,
                // TODO: This needs to be aligned to 256 bytes?
                bytes_per_row: NonZeroU32::new(width * 4),
                rows_per_image: None,
            },
        },
        texture_desc.size,
    );

    queue.submit([encoder.finish()]);

    let image = read_buffer_to_image(&output_buffer, device, width, height, output_rect);
    output_buffer.unmap();

    image
}

fn read_buffer_to_image(
    output_buffer: &wgpu::Buffer,
    device: &wgpu::Device,
    width: u32,
    height: u32,
    output_rect: [u32; 4],
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

    // Crop the image to the viewport region.
    // This also removes any padding needed to meet alignment requirements.
    // TODO: This doesn't always crop correctly.
    image::imageops::crop(
        &mut buffer,
        output_rect[0],
        output_rect[1],
        output_rect[2],
        output_rect[3],
    )
    .to_image()
}
