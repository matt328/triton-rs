use anyhow::Context;
use triton::GameLoop;
use winit::{
    event::{ElementState, Event, MouseButton, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    keyboard::{KeyCode, PhysicalKey},
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

    let mut mouse_captured = false;

    event_loop
        .run(move |event, elwt| {
            game_loop.process_winit_event(&event, mouse_captured);

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
                        WindowEvent::MouseInput { state, button, .. } => match (state, button) {
                            (ElementState::Released, MouseButton::Left) => {
                                log::info!("Capturing Mouse");
                                game_loop.set_cursor_captured();
                                mouse_captured = true;
                            }
                            (ElementState::Released, MouseButton::Right) => {
                                game_loop.set_cursor_released();
                                mouse_captured = false;
                            }
                            _ => (),
                        },
                        WindowEvent::KeyboardInput {
                            device_id: _,
                            event,
                            is_synthetic: _,
                        } => {
                            if event.physical_key == PhysicalKey::Code(KeyCode::Escape) {
                                game_loop.set_cursor_released();
                                mouse_captured = false;
                            }
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
