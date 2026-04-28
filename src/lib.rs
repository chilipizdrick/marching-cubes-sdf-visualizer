mod app;

use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::ActiveEventLoop,
    window::{WindowAttributes, WindowId},
};

use crate::app::AppState;

#[derive(Default)]
pub struct App {
    state: Option<AppState<'static>>,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window_attributes = WindowAttributes::default();
        let window = event_loop.create_window(window_attributes).unwrap();

        let state = AppState::new(window).unwrap();
        self.state = Some(state);
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, id: WindowId, event: WindowEvent) {
        if let Some(state) = &mut self.state {
            state.window_event(event_loop, id, event);
        }
    }

    fn device_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _device_id: winit::event::DeviceId,
        event: winit::event::DeviceEvent,
    ) {
        if let Some(state) = &mut self.state {
            state.device_event(event_loop, event);
        }
    }
}

impl App {
    pub fn new() -> Self {
        Self { state: None }
    }
}
