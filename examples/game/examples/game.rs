use std::sync::Arc;

use rl_graphics::winit::event_loop::EventLoop;
use rl_graphics::Graphics;

use game::Cube;

pub fn main() {
    env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .filter_module("wgpu_core", log::LevelFilter::Info)
        .filter_module("wgpu_hal", log::LevelFilter::Error)
        .filter_module("naga", log::LevelFilter::Error)
        .parse_default_env()
        .init();

    let event_loop = EventLoop::new().unwrap();
    let builder = rl_graphics::winit::window::WindowBuilder::new();
    let window = Arc::new(builder.build(&event_loop).unwrap());
    let mut graphics = pollster::block_on(Graphics::new(window)).unwrap();

    let cube = Cube::new(&graphics);

    graphics.renderables.push(Box::new(cube));

    pollster::block_on(graphics.run(event_loop));
}
