use anyhow::Context;
#[cfg(feature = "tracing")]
use log::info;
use specs::{
    Builder, DispatcherBuilder, ReadStorage, RunNow, System, World, WorldExt, WriteStorage,
};
use tracing::{span, Level};
#[cfg(feature = "tracing")]
use tracing_subscriber::layer::SubscriberExt;
use triton::{
    app::App,
    game::{Position, Velocity},
};

/*
    TODO:
    create a GameContext struct that owns and ties game stuff together with the renderer
    for now GameContext should own a first person camera that it will activate() within the
    renderer so that the renderer can get the view and projection matrices.

    After that, look into an ecs for rust, and move the cube and its rotating behavior into
    that somehow.
*/

struct HelloWorld;

impl<'a> System<'a> for HelloWorld {
    type SystemData = ReadStorage<'a, Position>;

    fn run(&mut self, position: Self::SystemData) {
        use specs::Join;

        for position in position.join() {
            println!("Hello, {:?}", &position);
        }
    }
}

struct UpdatePos;

impl<'a> System<'a> for UpdatePos {
    type SystemData = (ReadStorage<'a, Velocity>, WriteStorage<'a, Position>);

    fn run(&mut self, (vel, mut pos): Self::SystemData) {
        use specs::Join;
        for (vel, pos) in (&vel, &mut pos).join() {
            pos.x += vel.x * 0.05;
            pos.y += vel.y * 0.05;
        }
    }
}

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

    let _root = span!(Level::INFO, "root").entered();

    let mut world = World::new();
    world.register::<Position>();
    world.register::<Velocity>();

    world
        .create_entity()
        .with(Position {
            x: 4.0,
            y: 7.0,
            z: 0.0,
        })
        .build();

    world
        .create_entity()
        .with(Position {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        })
        .with(Velocity { x: 0.1, y: 0.1 })
        .build();

    let mut dispatcher = DispatcherBuilder::new()
        .with(HelloWorld, "hello_world", &[])
        .with(UpdatePos, "update_pos", &["hello_world"])
        .with(HelloWorld, "hello_updated", &["update_pos"])
        .build();

    dispatcher.dispatch(&mut world);
    world.maintain();

    Ok(())

    // let app = App::new()?;

    // app.run()
}
