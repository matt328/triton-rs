use anyhow::Context;

#[cfg(feature = "tracing")]
use log::info;
#[cfg(feature = "tracing")]
use tracing::{span, Level};
#[cfg(feature = "tracing")]
use tracing_subscriber::layer::SubscriberExt;
use triton::app::App;

fn main() -> anyhow::Result<()> {
    log4rs::init_file("log4rs.yml", Default::default()).context("Could not configure logger")?;

    #[cfg(feature = "tracing")]
    info!("Tracing enabled");

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

    let app = App::new()?;

    app.run()
}
