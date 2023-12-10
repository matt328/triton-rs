use std::sync::Arc;

use anyhow::Context;

use tracing::{span, Level};
use triton::renderer::Renderer;

use vulkano::swapchain::Surface;
use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop, EventLoopWindowTarget},
    window::WindowBuilder,
};

fn main() -> anyhow::Result<()> {
    log4rs::init_file("log4rs.yml", Default::default()).context("Could not configure logger")?;

    #[cfg(feature = "tracing")]
    info!("Tracing enabled");

    #[cfg(feature = "tracing")]
    #[global_allocator]
    static GLOBAL: ProfiledAllocator<std::alloc::System> =
        ProfiledAllocator::new(std::alloc::System, 100);

    #[cfg(feature = "tracing")]
    tracing::subscriber::set_global_default(
        tracing_subscriber::registry().with(tracing_tracy::TracyLayer::new()),
    )
    .expect("set up the subscriber");

    let _root = span!(Level::INFO, "root").entered();

    let event_loop = EventLoop::new().context("Creating Event Loop")?;

    let required_extensions =
        Surface::required_extensions(&event_loop).context("querying required extensions")?;

    let window = Arc::new(
        WindowBuilder::new()
            .build(&event_loop)
            .context("Creating Window")?,
    );

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
                    renderer.resized(new_size);
                }
                Event::WindowEvent {
                    event: WindowEvent::RedrawRequested,
                    ..
                } => match renderer.update() {
                    Ok(()) => (),
                    Err(e) => panic!("Renderer Update Error {e}"),
                },
                _ => (),
            }
        })
        .context("Executing Event Loop")
}
