use bytemuck;
use std::{iter, sync::Arc, time::Duration};

use wgpu::util::{BufferInitDescriptor, DeviceExt};
use winit::{
    application::ApplicationHandler,
    event::*,
    event_loop::{ActiveEventLoop, EventLoop},
    window::Window,
};
use rand::Rng;

#[derive(Clone, Debug)]
pub struct BufferBundle {
    buffer: Vec<u32>,
    width: u32,
    height: u32,
}

#[derive(Clone, Debug)]
enum UserEvent {
    SetState(BufferBundle),
    UpdateBuffer(BufferBundle),
}
// uniform buffers need to be 16 byte aligned. the fields are not necessary, but are more obvious
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable, Debug)]
struct Vertex {
    corner: [f32; 2], // x: along segment (-1..1), y: offset (-1..1)
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable, Debug)]
struct Instance {
    a: [f32; 2],
    b: [f32; 2],
    radius: f32,
    _pad: f32,
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable, Debug)]
struct ScreenUniform {
    size: [f32; 2], // width, height in pixels
    _pad: [f32; 2],
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable, Debug)]
struct TextureDimsUniform {
    dims: [f32; 2], // width, height in pixels
    _pad: [f32; 2],
}

pub struct State {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    is_surface_configured: bool,

    window: Arc<Window>,

    screen_ubo: wgpu::Buffer,
    screen_bind_group: wgpu::BindGroup,

    dims_ubo: wgpu::Buffer,
    input_bind_group: wgpu::BindGroup,

    render_pipeline: wgpu::RenderPipeline,
}

impl State {
    async fn new(window: Arc<Window>, buffer_bundle: BufferBundle) -> anyhow::Result<State> {
        let win_size = window.inner_size();

        // The instance is a handle to our GPU
        // BackendBit::PRIMARY => Vulkan + Metal + DX12 + Browser WebGPU
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,
            ..Default::default()
        });

        let surface = instance.create_surface(window.clone()).unwrap();

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .unwrap();

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: None,
                required_features: wgpu::Features::empty(),
                experimental_features: wgpu::ExperimentalFeatures::disabled(),
                // WebGL doesn't support all of wgpu's features, so if
                // we're building for the web we'll have to disable some.
                required_limits: if cfg!(target_arch = "wasm32") {
                    wgpu::Limits::downlevel_webgl2_defaults()
                } else {
                    wgpu::Limits::default()
                },
                memory_hints: Default::default(),
                trace: wgpu::Trace::Off, // Trace path
            })
            .await
            .unwrap();

        let surface_caps = surface.get_capabilities(&adapter);
        // Shader code in this tutorial assumes an Srgb surface texture. Using a different
        // one will result all the colors comming out darker. If you want to support non
        // Srgb surfaces, you'll need to account for that when drawing to the frame.
        let surface_format = surface_caps
            .formats
            .iter()
            .copied()
            .find(|f| f.is_srgb())
            .unwrap_or(surface_caps.formats[0]);
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: win_size.width,
            height: win_size.height,
            present_mode: surface_caps.present_modes[0],
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        // surface.configure(&device, &config);

        // Vertex buffer layouts
        // getting the screen
        let screen = ScreenUniform {
            size: [win_size.width as f32, win_size.height as f32],
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
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
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

        let shader =
            device.create_shader_module(wgpu::include_wgsl!("shaders/compute_to_render.wgsl"));

        let dims = TextureDimsUniform {
            dims: [buffer_bundle.width as f32, buffer_bundle.height as f32],
            _pad: [0.0, 0.0],
        };
        let dims_ubo = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("screen ubo"),
            contents: bytemuck::bytes_of(&dims),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let input_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("input"),
            contents: bytemuck::cast_slice(&buffer_bundle.buffer),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::STORAGE,
        });

        let input_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("input_bind_group_layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None, // or Some(NonZeroU64::new(labels_size).unwrap())
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility:  wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
            });

        let input_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("input_bind_group"),
            layout: &input_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: input_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: dims_ubo.as_entire_binding(),
                },
            ],
        });

        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[&screen_bind_group_layout, &input_bind_group_layout],
                immediate_size: 0,
            });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None, // Some(wgpu::Face::Back),
                // Setting this to anything other than Fill requires Features::POLYGON_MODE_LINE
                // or Features::POLYGON_MODE_POINT
                polygon_mode: wgpu::PolygonMode::Fill,
                // Requires Features::DEPTH_CLIP_CONTROL
                unclipped_depth: false,
                // Requires Features::CONSERVATIVE_RASTERIZATION
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            // If the pipeline will be used with a multiview render pass, this
            // indicates how many array layers the attachments will have.
            multiview_mask: None,
            // Useful for optimizing shader compilation on Android
            cache: None,
        });

        Ok(Self {
            surface,
            device,
            queue,
            config,
            is_surface_configured: false,
            window,

            screen_ubo,
            screen_bind_group,

            dims_ubo,
            input_bind_group,

            render_pipeline,
        })
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        if width > 0 && height > 0 {
            self.is_surface_configured = true;
            self.config.width = width;
            self.config.height = height;
            self.surface.configure(&self.device, &self.config);
            let screen = ScreenUniform {
                size: [width as f32, height as f32],
                _pad: [0.0, 0.0],
            };
            self.queue
                .write_buffer(&self.screen_ubo, 0, bytemuck::bytes_of(&screen));
        }
    }
    pub fn change_buffer(&mut self, buffer_bundle: BufferBundle) {
        // muss eine neue sein self.input_bind_group
        let dims = TextureDimsUniform {
            dims: [buffer_bundle.width as f32, buffer_bundle.height as f32],
            _pad: [0.0, 0.0],
        };
        let dims_ubo = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("screen ubo"),
            contents: bytemuck::bytes_of(&dims),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        // self.queue.write_buffer(&self.dims_ubo, 0, bytemuck::bytes_of(&dims));

        let input_buffer = self.device.create_buffer_init(&BufferInitDescriptor {
            label: Some("input"),
            contents: bytemuck::cast_slice(&buffer_bundle.buffer),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::STORAGE,
        });

        self.input_bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("input_bind_group"),
            // TODO carefull with hard coded indices
            layout: &self.render_pipeline.get_bind_group_layout(1),
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: input_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: dims_ubo.as_entire_binding(),
                },
            ],
        });

    }

    fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        self.window.request_redraw();

        // We can't render unless the surface is configured
        if !self.is_surface_configured {
            return Ok(());
        }

        let output = self.surface.get_current_texture()?;
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.3,
                            g: 0.3,
                            b: 0.3,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
                multiview_mask: None,
            });

            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.set_bind_group(0, &self.screen_bind_group, &[]);
            render_pass.set_bind_group(1, &self.input_bind_group, &[]);
            render_pass.draw(0..3, 0..1);
        }

        self.queue.submit(iter::once(encoder.finish()));
        output.present();

        Ok(())
    }
}

pub struct App {
    state: Option<State>,
}

impl App {
    pub fn new() -> Self {
        Self {
            state: None,
        }
    }
}

// kann ich irgendwie in state userevent reinpacken?
// wenn es ein feld von state ist, würde es nicht direkt übernommen werden
// wie wird state hier weiter gegeben?
impl ApplicationHandler<UserEvent> for App {
    fn resumed(&mut self, _event_loop: &ActiveEventLoop) {
    }

    #[allow(unused_mut)]
    fn user_event(&mut self, event_loop: &ActiveEventLoop, mut event: UserEvent) {
        match event {
            UserEvent::SetState(buffer_bundle) => {
                let mut window_attributes = Window::default_attributes();
                let window = Arc::new(event_loop.create_window(window_attributes).unwrap());

                self.state = Some(pollster::block_on(State::new(window, buffer_bundle)).unwrap());
                        },
            UserEvent::UpdateBuffer(buffer_bundle) => 
            {
                if let Some(state) = self.state.as_mut() {
                    state.change_buffer(buffer_bundle);
                }
            }
        }

        // wenn UpdateBuffer, müsste updatebuffer die infos drin haben zum updaten.
        // Vielleicht haben beide Events einen buffer attached?
        // wenn SetState, soll das hier passieren
        // self.state = Some(pollster::block_on(State::new(window, buffer_bundle)).unwrap());
        // muss eine neue sein self.input_bind_group
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        let state = match &mut self.state {
            Some(canvas) => canvas,
            None => return,
        };

        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => state.resize(size.width, size.height),
            WindowEvent::RedrawRequested => {
                // TODO if state
                match state.render() {
                    Ok(_) => {}
                    // Reconfigure the surface if it's lost or outdated
                    Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                        let size = state.window.inner_size();
                        state.resize(size.width, size.height);
                    }
                    Err(e) => {
                        log::error!("Unable to render {e}");
                    }
                }
            }
            _ => {}
        }
    }
}

pub async fn run() -> anyhow::Result<()> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        env_logger::init();
    }

    let event_loop = EventLoop::<UserEvent>::with_user_event().build()?;
    let proxy = event_loop.create_proxy();

    let mut app = App::new();
    tokio::spawn(async move 
        {
            // let buffer_bundle = BufferBundle { buffer: vec![0, 1, 0, 1], width: 4, height: 1 };
            let buffer_bundle = random_buffer_bundle();
            proxy.send_event(UserEvent::SetState(buffer_bundle)).unwrap();
            loop {
                tokio::time::sleep(Duration::from_secs(5)).await;
                // let buffer_bundle = BufferBundle { buffer: vec![0,1], width: 1, height: 2 };
                let buffer_bundle = random_buffer_bundle();
                proxy.send_event(UserEvent::UpdateBuffer(buffer_bundle)).unwrap();
            }
        }
    );
    event_loop.run_app(&mut app)?;

    Ok(())
}

fn random_buffer_bundle() -> BufferBundle {
    let mut rng = rand::rng();
    let mut buffer = vec![];
    let height = rng.random::<u32>() % 100 + 1;
    let width = rng.random::<u32>() % 100 + 1;
    for _ in 0..height {
        for _ in 0..width {
            buffer.push(rng.random::<u32>() % 2);
        }
    }
    BufferBundle { buffer, width, height }
}

// struct for buffer dims+data

// fn for randomizing data in a buffer


#[tokio::main]
async fn main() {
    // env_logger::init();
    run().await.unwrap();
}
