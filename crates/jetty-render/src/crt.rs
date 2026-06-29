//! Crt post-effect — a GPU pass that samples a fully-rendered offscreen scene
//! and writes the result to the surface. This scaffold is a passthrough (exact
//! blit): the WGSL fragment simply returns `textureSample(src_tex, src_samp, uv)`.
//! Real CRT effects (scanlines, phosphor bloom, barrel distortion) are added in
//! subsequent tasks without touching the public API established here.
//!
//! One fullscreen-triangle pass with bindings {uniform, src_texture, sampler}
//! and a `replace` blend (this pass owns every output pixel). At all times the
//! output is identical to the input — the caller decides when to route through
//! this pass.
//!
//! Self-contained: our own wgpu/WGSL, no desktop-environment / compositor /
//! OS-specific code.

// 16-byte uniform (4 scalars). No vec3<f32> so the host buffer layout is exact.
// `resolution` carries physical pixel dimensions for future shader math; the two
// `_pad` fields keep the struct 16-byte aligned and ready for extension.
pub(crate) const CRT_SHADER: &str = r#"
struct P { res_x: f32, res_y: f32, _a: f32, _b: f32 };
@group(0) @binding(0) var<uniform> p: P;
@group(0) @binding(1) var src_tex: texture_2d<f32>;
@group(0) @binding(2) var src_samp: sampler;

struct VsOut { @builtin(position) pos: vec4<f32>, @location(0) uv: vec2<f32> };

@vertex
fn vs(@builtin(vertex_index) vi: u32) -> VsOut {
    var verts = array<vec2<f32>, 3>(vec2(-1.0, -1.0), vec2(3.0, -1.0), vec2(-1.0, 3.0));
    let v = verts[vi];
    var o: VsOut;
    o.pos = vec4(v, 0.0, 1.0);
    // uv in 0..1, y down (matches the offscreen frame's orientation).
    o.uv = vec2(v.x * 0.5 + 0.5, 1.0 - (v.y * 0.5 + 0.5));
    return o;
}

@fragment
fn fs(in: VsOut) -> @location(0) vec4<f32> {
    return textureSample(src_tex, src_samp, in.uv);
}
"#;

/// Per-frame uniform for the CRT pass.
///
/// Layout: 16 bytes, 4 flat `f32` scalars. No `vec3<f32>` so the Rust buffer
/// layout matches the WGSL struct exactly (see phosphor.rs:18 note).
/// `resolution` holds physical pixel dimensions for future shader math (scanline
/// pitch, curvature, etc.); `_pad` are reserved for extension.
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CrtUniform {
    /// Physical width and height in pixels.
    pub resolution: [f32; 2],
    /// Reserved / padding — keep the struct 16-byte aligned.
    pub _pad: [f32; 2],
}

pub struct Crt {
    pipeline: wgpu::RenderPipeline,
    uniform_buf: wgpu::Buffer,
    bind_group_layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    /// Cached (src view, bind group); rebuilt only when the src view changes
    /// (e.g. on window resize). The bind group references the stable uniform
    /// buffer + sampler and the per-frame src texture view. `RefCell` because
    /// `apply` takes `&self` (caller may hold a `&mut gpu`).
    cached_bind: std::cell::RefCell<Option<(wgpu::TextureView, wgpu::BindGroup)>>,
}

impl Crt {
    pub fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("crt-shader"),
            source: wgpu::ShaderSource::Wgsl(CRT_SHADER.into()),
        });

        let uniform_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("crt-uniform"),
            size: std::mem::size_of::<CrtUniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("crt-bgl"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("crt-sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::MipmapFilterMode::Nearest,
            ..Default::default()
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("crt-layout"),
            bind_group_layouts: &[Some(&bind_group_layout)],
            ..Default::default()
        });

        // Replace blend: this pass owns every output pixel (it re-samples the
        // whole frame), so write src directly with no blending.
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("crt-pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs"),
                buffers: &[],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs"),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });

        Self {
            pipeline,
            uniform_buf,
            bind_group_layout,
            sampler,
            cached_bind: std::cell::RefCell::new(None),
        }
    }

    /// Run the Crt pass: sample `src` (the offscreen rendered scene) and write
    /// the result into `dst` (the surface). Currently a passthrough (identity
    /// blit); real CRT effects are layered on in subsequent tasks.
    ///
    /// `width` and `height` are the physical pixel dimensions; they are written
    /// into the uniform for use by future shader stages.
    #[allow(clippy::too_many_arguments)]
    pub fn apply(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        dst: &wgpu::TextureView,
        src: &wgpu::TextureView,
        width: u32,
        height: u32,
        u: &CrtUniform,
    ) {
        // Build the uniform with the caller's data, overriding resolution from
        // the explicit width/height so callers don't have to compute it twice.
        let uniform = CrtUniform {
            resolution: [width as f32, height as f32],
            _pad: u._pad,
        };
        queue.write_buffer(&self.uniform_buf, 0, bytemuck::bytes_of(&uniform));

        // Rebuild the bind group only when the src view changes (resize);
        // otherwise reuse the cached one to avoid a GPU allocator round-trip on
        // every frame.
        let mut cache = self.cached_bind.borrow_mut();
        let stale = cache.as_ref().map(|(v, _)| v != src).unwrap_or(true);
        if stale {
            let bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("crt-bg"),
                layout: &self.bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: self.uniform_buf.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::TextureView(src),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: wgpu::BindingResource::Sampler(&self.sampler),
                    },
                ],
            });
            *cache = Some((src.clone(), bg));
        }
        let bind_group = &cache.as_ref().unwrap().1;

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("crt-encoder"),
        });
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("crt-pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: dst,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, bind_group, &[]);
            pass.draw(0..3, 0..1);
        }
        queue.submit(Some(encoder.finish()));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Validate the CRT WGSL shader compiles and passes naga's validator without
    /// requiring a GPU adapter. This is the always-run gate for the shader source.
    #[test]
    fn crt_shader_compiles() {
        let module = naga::front::wgsl::parse_str(CRT_SHADER)
            .expect("CRT_SHADER must parse as valid WGSL");
        let mut validator = naga::valid::Validator::new(
            naga::valid::ValidationFlags::all(),
            naga::valid::Capabilities::all(),
        );
        validator
            .validate(&module)
            .expect("CRT_SHADER must pass naga validation");
    }

    /// Smoke-test that `Crt::new` succeeds on a real wgpu device.
    /// Gated with `#[ignore]` because a GPU adapter may be unavailable in
    /// headless / CI environments. Run manually with:
    ///   cargo test -p jetty-render crt_new_with_device -- --ignored
    #[test]
    #[ignore]
    fn crt_new_with_device() {
        let instance = wgpu::Instance::default();
        let adapter = pollster::block_on(
            instance.request_adapter(&wgpu::RequestAdapterOptions::default()),
        )
        .expect("adapter");
        let (device, _queue) = pollster::block_on(
            adapter.request_device(&wgpu::DeviceDescriptor::default()),
        )
        .expect("device");
        let _crt = Crt::new(&device, wgpu::TextureFormat::Rgba8UnormSrgb);
    }
}
