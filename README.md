# airs-window

Window and wgpu context layer for AIRS.

## Library public API

- `version()` — package version
- `Error` — error type
- `WindowLoop` — event loop entry point
- `ActiveWindowLoop` — active loop, creates windows
- `WgpuContext` — GPU surface / device / queue
- `WgpuWindow<H>` — window with handler
- `WgpuWindowHandler` — handler trait (resize, event, render)
- `WindowEvent` — type alias for winit window events
- `WindowId` — type alias for winit window id
- `wgpu` — re-export of wgpu crate
