use std::sync::Arc;

use glam::Mat4;
use winit::{
    event::{ElementState, KeyEvent, WindowEvent},
    keyboard::{KeyCode, PhysicalKey},
};

mod model;
mod renderer;
mod texture;

struct Context {
    window: Arc<winit::window::Window>,
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    surface_configuration: wgpu::SurfaceConfiguration,
    render_pipeline: wgpu::RenderPipeline,
    depth_texture: texture::Texture,

    camera_uniform: renderer::Uniform,
    vertex_buffer: renderer::VertexBuffer,

    scene: model::Scene,
}

impl Context {
    async fn new(window: Arc<winit::window::Window>) -> Self {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,
            ..Default::default()
        });
        let surface = instance.create_surface(window.clone()).unwrap();
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                force_fallback_adapter: false,
                compatible_surface: Some(&surface),
            })
            .await
            .unwrap();

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor::default())
            .await
            .unwrap();

        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps
            .formats
            .iter()
            .find(|format| format.is_srgb())
            .copied()
            .unwrap_or(surface_caps.formats[0]);
        let size = window.inner_size();
        let surface_configuration = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: surface_caps.present_modes[0],
            desired_maximum_frame_latency: 2,
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
        };
        surface.configure(&device, &surface_configuration);

        let depth_texture = texture::Texture::create_depth_texture(&device, &surface_configuration);

        let camera_uniform = renderer::Uniform::new(&device, size_of::<Mat4>() as u64);
        let vertex_buffer = renderer::VertexBuffer::new(&device);

        let shader = device.create_shader_module(wgpu::include_wgsl!("shader/shader.wgsl"));
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[&camera_uniform.bind_group_layout],
            immediate_size: 0,
        });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: None,
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[renderer::Vertex::desc(), renderer::Instance::desc()],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_configuration.format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                // cull_mode: Some(wgpu::Face::Back),
                cull_mode: None,
                unclipped_depth: false,
                polygon_mode: wgpu::PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview_mask: None,
            cache: None,
        });

        let scene = model::Scene::from_glb("res/scene.glb").unwrap();

        Self {
            window,
            surface,
            device,
            queue,
            surface_configuration,
            render_pipeline,
            vertex_buffer,
            depth_texture,
            camera_uniform,
            scene,
        }
    }

    fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        let aspect_ratio =
            self.surface_configuration.width as f32 / self.surface_configuration.height as f32;
        let camera_matrix = match self.scene.camera.zfar {
            Some(zfar) => Mat4::perspective_rh(
                self.scene.camera.yfov,
                aspect_ratio,
                self.scene.camera.znear,
                zfar,
            ),
            None => Mat4::perspective_infinite_rh(
                self.scene.camera.yfov,
                aspect_ratio,
                self.scene.camera.znear,
            ),
        } * self.scene.camera.transform.matrix().inverse();
        self.camera_uniform
            .write(&self.queue, bytemuck::cast_slice(&[camera_matrix]));

        let mut vertices: Vec<renderer::Vertex> = Vec::new();
        let mut indices: Vec<u32> = Vec::new();
        let mut instances: Vec<renderer::Instance> = Vec::new();
        let mut draws: Vec<(u32, u32, i32, u32)> = Vec::new();
        for model in self.scene.models.iter() {
            for primitive in model.primitives.iter() {
                let base_index = vertices.len() as i32;
                draws.push((
                    indices.len() as u32,
                    indices.len() as u32 + primitive.indices.len() as u32,
                    base_index,
                    instances.len() as u32,
                ));
                vertices.extend_from_slice(primitive.vertices.as_slice());
                indices.extend_from_slice(primitive.indices.as_slice());

                instances.push(renderer::Instance {
                    model: model.transform.matrix(),
                });
            }
        }

        self.vertex_buffer
            .write(&self.device, &self.queue, &vertices, &indices, &instances);

        let output = self.surface.get_current_texture()?;
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: None,
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.0,
                            g: 0.0,
                            b: 0.0,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth_texture.view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                ..Default::default()
            });

            render_pass.set_pipeline(&self.render_pipeline);

            self.vertex_buffer.set(&mut render_pass);
            self.camera_uniform.set(&mut render_pass, 0);

            for (index_start, index_end, base_index, instance_num) in draws {
                render_pass.draw_indexed(
                    index_start..index_end,
                    base_index,
                    instance_num..instance_num + 1,
                );
            }
        }
        self.queue.submit(std::iter::once(encoder.finish()));
        self.window.pre_present_notify();
        output.present();

        Ok(())
    }

    fn resize(&mut self, width: u32, height: u32) {
        self.surface_configuration.width = width;
        self.surface_configuration.height = height;
        self.surface
            .configure(&self.device, &self.surface_configuration);

        self.depth_texture =
            texture::Texture::create_depth_texture(&self.device, &self.surface_configuration);

        self.window.request_redraw();
    }
}

#[derive(Default)]
struct App {
    context: Option<Context>,
}

impl winit::application::ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        let window = event_loop
            .create_window(
                winit::window::WindowAttributes::default().with_title("Physically based rendering"),
            )
            .unwrap();

        self.context = Some(pollster::block_on(Context::new(Arc::new(window))));
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        let context = match self.context.as_mut() {
            Some(context) => context,
            None => return,
        };

        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::RedrawRequested => {
                // redraw
                if let Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) =
                    context.render()
                {
                    let size = context.window.inner_size();
                    context.resize(size.width, size.height);
                }
            }
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        physical_key: PhysicalKey::Code(KeyCode::Escape),
                        state: ElementState::Pressed,
                        ..
                    },
                ..
            } => {
                event_loop.exit();
            }
            _ => (),
        }
    }
}

fn main() {
    let event_loop = winit::event_loop::EventLoop::new().unwrap();
    event_loop.run_app(&mut App::default()).unwrap();
}
