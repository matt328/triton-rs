use anyhow::Context;
use triton::GameLoop;
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

    let event_loop = EventLoop::new()?;
    event_loop.set_control_flow(ControlFlow::Poll);

    let mut game_loop = GameLoop::new(&event_loop).context("creating game loop")?;

    log::info!("Constructed Game Loop");

    event_loop
        .run(move |event, elwt| {
            game_loop.process_winit_event(&event, false);

            match event {
                Event::WindowEvent { event, window_id }
                    if window_id == game_loop.window_id().unwrap() =>
                {
                    match event {
                        WindowEvent::Resized(_) => {
                            if let Err(e) =
                                game_loop.resize().context("handling WindowEvent::Resized")
                            {
                                log::warn!("{}", e);
                            }
                        }
                        WindowEvent::ScaleFactorChanged { .. } => {
                            if let Err(e) = game_loop
                                .resize()
                                .context("handling WindowEvent::ScaleFactorChanged")
                            {
                                log::warn!("{}", e);
                            }
                        }
                        WindowEvent::CloseRequested => {
                            elwt.exit();
                        }
                        _ => (),
                    }
                }

                Event::AboutToWait => {
                    #[cfg(feature = "tracing")]
                    let _span = span!(Level::INFO, "Event::AboutToWait").entered();
                    game_loop.window_size().map(|image_extent| {
                        let image_extent_arr: [u32; 2] = image_extent.into();
                        if image_extent_arr.contains(&0) {
                            return;
                        }
                        if let Err(e) = game_loop.update().context("rendering") {
                            log::error!("{}", e);
                        }
                    });
                }
                _ => (),
            }
        })
        .context("event loop")
}
