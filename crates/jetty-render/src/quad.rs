use wgpu::util::DeviceExt;

const QUAD_SHADER: &str = r#"
struct Screen { size: vec2<f32>, _pad: vec2<f32> };
@group(0) @binding(0) var<uniform> screen: Screen;
struct VsOut { @builtin(position) pos: vec4<f32>, @location(0) color: vec4<f32> };
@vertex
fn vs(@builtin(vertex_index) vi: u32, @location(0) rect: vec4<f32>, @location(1) color: vec4<f32>) -> VsOut {
    var corners = array<vec2<f32>, 6>(vec2(0.,0.), vec2(1.,0.), vec2(0.,1.), vec2(0.,1.), vec2(1.,0.), vec2(1.,1.));
    let c = corners[vi];
    let px = rect.xy + c * rect.zw;
    let ndc = vec2(px.x / screen.size.x * 2.0 - 1.0, 1.0 - px.y / screen.size.y * 2.0);
    var o: VsOut; o.pos = vec4(ndc, 0.0, 1.0); o.color = color; return o;
}
fn s2l(c: f32) -> f32 { if (c <= 0.04045) { return c / 12.92; } return pow((c + 0.055) / 1.055, 2.4); }
@fragment
fn fs(in: VsOut) -> @location(0) vec4<f32> { return vec4(s2l(in.color.r), s2l(in.color.g), s2l(in.color.b), in.color.a); }
"#;

#[derive(Clone, Copy)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    pub color: [u8; 4],
}

pub struct QuadLayer {
    pipeline: wgpu::RenderPipeline,
    uniform_buf: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
}

impl QuadLayer {
    pub fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("quad-shader"),
            source: wgpu::ShaderSource::Wgsl(QUAD_SHADER.into()),
        });

        let uniform_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("quad-uniform"),
            size: 16,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("quad-bgl"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("quad-bg"),
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buf.as_entire_binding(),
            }],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("quad-layout"),
            bind_group_layouts: &[Some(&bind_group_layout)],
            ..Default::default()
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("quad-pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs"),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: 32,
                    step_mode: wgpu::VertexStepMode::Instance,
                    attributes: &[
                        wgpu::VertexAttribute {
                            shader_location: 0,
                            offset: 0,
                            format: wgpu::VertexFormat::Float32x4,
                        },
                        wgpu::VertexAttribute {
                            shader_location: 1,
                            offset: 16,
                            format: wgpu::VertexFormat::Float32x4,
                        },
                    ],
                }],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs"),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
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

        Self { pipeline, uniform_buf, bind_group }
    }

    pub fn render(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        view: &wgpu::TextureView,
        screen_w: u32,
        screen_h: u32,
        rects: &[Rect],
    ) {
        if rects.is_empty() {
            return;
        }

        let uniform_data: [f32; 4] = [screen_w as f32, screen_h as f32, 0.0, 0.0];
        queue.write_buffer(&self.uniform_buf, 0, bytemuck::cast_slice(&uniform_data));

        let mut instance_data: Vec<f32> = Vec::with_capacity(rects.len() * 8);
        for r in rects {
            instance_data.push(r.x);
            instance_data.push(r.y);
            instance_data.push(r.w);
            instance_data.push(r.h);
            instance_data.push(r.color[0] as f32 / 255.0);
            instance_data.push(r.color[1] as f32 / 255.0);
            instance_data.push(r.color[2] as f32 / 255.0);
            instance_data.push(r.color[3] as f32 / 255.0);
        }

        let instance_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("quad-instances"),
            contents: bytemuck::cast_slice(&instance_data),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("quad-encoder"),
        });
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("quad-pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
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
            pass.set_bind_group(0, &self.bind_group, &[]);
            pass.set_vertex_buffer(0, instance_buf.slice(..));
            pass.draw(0..6, 0..rects.len() as u32);
        }
        queue.submit(Some(encoder.finish()));
    }
}

pub fn scrollbar_rect(
    snapshot: &jetty_core::GridSnapshot,
    screen_w: u32,
    screen_h: u32,
) -> Option<Rect> {
    if snapshot.scroll_max == 0 {
        return None;
    }
    let total = snapshot.rows + snapshot.scroll_max;
    let thumb_h =
        (screen_h as f32 * snapshot.rows as f32 / total as f32).max(24.0);
    let frac = (snapshot.scroll_max - snapshot.scroll_offset) as f32
        / snapshot.scroll_max as f32;
    let thumb_y = frac * (screen_h as f32 - thumb_h);
    Some(Rect {
        x: screen_w as f32 - 8.0,
        y: thumb_y,
        w: 8.0,
        h: thumb_h,
        color: [150, 150, 165, 220],
    })
}
