//! Offscreen scene target + a final composite pass for full-frame effects.
//!
//! Normally the app paints the terminal straight to the swap-chain surface (and
//! that fast path is unchanged — see app.rs). But for the center-summon
//! materialization we need to transform the WHOLE composited frame: scale it from
//! the center and fade its alpha. That can't be done while drawing the individual
//! layers, so during a summon the app instead paints the scene into this
//! offscreen texture, then runs [`SceneComposite::composite`] to sample it back
//! onto the surface with a uniform center `scale` + `global_alpha`, also applying
//! the rounded-corner mask in the same pass.
//!
//! With `scale == 1.0`, `alpha == 1.0`, `radius == 0.0` the pass reproduces the
//! scene 1:1 (identity sample, full alpha, no rounding) — but the app only routes
//! through here while animating, so ordinary frames never pay the offscreen cost.

/// Params uploaded to the composite shader. Layout must match the WGSL `Params`.
/// `size` = target pixels, `scale` = center zoom (1.0 = none), `alpha` = global
/// multiply (1.0 = opaque), `radius` = rounded-corner radius in px (0.0 = square).
const COMPOSITE_SHADER: &str = r#"
struct Params { size: vec2<f32>, scale: f32, alpha: f32, radius: f32, _pad0: f32, _pad1: f32, _pad2: f32 };
@group(0) @binding(0) var<uniform> params: Params;
@group(0) @binding(1) var scene_tex: texture_2d<f32>;
@group(0) @binding(2) var scene_smp: sampler;

struct VsOut { @builtin(position) pos: vec4<f32>, @location(0) uv: vec2<f32> };

@vertex
fn vs(@builtin(vertex_index) vi: u32) -> VsOut {
    // Fullscreen triangle covering clip space.
    var verts = array<vec2<f32>, 3>(vec2(-1.0, -1.0), vec2(3.0, -1.0), vec2(-1.0, 3.0));
    let p = verts[vi];
    var o: VsOut;
    o.pos = vec4(p, 0.0, 1.0);
    // UV in 0..1 with y flipped to texture space.
    o.uv = vec2(p.x * 0.5 + 0.5, 1.0 - (p.y * 0.5 + 0.5));
    return o;
}

// Signed distance to a rounded rect (negative inside), in pixel space.
fn sd_round_rect(p: vec2<f32>, b: vec2<f32>, r: f32) -> f32 {
    let q = abs(p) - b + vec2(r, r);
    return min(max(q.x, q.y), 0.0) + length(max(q, vec2(0.0, 0.0))) - r;
}

@fragment
fn fs(in: VsOut) -> @location(0) vec4<f32> {
    // Scale the SAMPLE coordinate about the center by the inverse of the visual
    // scale: a visual scale > 1 means we read a smaller (zoomed-in) region. The
    // summon uses scale slightly < 1 growing to 1, so the frame appears to grow
    // out from the center.
    let inv = 1.0 / params.scale;
    let centered = (in.uv - vec2(0.5, 0.5)) * inv + vec2(0.5, 0.5);

    // Outside the source texture after the inverse transform → fully transparent
    // (the corners that scale-up would otherwise smear an edge texel).
    if (centered.x < 0.0 || centered.x > 1.0 || centered.y < 0.0 || centered.y > 1.0) {
        return vec4(0.0, 0.0, 0.0, 0.0);
    }

    var c = textureSample(scene_tex, scene_smp, centered);

    // Rounded-corner coverage in the TARGET's pixel space (so rounding tracks the
    // window edges, not the scaled content).
    if (params.radius > 0.0) {
        let px = in.uv * params.size;
        let half = params.size * 0.5;
        let d = sd_round_rect(px - half, half, params.radius);
        let cov = 1.0 - smoothstep(-0.75, 0.75, d);
        c = c * cov;
    }

    // Premultiplied-alpha frame: multiply all channels by the global alpha so the
    // fade stays premultiplication-consistent.
    return c * params.alpha;
}
"#;

pub struct SceneComposite {
    pipeline: wgpu::RenderPipeline,
    uniform_buf: wgpu::Buffer,
    bind_group_layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    /// Offscreen scene texture (the app renders the frame into this), recreated
    /// whenever the target size changes.
    texture: Option<wgpu::Texture>,
    view: Option<wgpu::TextureView>,
    bind_group: Option<wgpu::BindGroup>,
    size: (u32, u32),
    format: wgpu::TextureFormat,
}

impl SceneComposite {
    pub fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("scene-composite-shader"),
            source: wgpu::ShaderSource::Wgsl(COMPOSITE_SHADER.into()),
        });

        let uniform_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("scene-composite-uniform"),
            size: 32,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("scene-composite-bgl"),
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
            label: Some("scene-composite-sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::MipmapFilterMode::Nearest,
            ..Default::default()
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("scene-composite-layout"),
            bind_group_layouts: &[Some(&bind_group_layout)],
            ..Default::default()
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("scene-composite-pipeline"),
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
                    // The scene is already premultiplied; we write the transformed
                    // premultiplied result directly (REPLACE) into the cleared
                    // surface, so the swap-chain alpha stays correct.
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
            texture: None,
            view: None,
            bind_group: None,
            size: (0, 0),
            format,
        }
    }

    /// Ensure the offscreen scene texture matches `(width, height)`, recreating it
    /// (and its bind group) on a size change. Call this (mutable) BEFORE rendering;
    /// then hold [`SceneComposite::scene_view`] (immutable) as the render target.
    pub fn ensure(&mut self, device: &wgpu::Device, width: u32, height: u32) {
        let want = (width.max(1), height.max(1));
        if self.texture.is_none() || self.size != want {
            let texture = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("scene-offscreen-tex"),
                size: wgpu::Extent3d { width: want.0, height: want.1, depth_or_array_layers: 1 },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: self.format,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            });
            let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
            let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("scene-composite-bg"),
                layout: &self.bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry { binding: 0, resource: self.uniform_buf.as_entire_binding() },
                    wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(&view) },
                    wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::Sampler(&self.sampler) },
                ],
            });
            self.texture = Some(texture);
            self.view = Some(view);
            self.bind_group = Some(bind_group);
            self.size = want;
        }
    }

    /// The offscreen scene view to render into. Returns `None` until `ensure` has
    /// created it. Immutable, so it can be held while `composite` is also called.
    pub fn scene_view(&self) -> Option<&wgpu::TextureView> {
        self.view.as_ref()
    }

    /// Composite the offscreen scene onto `target` with a center `scale`, a
    /// uniform `global_alpha`, and a rounded-corner `radius` (px). The surface is
    /// cleared to transparent first, then the transformed scene is written.
    ///
    /// Must be called after `scene_view` (which creates the texture/bind group)
    /// and after the scene has been rendered into that view.
    pub fn composite(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        target: &wgpu::TextureView,
        width: u32,
        height: u32,
        scale: f32,
        global_alpha: f32,
        radius: f32,
    ) {
        let Some(bind_group) = &self.bind_group else { return };
        // Guard against a degenerate scale.
        let scale = scale.max(0.01);
        let params: [f32; 8] = [
            width as f32, height as f32, scale, global_alpha.clamp(0.0, 1.0),
            radius.max(0.0), 0.0, 0.0, 0.0,
        ];
        queue.write_buffer(&self.uniform_buf, 0, bytemuck::cast_slice(&params));

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("scene-composite-encoder"),
        });
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("scene-composite-pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: target,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        // Clear to transparent so the faded/shrunken frame composites
                        // over an empty surface (the area outside the scaled content
                        // is transparent, revealing the desktop behind the window).
                        load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
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

/// CPU mirror of the composite transform, used by the headless `jetty-shot`
/// harness (`JETTY_SHOT_SUMMON_T`) so summon keyframes are inspectable without a
/// window. Given an already-rendered premultiplied-RGBA scene buffer, produce a
/// new buffer with the same center `scale` + `global_alpha` applied.
///
/// `radius` is intentionally NOT applied here — the shot harness applies its own
/// rounded-corner mask afterward (see jetty-shot.rs), matching the live ordering.
pub fn composite_cpu(
    scene: &[u8],
    width: u32,
    height: u32,
    scale: f32,
    global_alpha: f32,
) -> Vec<u8> {
    let w = width as usize;
    let h = height as usize;
    let scale = scale.max(0.01);
    let inv = 1.0 / scale;
    let alpha = global_alpha.clamp(0.0, 1.0);
    let mut out = vec![0u8; w * h * 4];
    for y in 0..h {
        for x in 0..w {
            // Map target pixel center to a source UV, inverse-scaled about center.
            let u = (x as f32 + 0.5) / width as f32;
            let v = (y as f32 + 0.5) / height as f32;
            let su = (u - 0.5) * inv + 0.5;
            let sv = (v - 0.5) * inv + 0.5;
            let oi = (y * w + x) * 4;
            if su < 0.0 || su > 1.0 || sv < 0.0 || sv > 1.0 {
                // Outside the source → transparent.
                continue;
            }
            // Bilinear sample of the premultiplied scene.
            let fx = su * width as f32 - 0.5;
            let fy = sv * height as f32 - 0.5;
            let x0 = fx.floor().clamp(0.0, (w - 1) as f32) as usize;
            let y0 = fy.floor().clamp(0.0, (h - 1) as f32) as usize;
            let x1 = (x0 + 1).min(w - 1);
            let y1 = (y0 + 1).min(h - 1);
            let tx = (fx - x0 as f32).clamp(0.0, 1.0);
            let ty = (fy - y0 as f32).clamp(0.0, 1.0);
            for c in 0..4 {
                let p00 = scene[(y0 * w + x0) * 4 + c] as f32;
                let p10 = scene[(y0 * w + x1) * 4 + c] as f32;
                let p01 = scene[(y1 * w + x0) * 4 + c] as f32;
                let p11 = scene[(y1 * w + x1) * 4 + c] as f32;
                let top = p00 + (p10 - p00) * tx;
                let bot = p01 + (p11 - p01) * tx;
                let val = (top + (bot - top) * ty) * alpha;
                out[oi + c] = val.round().clamp(0.0, 255.0) as u8;
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::composite_cpu;

    #[test]
    fn identity_is_lossless_at_scale_one_alpha_one() {
        // A 4x4 premultiplied scene; scale 1, alpha 1 should reproduce it ~exactly
        // (bilinear at integer offsets samples the same texel).
        let w = 4u32;
        let h = 4u32;
        let mut scene = vec![0u8; (w * h * 4) as usize];
        for (i, px) in scene.chunks_mut(4).enumerate() {
            px[0] = (i * 7 % 255) as u8;
            px[1] = (i * 13 % 255) as u8;
            px[2] = (i * 17 % 255) as u8;
            px[3] = 255;
        }
        let out = composite_cpu(&scene, w, h, 1.0, 1.0);
        // Interior pixels reproduce exactly; edge pixels may differ by ≤1 from
        // clamp rounding, so allow a tiny tolerance across the board.
        for (a, b) in scene.iter().zip(out.iter()) {
            assert!((*a as i32 - *b as i32).abs() <= 1, "{a} vs {b}");
        }
    }

    #[test]
    fn alpha_scales_all_channels() {
        let scene = vec![200u8; 16]; // 2x2, all (200,200,200,200) premultiplied.
        let out = composite_cpu(&scene, 2, 2, 1.0, 0.5);
        for v in out {
            assert_eq!(v, 100, "0.5 alpha should halve every channel");
        }
    }

    #[test]
    fn scale_down_leaves_transparent_border() {
        // scale 0.5 zooms OUT: the content shrinks, so the outer ring maps outside
        // the source and stays transparent.
        let w = 8u32;
        let h = 8u32;
        let scene = vec![255u8; (w * h * 4) as usize];
        let out = composite_cpu(&scene, w, h, 0.5, 1.0);
        // Corner pixel maps far outside the source → transparent.
        assert_eq!(out[3], 0, "corner alpha should be 0 at scale 0.5");
        // Center pixel stays opaque.
        let ci = ((h / 2) * w + w / 2) as usize * 4;
        assert!(out[ci + 3] > 200, "center should stay ~opaque");
    }
}
