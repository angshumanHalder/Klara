use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{self, EventLoop},
    window::Window,
};

use klara::{
    config::{Config, ConfigError},
    error::KlaraError,
    input::{Action, InputHandler},
    layout, renderer,
    window::WindowManager,
};

struct App {
    config: Config,
    window: Option<std::sync::Arc<Window>>,
    surface_state: Option<SurfaceState>,
    input: InputHandler,
    wm: Option<WindowManager>,
    renderer: Option<renderer::Renderer>,
}

struct SurfaceState {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
}

impl App {
    fn new(config: Config) -> Self {
        Self {
            config,
            window: None,
            surface_state: None,
            input: InputHandler::new(),
            wm: None,
            renderer: None,
        }
    }

    fn render(&mut self) {
        let Some(state) = self.surface_state.as_mut() else {
            return;
        };
        let Ok(frame) = state.surface.get_current_texture() else {
            return;
        };
        let view = frame.texture.create_view(&Default::default());
        let mut encoder = state.device.create_command_encoder(&Default::default());
        let bg = self.config.parse_color(&self.config.theme.background);
        {
            let _pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: bg[0],
                            g: bg[1],
                            b: bg[2],
                            a: bg[3],
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                ..Default::default()
            });
        }
        if let (Some(renderer), Some(wm)) = (self.renderer.as_mut(), self.wm.as_ref()) {
            let layouts = wm.pane_layouts();
            renderer.draw(&state.device, &state.queue, &mut encoder, &view, &layouts);
        }
        state.queue.submit([encoder.finish()]);
        frame.present();
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        let window = std::sync::Arc::new(
            event_loop
                .create_window(
                    Window::default_attributes()
                        .with_title("Klara")
                        .with_inner_size(winit::dpi::LogicalSize::new(
                            self.config.window.width,
                            self.config.window.height,
                        )),
                )
                .unwrap(),
        );
        self.window = Some(window.clone());

        let instance = wgpu::Instance::default();
        let surface = instance.create_surface(window.clone()).unwrap();
        let size = self.window.as_ref().unwrap().inner_size();
        self.wm = Some(
            WindowManager::new(size.width as f32, size.height as f32, Some(window.clone()))
                .unwrap(),
        );

        let (adapter, device, queue) = pollster::block_on(async {
            let adapter = instance
                .request_adapter(&wgpu::RequestAdapterOptions {
                    compatible_surface: Some(&surface),
                    ..Default::default()
                })
                .await
                .unwrap();
            let (device, queue) = adapter
                .request_device(&Default::default(), None)
                .await
                .unwrap();
            (adapter, device, queue)
        });

        let size = self.window.as_ref().unwrap().inner_size();
        let caps = surface.get_capabilities(&adapter);
        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: caps.formats[0],
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &surface_config);
        self.surface_state = Some(SurfaceState {
            surface,
            device,
            queue,
            config: surface_config,
        });

        self.renderer = Some(renderer::Renderer::new(
            &self.surface_state.as_ref().unwrap().device,
            &self.surface_state.as_ref().unwrap().queue,
            self.surface_state.as_ref().unwrap().config.format,
            size.width,
            size.height,
        ));

        self.window.as_ref().unwrap().request_redraw();
    }

    fn window_event(
        &mut self,
        event_loop: &event_loop::ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => {
                if let Some(state) = self.surface_state.as_mut() {
                    state.config.width = size.width;
                    state.config.height = size.height;
                    state.surface.configure(&state.device, &state.config);
                    if let Some(r) = self.renderer.as_mut() {
                        r.resize(&state.queue, size.width, size.height);
                    }
                }
                if let Some(wm) = self.wm.as_mut() {
                    wm.resize(size.width as f32, size.height as f32);
                }
            }
            WindowEvent::RedrawRequested => self.render(),
            WindowEvent::ModifiersChanged(mods) => self.input.modifiers = mods,
            WindowEvent::KeyboardInput { event, .. } => {
                if let Some(wm) = self.wm.as_mut() {
                    let app_cursor = wm
                        .active
                        .lock()
                        .unwrap()
                        .grid
                        .lock()
                        .unwrap()
                        .application_cursor;
                    match self.input.handle(&event, app_cursor) {
                        Action::SendBytes(bytes) => wm.active.lock().unwrap().write_input(&bytes),
                        Action::SplitVerticle => {
                            wm.split_pane(layout::SplitDirection::Vertical).unwrap()
                        }
                        Action::SplitHorizontal => {
                            wm.split_pane(layout::SplitDirection::Horizontal).unwrap()
                        }
                        Action::None => {}
                    }
                }
            }
            _ => {}
        }
    }
}

fn main() -> Result<(), KlaraError> {
    env_logger::init();
    let config = match Config::load("config.toml") {
        Ok(config) => config,
        Err(ConfigError::Read { source, .. }) if source.kind() == std::io::ErrorKind::NotFound => {
            Config::default()
        }
        Err(error) => return Err(error.into()),
    };
    let event_loop = EventLoop::new()?;
    let mut app = App::new(config);
    event_loop.run_app(&mut app)?;
    Ok(())
}
