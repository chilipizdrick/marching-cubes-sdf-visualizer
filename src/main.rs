mod app;

use env_logger::Env;
use winit::event_loop::EventLoop;

use crate::app::App;

fn main() {
    let env = Env::default().filter_or("RUST_LOG", "info");
    env_logger::init_from_env(env);

    let event_loop = EventLoop::new().unwrap();

    let mut app = App::new();

    event_loop.run_app(&mut app).unwrap();
}
