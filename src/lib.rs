mod error;
mod wgpu_window;
mod window_loop;

pub use error::Error;
pub use wgpu;
pub use wgpu_window::{WgpuContext, WgpuWindow, WgpuWindowHandler};
pub use window_loop::WindowLoop;

pub type WindowEvent = winit::event::WindowEvent;
pub type WindowId = winit::window::WindowId;

pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_matches_package_version() {
        assert_eq!(version(), env!("CARGO_PKG_VERSION"));
    }
}
