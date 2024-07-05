pub mod object;

pub use wgpu;
pub use winit;

use std::{error::Error, sync::Arc};

use winit::{
    event::{Event, KeyEvent, WindowEvent},
    event_loop::{EventLoop, EventLoopWindowTarget},
    keyboard::{Key, NamedKey},
    window::Window,
};

pub trait Renderable {
    fn render<'a>(&'a mut self, rpass: &mut wgpu::RenderPass<'a>, queue: &wgpu::Queue);
    fn resize(&mut self, width: u32, height: u32, queue: &wgpu::Queue);
}

pub struct Graphics {
    pub window: Arc<Window>,
    pub instance: wgpu::Instance,
    pub config: wgpu::SurfaceConfiguration,
    pub surface: wgpu::Surface<'static>,
    pub adapter: wgpu::Adapter,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub renderables: Vec<Box<dyn Renderable>>,
}

impl Graphics {
    pub async fn new(window: Arc<Window>) -> Result<Self, Box<dyn Error>> {
        let size = window.inner_size();
        let instance = wgpu::Instance::default();
        let surface = instance.create_surface(window.clone())?;
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                force_fallback_adapter: false,
                compatible_surface: Some(&surface),
            })
            .await
            .expect("Failed to find an appropriate adapter");

        let optional_features = wgpu::Features::POLYGON_MODE_LINE;
        let required_features = wgpu::Features::empty();
        let trace_dir = std::env::var("WGPU_TRACE");
        let needed_limits =
            wgpu::Limits::downlevel_webgl2_defaults().using_resolution(adapter.limits());

        let adapter_features = adapter.features();

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: None,
                    required_features: (optional_features & adapter_features) | required_features,
                    required_limits: needed_limits,
                },
                trace_dir.ok().as_ref().map(std::path::Path::new),
            )
            .await
            .expect("Unable to find a suitable GPU adapter!");

        let swapchain_capabilities = surface.get_capabilities(&adapter);
        let swapchain_format = swapchain_capabilities.formats[0];

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: swapchain_format,
            width: size.width,
            height: size.height,
            desired_maximum_frame_latency: 2,
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: swapchain_capabilities.alpha_modes[0],
            view_formats: vec![swapchain_format],
        };

        surface.configure(&device, &config);

        Ok(Self {
            window,
            instance,
            config,
            surface,
            adapter,
            device,
            queue,
            renderables: vec![],
        })
    }

    pub async fn run(mut self, event_loop: EventLoop<()>) {
        event_loop
            .run(|event: Event<()>, target: &EventLoopWindowTarget<()>| {
                let _ = (&self.instance, &self.adapter);

                match event {
                    Event::WindowEvent {
                        event: WindowEvent::Resized(size),
                        ..
                    } => {
                        self.config.width = size.width;
                        self.config.height = size.height;
                        self.surface.configure(&self.device, &self.config);
                        self.window.request_redraw();
                    }

                    Event::WindowEvent { event, .. } => match event {
                        WindowEvent::Resized(size) => {
                            self.config.width = size.width.max(1);
                            self.config.height = size.height.max(1);
                            self.surface.configure(&self.device, &self.config);

                            for renderable in self.renderables.iter_mut() {
                                renderable.resize(size.width, size.height, &self.queue);
                            }

                            self.window.request_redraw();
                        }
                        WindowEvent::KeyboardInput {
                            event:
                                KeyEvent {
                                    logical_key: Key::Named(NamedKey::Escape),
                                    ..
                                },
                            ..
                        }
                        | WindowEvent::CloseRequested => {
                            target.exit();
                        }

                        WindowEvent::RedrawRequested => {
                            let frame = self
                                .surface
                                .get_current_texture()
                                .expect("Failed to acquire next swap chain texture");
                            let view = frame
                                .texture
                                .create_view(&wgpu::TextureViewDescriptor::default());
                            let mut encoder = self.device.create_command_encoder(
                                &wgpu::CommandEncoderDescriptor { label: None },
                            );
                            {
                                let mut rpass =
                                    encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                                        label: None,
                                        color_attachments: &[Some(
                                            wgpu::RenderPassColorAttachment {
                                                view: &view,
                                                resolve_target: None,
                                                ops: wgpu::Operations {
                                                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                                                    store: wgpu::StoreOp::Store,
                                                },
                                            },
                                        )],
                                        depth_stencil_attachment: None,
                                        timestamp_writes: None,
                                        occlusion_query_set: None,
                                    });
                                rpass.push_debug_group("Prepare data for draw.");
                                for renderable in self.renderables.iter_mut() {
                                    renderable.render(&mut rpass, &self.queue);
                                }
                            }

                            self.queue.submit(Some(encoder.finish()));
                            frame.present();
                            self.window.request_redraw();
                        }
                        _ => {}
                    },
                    _ => {}
                }
            })
            .expect("");
    }
}
