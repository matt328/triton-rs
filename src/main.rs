use anyhow::Context;
use triton::Renderer;
use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
};

#[cfg(feature = "tracing")]
use tracing::{span, Level};
#[cfg(feature = "tracing")]
use tracing_subscriber::layer::SubscriberExt;

pub fn main() -> anyhow::Result<()> {
    log4rs::init_file("log4rs.yml", Default::default()).context("Could not configure logger")?;

    #[cfg(feature = "tracing")]
    log::info!("Tracing enabled");

    #[cfg(feature = "tracing")]
    #[global_allocator]
    static GLOBAL: tracy_client::ProfiledAllocator<std::alloc::System> =
        tracy_client::ProfiledAllocator::new(std::alloc::System, 100);

    #[cfg(feature = "tracing")]
    tracing::subscriber::set_global_default(
        tracing_subscriber::registry().with(tracing_tracy::TracyLayer::new()),
    )
    .expect("setting up tracing");

    #[cfg(feature = "tracing")]
    let _root = span!(Level::INFO, "root").entered();

    let event_loop = EventLoop::new();

    let mut triton_renderer = Renderer::new(&event_loop)?;

    log::info!("Constructed Renderer");

    event_loop.run(move |event, _, control_flow| match event {
        Event::WindowEvent { event, window_id }
            if window_id == triton_renderer.window_id().unwrap() =>
        {
            match event {
                WindowEvent::Resized(_) => {
                    if let Err(e) = triton_renderer
                        .resize()
                        .context("handling WindowEvent::Resized")
                    {
                        log::warn!("{}", e);
                    }
                }
                WindowEvent::ScaleFactorChanged { .. } => {
                    if let Err(e) = triton_renderer
                        .resize()
                        .context("handling WindowEvent::ScaleFactorChanged")
                    {
                        log::warn!("{}", e);
                    }
                }
                WindowEvent::CloseRequested => {
                    *control_flow = ControlFlow::Exit;
                }
                _ => (),
            }
        }

        Event::WindowEvent {
            event: WindowEvent::CloseRequested,
            window_id,
            ..
        } => {
            if let Some(current_window_id) = triton_renderer.window_id() {
                if window_id == current_window_id {
                    *control_flow = ControlFlow::Exit;
                }
            }
        }

        Event::RedrawRequested(window_id) => {
            #[cfg(feature = "tracing")]
            let _span = span!(Level::INFO, "RedrawRequested").entered();
            triton_renderer
                .window_id()
                .and_then(|current_window_id| {
                    if window_id == current_window_id {
                        Some(())
                    } else {
                        None
                    }
                })
                .and_then(|_| triton_renderer.window_size())
                .map(|image_extent| {
                    let image_extent_arr: [u32; 2] = image_extent.into();
                    if image_extent_arr.contains(&0) {
                        return;
                    }
                    if let Err(e) = triton_renderer.render().context("rendering") {
                        log::error!("{}", e);
                    }
                });
        }

        Event::MainEventsCleared => {
            if let Err(e) = triton_renderer
                .request_redraw()
                .context("requesting redraw")
            {
                log::warn!("{}", e);
            }
        }

        _ => (),
    })
}
