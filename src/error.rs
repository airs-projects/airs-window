#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("failed to create window: {0}")]
    CreateWindow(#[from] winit::error::OsError),
    #[error("failed to create event loop: {0}")]
    EventLoop(#[from] winit::error::EventLoopError),
    #[error("failed to create wgpu surface: {0}")]
    CreateSurface(#[from] wgpu::CreateSurfaceError),
    #[error("failed to request wgpu adapter: {0}")]
    RequestAdapter(#[from] wgpu::RequestAdapterError),
    #[error("wgpu adapter is incompatible with the surface")]
    IncompatibleAdapter,
    #[error("failed to request wgpu device: {0}")]
    RequestDevice(#[from] wgpu::RequestDeviceError),
    #[error("wgpu surface lost")]
    SurfaceLost,
    #[error("wgpu surface validation failed")]
    SurfaceValidation,
    #[error("window max frame rate must be greater than zero")]
    InvalidFrameRate,
    #[error("user callback error: {0}")]
    User(#[source] Box<dyn std::error::Error + Send + Sync>),
    #[error("{0}")]
    Other(Box<dyn std::error::Error + Send + Sync>),
}
