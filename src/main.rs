use std::{collections::HashSet, sync::Arc, time::Instant};

use glam::Vec3;
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

    renderer: renderer::Renderer,

    scene: model::Scene,

    cursor_visible: bool,
    focused: bool,
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
            .request_device(&wgpu::DeviceDescriptor {
                required_features: wgpu::Features {
                    features_wgpu: wgpu::FeaturesWGPU::TEXTURE_BINDING_ARRAY,
                    features_webgpu: wgpu::FeaturesWebGPU::default(),
                },
                required_limits: wgpu::Limits {
                    max_binding_array_elements_per_shader_stage: 4,
                    ..Default::default()
                },
                ..Default::default()
            })
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

        let renderer = renderer::Renderer::new(&device, &surface_configuration);

        let scene = model::Scene::from_glb("res/scene2.glb").unwrap();

        Self {
            window,
            surface,
            device,
            queue,
            surface_configuration,
            renderer,
            scene,
            cursor_visible: true,
            focused: true,
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
        let dt_sec = dt.as_secs_f32();
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

        self.scene.camera.position += dir.normalize_or_zero() * camera_speed * dt_sec;

        let sensitivity = 0.002;
        self.scene.camera.yaw -= sensitivity * self.mouse_motion.0 as f32;
        self.scene.camera.pitch -= sensitivity * self.mouse_motion.1 as f32;

        self.mouse_motion = (0.0, 0.0);

        let sensitivity = 1.0;
        if self.is_key_pressed(KeyCode::ArrowLeft) {
            self.scene.camera.yaw += sensitivity * dt_sec;
        }
        if self.is_key_pressed(KeyCode::ArrowRight) {
            self.scene.camera.yaw -= sensitivity * dt_sec;
        }
        if self.is_key_pressed(KeyCode::ArrowUp) {
            self.scene.camera.pitch += sensitivity * dt_sec;
        }
        if self.is_key_pressed(KeyCode::ArrowDown) {
            self.scene.camera.pitch -= sensitivity * dt_sec;
        }
        self.scene.camera.pitch = f32::clamp(
            self.scene.camera.pitch,
            -std::f32::consts::PI * 0.5,
            std::f32::consts::PI * 0.5,
        );

        if self.is_key_pressed(KeyCode::Minus) {
            self.scene.camera.yfov += 0.5 * dt_sec
        }
        if self.is_key_pressed(KeyCode::Equal) {
            self.scene.camera.yfov -= 0.5 * dt_sec
        }
        self.scene.camera.yfov = f32::clamp(self.scene.camera.yfov, 0.01, std::f32::consts::PI);
    }

    fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        let output = self.surface.get_current_texture()?;
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut command_encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());

        self.renderer
            .write_vertex(&self.device, &self.queue, &self.scene);
        self.renderer
            .render(&mut command_encoder, &view, &self.queue, &self.scene);

        self.queue.submit(std::iter::once(command_encoder.finish()));

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

        self.renderer.resize(&self.device, width, height);

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
            let window_size = self.window.inner_size();
            self.window
                .set_cursor_grab(winit::window::CursorGrabMode::None)
                .unwrap();
            self.window
                .set_cursor_position(winit::dpi::PhysicalPosition::new(
                    window_size.width / 2,
                    window_size.height / 2,
                ))
                .unwrap();
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
        let icon = winit::window::Icon::from_rgba(include_bytes!("icon").into(), 32, 32).unwrap();

        let window = event_loop
            .create_window(
                winit::window::WindowAttributes::default()
                    .with_title("Physically based rendering")
                    .with_window_icon(Some(icon)),
            )
            .unwrap();

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
            WindowEvent::Focused(focus) => {
                context.focused = focus;
                context.set_cursor_visible(!focus);
            }
            _ => (),
        }
    }
}

fn main() {
    let event_loop = winit::event_loop::EventLoop::new().unwrap();
    event_loop.run_app(&mut App::default()).unwrap();
}
