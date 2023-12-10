use std::sync::Arc;

use anyhow::Context;
use triton::graphics::Renderer;

use vulkano::swapchain::Surface;
use winit::event::{Event, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop, EventLoopWindowTarget};
use winit::window::WindowBuilder;

fn main() -> anyhow::Result<()> {
    let event_loop = EventLoop::new().unwrap();
    let required_extensions = Surface::required_extensions(&event_loop);

    let window = Arc::new(WindowBuilder::new().build(&event_loop).unwrap());

    let mut renderer = Renderer::new(required_extensions, window.clone())?;

    event_loop
        .run(move |event, elwt: &EventLoopWindowTarget<()>| {
            elwt.set_control_flow(ControlFlow::Poll);
            match event {
                Event::WindowEvent {
                    event: WindowEvent::CloseRequested,
                    ..
                } => {
                    elwt.exit();
                }
                Event::WindowEvent {
                    event: WindowEvent::Resized(new_size),
                    ..
                } => {
                    renderer.window_resized(new_size);
                }
                Event::WindowEvent {
                    event: WindowEvent::RedrawRequested,
                    ..
                } => {
                    let _ = renderer.draw();
                }
                _ => (),
            }
        })
        .context("Processing EventLoop")
}
