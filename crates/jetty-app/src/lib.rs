mod app;

use winit::event_loop::{ControlFlow, EventLoop};

pub fn run() {
    let event_loop = EventLoop::new().expect("event loop");
    event_loop.set_control_flow(ControlFlow::Wait);
    let mut app = app::App::default();
    event_loop.run_app(&mut app).expect("run_app");
}
