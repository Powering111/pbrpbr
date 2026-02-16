use std::{collections::HashSet, sync::Arc, time::Instant};

use glam::{Mat4, Vec3};
use winit::{
    event::{ElementState, KeyEvent, WindowEvent},
    keyboard::{KeyCode, PhysicalKey},
    window::CursorGrabMode,
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

    cursor_visible: bool,
    pressed_key: HashSet<KeyCode>,
    mouse_motion: (f64, f64),
    frame_instant: std::time::Instant,
    time: u64,
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
                cull_mode: Some(wgpu::Face::Back),
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
            cursor_visible: false,
            frame_instant: Instant::now(),
            pressed_key: HashSet::new(),
            mouse_motion: (0.0, 0.0),
            time: 0,
        }
    }

    fn is_key_pressed(&mut self, code: KeyCode) -> bool {
        self.pressed_key.contains(&code)
    }

    fn update(&mut self) {
        let now = Instant::now();
        let dt = now - self.frame_instant;
        self.frame_instant = now;

        self.time += dt.as_nanos() as u64;

        let camera_speed = 10.0;
        let forward_dir = self.scene.camera.forward_vec();

        let right_dir = forward_dir.cross(Vec3::Y);

        let mut dir = Vec3::ZERO;
        if self.is_key_pressed(KeyCode::KeyW) {
            dir += forward_dir;
        }
        if self.is_key_pressed(KeyCode::KeyS) {
            dir -= forward_dir;
        }
        if self.is_key_pressed(KeyCode::KeyA) {
            dir -= right_dir;
        }
        if self.is_key_pressed(KeyCode::KeyD) {
            dir += right_dir;
        }
        if self.is_key_pressed(KeyCode::Space) {
            dir += Vec3::Y;
        }
        if self.is_key_pressed(KeyCode::ShiftLeft) {
            dir += Vec3::NEG_Y;
        }

        self.scene.camera.position += dir.normalize_or_zero() * camera_speed * dt.as_secs_f32();

        let sensitivity = 0.002;
        self.scene.camera.yaw -= sensitivity * self.mouse_motion.0 as f32;
        self.scene.camera.pitch -= sensitivity * self.mouse_motion.1 as f32;

        self.mouse_motion = (0.0, 0.0);

        let sensitivity = 1.0;
        if self.is_key_pressed(KeyCode::ArrowLeft) {
            self.scene.camera.yaw += sensitivity * dt.as_secs_f32();
        }
        if self.is_key_pressed(KeyCode::ArrowRight) {
            self.scene.camera.yaw -= sensitivity * dt.as_secs_f32();
        }
        if self.is_key_pressed(KeyCode::ArrowUp) {
            self.scene.camera.pitch += sensitivity * dt.as_secs_f32();
        }
        if self.is_key_pressed(KeyCode::ArrowDown) {
            self.scene.camera.pitch -= sensitivity * dt.as_secs_f32();
        }
        self.scene.camera.pitch = f32::clamp(
            self.scene.camera.pitch,
            -std::f32::consts::PI * 0.5,
            std::f32::consts::PI * 0.5,
        );

        if self.is_key_pressed(KeyCode::Minus) {
            self.scene.camera.yfov += 0.002
        }
        if self.is_key_pressed(KeyCode::Equal) {
            self.scene.camera.yfov -= 0.002
        }
        self.scene.camera.yfov = f32::clamp(self.scene.camera.yfov, 0.01, std::f32::consts::PI);
    }

    fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        let aspect_ratio =
            self.surface_configuration.width as f32 / self.surface_configuration.height as f32;
        let camera_matrix = self.scene.camera.get_matrix(aspect_ratio);
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
                    rot: model.transform.rot(),
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

    fn add_mouse_motion(&mut self, delta: (f64, f64)) {
        if !self.cursor_visible {
            self.mouse_motion.0 += delta.0;
            self.mouse_motion.1 += delta.1;
        }
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

    fn set_cursor_visible(&mut self, visible: bool) {
        self.cursor_visible = visible;
        self.window.set_cursor_visible(visible);
        if visible {
            self.window
                .set_cursor_grab(winit::window::CursorGrabMode::None)
                .unwrap();
        } else {
            self.window
                .set_cursor_grab(winit::window::CursorGrabMode::Locked)
                .unwrap();
        }
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
        window.set_cursor_grab(CursorGrabMode::Locked).unwrap();
        window.set_cursor_visible(false);

        self.context = Some(pollster::block_on(Context::new(Arc::new(window))));
    }

    fn device_event(
        &mut self,
        _event_loop: &winit::event_loop::ActiveEventLoop,
        _device_id: winit::event::DeviceId,
        event: winit::event::DeviceEvent,
    ) {
        let context = match &mut self.context {
            Some(context) => context,
            None => return,
        };

        if let winit::event::DeviceEvent::MouseMotion { delta } = event {
            context.add_mouse_motion(delta);
        }
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
                context.update();
                // redraw
                if let Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) =
                    context.render()
                {
                    let size = context.window.inner_size();
                    context.resize(size.width, size.height);
                }

                context.window.request_redraw();
            }
            WindowEvent::KeyboardInput { event, .. } => match event {
                KeyEvent {
                    physical_key: PhysicalKey::Code(KeyCode::Escape),
                    state: ElementState::Pressed,
                    ..
                } => event_loop.exit(),

                KeyEvent {
                    physical_key: PhysicalKey::Code(KeyCode::AltLeft),
                    state,
                    ..
                } => match state {
                    ElementState::Pressed => {
                        context.set_cursor_visible(true);
                    }
                    ElementState::Released => {
                        context.set_cursor_visible(false);
                    }
                },
                KeyEvent {
                    physical_key: PhysicalKey::Code(code),
                    state,
                    ..
                } => {
                    if state.is_pressed() {
                        context.pressed_key.insert(code);
                    } else {
                        context.pressed_key.remove(&code);
                    }
                }
                _ => (),
            },
            _ => (),
        }
    }
}

fn main() {
    let event_loop = winit::event_loop::EventLoop::new().unwrap();
    event_loop.run_app(&mut App::default()).unwrap();
}
