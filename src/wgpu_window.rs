use std::{ops::Deref, sync::Arc};

use winit::{
    event::WindowEvent,
    event_loop::ActiveEventLoop,
    window::{CursorIcon, Window, WindowAttributes},
};

use crate::error::Error;
use crate::window_loop::WindowEventHandler;

pub trait WgpuWindowHandler {
    fn window_attributes(&self) -> WindowAttributes;

    fn max_frame_rate(&self) -> u32;

    fn init(&mut self, _wgpu_window: &WgpuWindow) -> anyhow::Result<()> {
        Ok(())
    }

    fn update(&mut self) -> bool {
        false
    }

    fn resize(&mut self, _wgpu_ctx: &WgpuContext<'_>, _width: u32, _height: u32) {}

    fn rescale(&mut self, _wgpu_ctx: &WgpuContext<'_>, _scale_factor: f32) {}

    fn close(&mut self, _wgpu_ctx: &WgpuContext<'_>) {}

    fn event(&mut self, _event: &WindowEvent) {}

    fn cursor_icon(&self) -> CursorIcon {
        CursorIcon::Default
    }

    fn render(
        &mut self,
        _wgpu_ctx: &WgpuContext<'_>,
        _texture_view: &wgpu::TextureView,
        _command_encoder: &mut wgpu::CommandEncoder,
    ) -> anyhow::Result<()> {
        Ok(())
    }
}

/// GPU objects associated with a wgpu surface target.
pub struct WgpuContext<'window> {
    instance: wgpu::Instance,
    surface: wgpu::Surface<'window>,
    adapter: wgpu::Adapter,
    device: wgpu::Device,
    queue: wgpu::Queue,
    surface_config: wgpu::SurfaceConfiguration,
    clear_color: wgpu::Color,
}

impl<'window> WgpuContext<'window> {
    pub fn new(
        target: impl Into<wgpu::SurfaceTarget<'window>>,
        width: u32,
        height: u32,
    ) -> std::result::Result<Self, Error> {
        let instance = create_instance();
        let surface = create_surface(&instance, target)?;
        let adapter = create_adapter(&instance, &surface)?;
        let (device, queue) = create_device(&adapter)?;
        let surface_config = create_surface_config(&surface, &adapter, width, height)?;
        surface.configure(&device, &surface_config);

        Ok(Self {
            instance,
            surface,
            adapter,
            device,
            queue,
            surface_config,
            clear_color: wgpu::Color {
                r: 17.0 / 255.0,
                g: 24.0 / 255.0,
                b: 39.0 / 255.0,
                a: 1.0,
            },
        })
    }

    pub fn instance(&self) -> &wgpu::Instance {
        &self.instance
    }

    pub fn surface(&self) -> &wgpu::Surface<'window> {
        &self.surface
    }

    pub fn adapter(&self) -> &wgpu::Adapter {
        &self.adapter
    }

    pub fn device(&self) -> &wgpu::Device {
        &self.device
    }

    pub fn queue(&self) -> &wgpu::Queue {
        &self.queue
    }

    pub fn surface_config(&self) -> &wgpu::SurfaceConfiguration {
        &self.surface_config
    }

    pub fn clear_color(&self) -> wgpu::Color {
        self.clear_color
    }

    pub fn set_clear_color(&mut self, color: wgpu::Color) {
        self.clear_color = color;
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        if width == 0 || height == 0 {
            return;
        }

        self.surface_config.width = width;
        self.surface_config.height = height;
        self.surface.configure(&self.device, &self.surface_config);
    }

    pub fn render(
        &self,
        render: impl FnOnce(&wgpu::TextureView, &mut wgpu::CommandEncoder) -> anyhow::Result<()>,
    ) -> std::result::Result<(), Error> {
        let surface_texture = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(texture)
            | wgpu::CurrentSurfaceTexture::Suboptimal(texture) => texture,
            wgpu::CurrentSurfaceTexture::Timeout | wgpu::CurrentSurfaceTexture::Occluded => {
                return Ok(());
            }
            wgpu::CurrentSurfaceTexture::Outdated => {
                self.surface.configure(&self.device, &self.surface_config);
                return Ok(());
            }
            wgpu::CurrentSurfaceTexture::Lost => return Err(Error::SurfaceLost),
            wgpu::CurrentSurfaceTexture::Validation => return Err(Error::SurfaceValidation),
        };

        let view = surface_texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("airs-window clear encoder"),
            });
        {
            let _render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("airs-window clear pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(self.clear_color),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
        }
        render(&view, &mut encoder).map_err(|e| Error::User(e.into()))?;
        self.queue.submit([encoder.finish()]);
        self.queue.present(surface_texture);
        Ok(())
    }
}

pub struct WgpuWindow<H = ()> {
    window: Arc<Window>,
    wgpu_context: WgpuContext<'static>,
    max_frame_rate: u32,
    cursor_icon: CursorIcon,
    handler: H,
}

impl WgpuWindow<()> {
    #[tracing::instrument(skip_all)]
    pub fn new(window: Window, max_frame_rate: u32) -> std::result::Result<Self, Error> {
        if max_frame_rate == 0 {
            return Err(Error::InvalidFrameRate);
        }
        let window = Arc::new(window);
        let size = window.inner_size();
        let wgpu_context = WgpuContext::new(window.clone(), size.width, size.height)?;
        Ok(Self {
            window,
            wgpu_context,
            max_frame_rate,
            cursor_icon: CursorIcon::Default,
            handler: (),
        })
    }

    pub fn with_handler<H>(self, handler: H) -> WgpuWindow<H>
    where
        H: WgpuWindowHandler,
    {
        let window = WgpuWindow {
            window: self.window,
            wgpu_context: self.wgpu_context,
            max_frame_rate: self.max_frame_rate,
            cursor_icon: self.cursor_icon,
            handler,
        };
        window.request_redraw();
        window
    }
}

impl<H> WgpuWindow<H> {
    pub fn max_frame_rate(&self) -> u32 {
        self.max_frame_rate
    }

    pub fn wgpu_context(&self) -> &WgpuContext<'static> {
        &self.wgpu_context
    }

    pub fn wgpu_context_mut(&mut self) -> &mut WgpuContext<'static> {
        &mut self.wgpu_context
    }

    pub fn instance(&self) -> &wgpu::Instance {
        self.wgpu_context.instance()
    }

    pub fn surface(&self) -> &wgpu::Surface<'static> {
        self.wgpu_context.surface()
    }

    pub fn adapter(&self) -> &wgpu::Adapter {
        self.wgpu_context.adapter()
    }

    pub fn device(&self) -> &wgpu::Device {
        self.wgpu_context.device()
    }

    pub fn queue(&self) -> &wgpu::Queue {
        self.wgpu_context.queue()
    }

    pub fn surface_config(&self) -> &wgpu::SurfaceConfiguration {
        self.wgpu_context.surface_config()
    }

    pub fn clear_color(&self) -> wgpu::Color {
        self.wgpu_context.clear_color()
    }

    pub fn set_clear_color(&mut self, color: wgpu::Color) {
        self.wgpu_context.set_clear_color(color);
    }
}

impl<H> WgpuWindow<H>
where
    H: WgpuWindowHandler,
{
    fn update(&mut self) {
        let needs_redraw = self.handler.update();
        self.update_cursor();
        if needs_redraw {
            self.request_redraw();
        }
    }

    fn resize(&mut self, width: u32, height: u32) {
        self.wgpu_context.resize(width, height);
        self.handler.resize(&self.wgpu_context, width, height);
    }

    fn render(&mut self) -> std::result::Result<(), Error> {
        let wgpu_ctx = &self.wgpu_context;
        wgpu_ctx.render(|texture_view, command_encoder| {
            self.handler.render(wgpu_ctx, texture_view, command_encoder)
        })
    }

    fn update_cursor(&mut self) {
        let cursor_icon = self.handler.cursor_icon();
        if self.cursor_icon != cursor_icon {
            self.window.set_cursor(cursor_icon);
            self.cursor_icon = cursor_icon;
        }
    }
}

impl<H> WindowEventHandler for WgpuWindow<H>
where
    H: WgpuWindowHandler + 'static,
{
    fn update(&mut self) {
        self.update();
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => {
                self.handler.close(&self.wgpu_context);
                event_loop.exit();
            }
            WindowEvent::Resized(size) => {
                self.resize(size.width, size.height);
                self.request_redraw();
            }
            WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                self.handler
                    .rescale(&self.wgpu_context, scale_factor as f32);
                self.request_redraw();
            }
            WindowEvent::RedrawRequested => {
                if let Err(error) = self.render() {
                    tracing::error!(?error, "wgpu window render failed");
                    event_loop.exit();
                }
            }
            event => {
                self.handler.event(&event);
                self.update_cursor();
            }
        }
    }
}

impl<H> Deref for WgpuWindow<H> {
    type Target = Window;

    fn deref(&self) -> &Self::Target {
        &self.window
    }
}

#[tracing::instrument(skip_all)]
fn create_instance() -> wgpu::Instance {
    let backends = if cfg!(target_os = "windows") {
        wgpu::Backends::DX12
    } else {
        wgpu::Backends::all()
    };

    wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends,
        flags: wgpu::InstanceFlags::empty().with_env(),
        memory_budget_thresholds: wgpu::MemoryBudgetThresholds::default(),
        display: None,
        backend_options: wgpu::BackendOptions::default(),
    })
}

#[tracing::instrument(skip_all)]
fn create_surface<'window>(
    instance: &wgpu::Instance,
    target: impl Into<wgpu::SurfaceTarget<'window>>,
) -> std::result::Result<wgpu::Surface<'window>, Error> {
    Ok(instance.create_surface(target)?)
}

#[tracing::instrument(skip_all)]
fn create_adapter(
    instance: &wgpu::Instance,
    surface: &wgpu::Surface<'_>,
) -> std::result::Result<wgpu::Adapter, Error> {
    Ok(pollster::block_on(instance.request_adapter(
        &wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: Some(surface),
            force_fallback_adapter: false,
            apply_limit_buckets: false,
        },
    ))?)
}

#[tracing::instrument(skip_all)]
fn create_device(
    adapter: &wgpu::Adapter,
) -> std::result::Result<(wgpu::Device, wgpu::Queue), Error> {
    Ok(pollster::block_on(adapter.request_device(
        &wgpu::DeviceDescriptor {
            label: Some("airs-window device"),
            ..Default::default()
        },
    ))?)
}

#[tracing::instrument(skip_all)]
fn create_surface_config(
    surface: &wgpu::Surface<'_>,
    adapter: &wgpu::Adapter,
    width: u32,
    height: u32,
) -> std::result::Result<wgpu::SurfaceConfiguration, Error> {
    surface
        .get_default_config(adapter, width.max(1), height.max(1))
        .ok_or(Error::IncompatibleAdapter)
}
