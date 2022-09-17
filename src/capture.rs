use crate::app::SsbhApp;
use futures::executor::block_on;
use ssbh_wgpu::SsbhRenderer;
use std::num::NonZeroU32;
use winit::dpi::PhysicalSize;

// TODO: Add an option to make the background transparent.
pub fn render_screenshot(
    renderer: &mut SsbhRenderer,
    app: &SsbhApp,
    size: PhysicalSize<u32>,
) -> image::ImageBuffer<image::Rgba<u8>, Vec<u8>> {
    // Round up to satisfy alignment requirements for texture copies.
    let round_up = |x, n| ((x + n - 1) / n) * n;
    let screenshot_width = round_up(size.width, 64);
    let screenshot_height = size.height;

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
        app.models.iter().map(|m| {
            // TODO: Find a cleaner way to disable bone rendering.
            if app.draw_skeletons {
                m.find_skel()
            } else {
                None
            }
        }),
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
        size.width,
        size.height,
    )
}

fn read_texture_to_image(
    mut encoder: wgpu::CommandEncoder,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    output: &wgpu::Texture,
    width: u32,
    height: u32,
    output_width: u32,
    output_height: u32,
) -> image::ImageBuffer<image::Rgba<u8>, Vec<u8>> {
    let output_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        size: width as u64 * height as u64 * 4,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        label: None,
        mapped_at_creation: false,
    });

    let texture_desc = wgpu::TextureDescriptor {
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
        label: None,
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

    let image = read_buffer_to_image(
        &output_buffer,
        device,
        width,
        height,
        output_width,
        output_height,
    );
    output_buffer.unmap();

    image
}

fn read_buffer_to_image(
    output_buffer: &wgpu::Buffer,
    device: &wgpu::Device,
    width: u32,
    height: u32,
    output_width: u32,
    output_height: u32,
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
    // Remove any padding needed to meet alignment requirements.
    image::imageops::crop(&mut buffer, 0, 0, output_width, output_height).to_image()
}
