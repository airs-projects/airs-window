use std::{
    collections::HashMap,
    time::{Duration, Instant},
};

use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    window::WindowId,
};

use crate::error::Error;
use crate::wgpu_window::{WgpuWindow, WgpuWindowHandler};

pub struct WindowLoop;

impl WindowLoop {
    pub fn new() -> Self {
        Self
    }

    pub fn run<H>(self, handler: H) -> std::result::Result<(), Error>
    where
        H: WgpuWindowHandler + 'static,
    {
        let event_loop = EventLoop::new()?;
        event_loop.set_control_flow(ControlFlow::Wait);

        let mut runner = WindowLoopRunner {
            handler: Some(handler),
            error: None,
            frame_interval: None,
            next_frame: None,
            windows: HashMap::new(),
        };
        event_loop.run_app(&mut runner)?;

        if let Some(error) = runner.error {
            return Err(error);
        }
        Ok(())
    }
}

impl Default for WindowLoop {
    fn default() -> Self {
        Self::new()
    }
}

struct WindowLoopRunner<H> {
    handler: Option<H>,
    error: Option<Error>,
    frame_interval: Option<Duration>,
    next_frame: Option<Instant>,
    windows: HashMap<WindowId, Box<dyn WindowEventHandler>>,
}

impl<H> WindowLoopRunner<H>
where
    H: WgpuWindowHandler + 'static,
{
    fn create_window(&mut self, event_loop: &ActiveEventLoop) -> std::result::Result<(), Error> {
        let mut handler = self
            .handler
            .take()
            .expect("window handler must exist before initialization");
        let attributes = handler.window_attributes();
        let max_frame_rate = handler.max_frame_rate();

        let window = event_loop.create_window(attributes)?;
        let wgpu_window = WgpuWindow::new(window, max_frame_rate)?;
        handler
            .init(&wgpu_window)
            .map_err(|error| Error::User(error.into()))?;
        let wgpu_window = wgpu_window.with_handler(handler);

        let frame_interval = Duration::from_secs_f64(1.0 / wgpu_window.max_frame_rate() as f64);
        self.frame_interval = Some(frame_interval);
        self.next_frame = Some(Instant::now() + frame_interval);
        self.windows.insert(wgpu_window.id(), Box::new(wgpu_window));
        Ok(())
    }
}

impl<H> ApplicationHandler for WindowLoopRunner<H>
where
    H: WgpuWindowHandler + 'static,
{
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.handler.is_none() {
            return;
        }

        if let Err(error) = self.create_window(event_loop) {
            self.error = Some(error);
            event_loop.exit();
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        if let Some(window) = self.windows.get_mut(&window_id) {
            window.window_event(event_loop, event);
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        let (Some(frame_interval), Some(next_frame)) =
            (self.frame_interval, self.next_frame.as_mut())
        else {
            event_loop.set_control_flow(ControlFlow::Wait);
            return;
        };

        let now = Instant::now();
        if now >= *next_frame {
            for window in self.windows.values_mut() {
                window.update();
            }
            *next_frame = advance_frame_deadline(*next_frame, now, frame_interval);
        }
        event_loop.set_control_flow(ControlFlow::WaitUntil(*next_frame));
    }
}

fn advance_frame_deadline(
    mut deadline: Instant,
    now: Instant,
    frame_interval: Duration,
) -> Instant {
    while deadline <= now {
        deadline += frame_interval;
    }
    deadline
}

pub(crate) trait WindowEventHandler {
    fn update(&mut self);

    fn window_event(&mut self, event_loop: &ActiveEventLoop, event: WindowEvent);
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use super::advance_frame_deadline;

    #[test]
    fn frame_deadline_skips_elapsed_frames_without_drifting() {
        let started_at = Instant::now();
        let frame_interval = Duration::from_nanos(16_666_667);
        let first_deadline = started_at + frame_interval;
        let now = first_deadline + frame_interval + frame_interval / 2;

        let next_deadline = advance_frame_deadline(first_deadline, now, frame_interval);

        assert_eq!(next_deadline, started_at + frame_interval * 3);
        assert!(next_deadline > now);
    }
}
