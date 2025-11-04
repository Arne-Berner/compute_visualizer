//! Minimal sketch showing "instanced quad + SDF in fragment shader" for thick line rendering.
//! - Each instance carries segment endpoints (pixel coords) + radius (pixels).
//! - Vertex shader expands a unit quad per-instance into screen/pixel space.
//! - Fragment shader computes distance to the segment and emits a white pixel when inside the radius.
//! This sketch renders to the window (RGBA8) — swap this for an offscreen R8Unorm target if you need a mask texture.

use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};
use wgpu::util::DeviceExt;
use bytemuck::{Pod, Zeroable};
use std::mem;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable, Debug)]
struct Vertex {
    corner: [f32; 2], // x: along segment (-1..1), y: offset (-1..1)
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable, Debug)]
struct Instance {
    a: [f32; 2],   // endpoint A in pixel coordinates
    b: [f32; 2],   // endpoint B in pixel coordinates
    radius: f32,   // radius in pixels
    _pad: f32,     // padding to 16-byte alignment for safety
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable, Debug)]
struct ScreenUniform {
    size: [f32; 2], // width, height in pixels
    _pad: [f32; 2],
}

fn main() {
    // Init window & GPU
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_title("wgpu instanced-line-sdf sketch")
        .with_inner_size(winit::dpi::PhysicalSize::new(800, 600))
        .build(&event_loop)
        .unwrap();

    // Use pollster to block on async init
    let mut state = pollster::block_on(State::new(&window));

    event_loop.run(move |event, _, control_flow| match event {
        Event::WindowEvent {
            ref event,
            window_id,
        } if window_id == window.id() => match event {
            WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
            WindowEvent::Resized(size) => state.resize(*size),
            WindowEvent::ScaleFactorChanged { new_inner_size, .. } => state.resize(**new_inner_size),
            _ => {}
        },
        Event::RedrawRequested(_) => {
            state.update();
            match state.render() {
                Ok(_) => {}
                // Recreate the surface if lost
                Err(wgpu::SurfaceError::Lost) => state.recreate_surface(),
                // The system is out of memory, we should exit
                Err(wgpu::SurfaceError::OutOfMemory) => *control_flow = ControlFlow::Exit,
                Err(e) => eprintln!("{:?}", e),
            }
        }
        Event::MainEventsCleared => {
            // Redraw as fast as possible
            window.request_redraw();
        }
        _ => {}
    });
}

struct State {
    surface: wgpu::Surface,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    size: winit::dpi::PhysicalSize<u32>,

    vertex_buf: wgpu::Buffer,
    index_buf: wgpu::Buffer,
    index_count: u32,
    instance_buf: wgpu::Buffer,
    instance_count: u32,

    screen_ubo: wgpu::Buffer,
    screen_bind_group: wgpu::BindGroup,

    pipeline: wgpu::RenderPipeline,
}

impl State {
    async fn new(window: &winit::window::Window) -> Self {
        let size = window.inner_size();

        // Instance, adapter, device
        let backend = wgpu::Backends::all();
        let instance = wgpu::Instance::new(backend);
        let surface = unsafe { instance.create_surface(window) }.unwrap();
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .expect("Failed to find an adapter");

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("device"),
                    features: wgpu::Features::empty(),
                    limits: wgpu::Limits::default(),
                },
                None,
            )
            .await
            .expect("Failed to create device");

        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps
            .formats
            .iter()
            .copied()
            .find(|f| f.describe().srgb)
            .unwrap_or(surface_caps.formats[0]);

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: surface_caps.present_modes[0],
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
        };
        surface.configure(&device, &config);

        // Buffers 
        // those coordinates will be converted to the actual coordinates in the vertex shader
        let vertices: &[Vertex] = &[
            Vertex { corner: [-1.0, -1.0] },
            Vertex { corner: [ 1.0, -1.0] },
            Vertex { corner: [ 1.0,  1.0] },
            Vertex { corner: [-1.0,  1.0] },
        ];
        let indices: &[u16] = &[0, 1, 2, 2, 3, 0];

        let vertex_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("quad vertex buffer"),
            contents: bytemuck::cast_slice(vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let index_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("quad index buffer"),
            contents: bytemuck::cast_slice(indices),
            usage: wgpu::BufferUsages::INDEX,
        });

        // Example instances: two L-shaped segments
        let instances_data = vec![
            Instance { a: [100.0, 100.0], b: [700.0, 100.0], radius: 10.0, _pad: 0.0 },
            Instance { a: [700.0, 100.0], b: [700.0, 500.0], radius: 10.0, _pad: 0.0 },
            // Add more segments or generate from your point set...
        ];

        let instance_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("instance buffer"),
            contents: bytemuck::cast_slice(&instances_data),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        });

        // Screen uniform
        let screen = ScreenUniform {
            size: [size.width as f32, size.height as f32],
            _pad: [0.0, 0.0],
        };
        let screen_ubo = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("screen ubo"),
            contents: bytemuck::bytes_of(&screen),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        // Bind group for screen uniform
        let screen_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("screen bind group layout"),
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

        let screen_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("screen bind group"),
            layout: &screen_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: screen_ubo.as_entire_binding(),
            }],
        });

        // Shader
        let shader_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("line sdf shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/line_instanced.wgsl").into()),
        });

        // Pipeline layout
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("pipeline layout"),
            bind_group_layouts: &[&screen_bind_group_layout],
            push_constant_ranges: &[],
        });

        // Vertex buffer layouts
        let vertex_buffers = &[
            // per-vertex quad
            wgpu::VertexBufferLayout {
                array_stride: mem::size_of::<Vertex>() as wgpu::BufferAddress,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &[
                    wgpu::VertexAttribute {
                        offset: 0,
                        shader_location: 0,
                        format: wgpu::VertexFormat::Float32x2,
                    },
                ],
            },
            // per-instance data
            wgpu::VertexBufferLayout {
                array_stride: mem::size_of::<Instance>() as wgpu::BufferAddress,
                step_mode: wgpu::VertexStepMode::Instance,
                attributes: &[
                    // a: vec2<f32> -> location 1
                    wgpu::VertexAttribute {
                        offset: 0,
                        shader_location: 1,
                        format: wgpu::VertexFormat::Float32x2,
                    },
                    // b: vec2<f32> -> location 2
                    wgpu::VertexAttribute {
                        offset: 8,
                        shader_location: 2,
                        format: wgpu::VertexFormat::Float32x2,
                    },
                    // radius: f32 -> location 3
                    wgpu::VertexAttribute {
                        offset: 16,
                        shader_location: 3,
                        format: wgpu::VertexFormat::Float32,
                    },
                    // note: _pad occupies remaining bytes
                ],
            },
        ];

        // Render pipeline
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("line sdf pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader_module,
                entry_point: "vs_main",
                buffers: vertex_buffers,
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader_module,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
        });

        Self {
            surface,
            device,
            queue,
            config,
            size,

            vertex_buf,
            index_buf,
            index_count: indices.len() as u32,
            instance_buf,
            instance_count: instances_data.len() as u32,

            screen_ubo,
            screen_bind_group,

            pipeline,
        }
    }

    fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.size = new_size;
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);

            // update screen uniform
            let screen = ScreenUniform {
                size: [new_size.width as f32, new_size.height as f32],
                _pad: [0.0, 0.0],
            };
            self.queue.write_buffer(&self.screen_ubo, 0, bytemuck::bytes_of(&screen));
        }
    }

    fn recreate_surface(&mut self) {
        self.surface.configure(&self.device, &self.config);
    }

    fn update(&mut self) {
        // Update instances or other dynamic data here if desired.
        // Example: animate radius or positions by writing to self.instance_buf via queue.write_buffer.
    }

    fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        let output = self.surface.get_current_texture()?;
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder =
            self.device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("render encoder"),
                });

        {
            let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("main pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: true,
                    },
                })],
                depth_stencil_attachment: None,
            });

            rpass.set_pipeline(&self.pipeline);
            rpass.set_bind_group(0, &self.screen_bind_group, &[]);
            rpass.set_vertex_buffer(0, self.vertex_buf.slice(..));
            rpass.set_vertex_buffer(1, self.instance_buf.slice(..));
            rpass.set_index_buffer(self.index_buf.slice(..), wgpu::IndexFormat::Uint16);
            rpass.draw_indexed(0..self.index_count, 0, 0..self.instance_count);
        }

        self.queue.submit(Some(encoder.finish()));
        output.present();

        Ok(())
    }
}
