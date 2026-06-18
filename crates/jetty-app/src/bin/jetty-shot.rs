/// Headless offscreen screenshot tool — renders one terminal frame to a PNG
/// with NO window, surface, or display.
///
/// Config via env:
///   JETTY_SHOT_OUT   — output path (default: /tmp/jetty-shot.png)
///   JETTY_SHOT_INPUT — ANSI bytes to feed the terminal (default: built-in sample)
use std::fs::File;
use std::io::BufWriter;

use jetty_render::TextLayer;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out_path =
        std::env::var("JETTY_SHOT_OUT").unwrap_or_else(|_| "/tmp/jetty-shot.png".to_string());

    let width: u32 = 1000;
    let height: u32 = 640;
    let font_size: f32 = 16.0;

    let default_input = "\x1b[1;32muser@host\x1b[0m:\x1b[1;34m~/jetty\x1b[0m$ ls --color\r\n\x1b[1;34msrc\x1b[0m  \x1b[33mCargo.toml\x1b[0m  \x1b[31mREADME.md\x1b[0m\r\n\x1b[1;32muser@host\x1b[0m:\x1b[1;34m~/jetty\x1b[0m$ \r\n";
    let input_bytes: Vec<u8> = match std::env::var("JETTY_SHOT_INPUT") {
        Ok(s) => s.into_bytes(),
        Err(_) => default_input.as_bytes().to_vec(),
    };

    // --- wgpu offscreen setup (no surface) ---
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
    let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::LowPower,
        compatible_surface: None,
        force_fallback_adapter: false,
    }))?;

    eprintln!(
        "jetty-shot: GPU adapter = {} ({:?})",
        adapter.get_info().name,
        adapter.get_info().backend
    );

    let (device, queue) =
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some("jetty-shot-device"),
            required_features: wgpu::Features::empty(),
            required_limits: wgpu::Limits::default(),
            memory_hints: wgpu::MemoryHints::default(),
            trace: wgpu::Trace::Off,
            ..Default::default()
        }))?;

    let format = wgpu::TextureFormat::Rgba8UnormSrgb;

    // --- Build TextLayer ---
    let mut text = TextLayer::new(&device, &queue, format, font_size);
    let (cell_w, cell_h) = text.cell_size();

    let cols = (width as f32 / cell_w).floor().max(1.0) as usize;
    let rows = (height as f32 / cell_h).floor().max(1.0) as usize;

    eprintln!("jetty-shot: grid = {cols}x{rows} cells (cell {cell_w:.1}x{cell_h:.1}px)");

    // --- Build terminal snapshot ---
    let mut terminal = jetty_core::Terminal::new(cols, rows);
    terminal.feed(&input_bytes);
    let snap = terminal.snapshot();

    // --- Create offscreen texture ---
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("jetty-shot-tex"),
        size: wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

    // --- Render to offscreen texture ---
    text.render_to(&device, &queue, &view, width, height, &snap)?;

    // --- Read back to CPU ---
    // wgpu requires bytes_per_row to be a multiple of 256.
    let unpadded = width * 4;
    let align: u32 = 256;
    let padded = ((unpadded + align - 1) / align) * align;

    let buffer_size = (padded * height) as u64;
    let readback_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("jetty-shot-readback"),
        size: buffer_size,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });

    let mut encoder =
        device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("readback") });
    encoder.copy_texture_to_buffer(
        wgpu::TexelCopyTextureInfo {
            texture: &texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::TexelCopyBufferInfo {
            buffer: &readback_buffer,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(padded),
                rows_per_image: Some(height),
            },
        },
        wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
    );
    queue.submit(Some(encoder.finish()));

    // Map and read the buffer.
    let (tx, rx) = std::sync::mpsc::channel();
    readback_buffer.slice(..).map_async(wgpu::MapMode::Read, move |result| {
        tx.send(result).ok();
    });
    device.poll(wgpu::PollType::wait_indefinitely())?;
    rx.recv()??;

    let padded_data = readback_buffer.slice(..).get_mapped_range();
    // Strip row padding: copy only the unpadded bytes per row.
    let mut tight: Vec<u8> = Vec::with_capacity((width * height * 4) as usize);
    for row in 0..height {
        let row_start = (row * padded) as usize;
        let row_end = row_start + unpadded as usize;
        tight.extend_from_slice(&padded_data[row_start..row_end]);
    }
    drop(padded_data);
    readback_buffer.unmap();

    // --- Write PNG ---
    let file = File::create(&out_path)?;
    let writer = BufWriter::new(file);
    let mut encoder = png::Encoder::new(writer, width, height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut png_writer = encoder.write_header()?;
    png_writer.write_image_data(&tight)?;
    drop(png_writer);

    let file_size = std::fs::metadata(&out_path)?.len();
    println!("wrote {} ({}x{}, {} bytes)", out_path, width, height, file_size);

    Ok(())
}
