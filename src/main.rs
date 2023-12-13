use anyhow::Context;
use cgmath::{Quaternion, Vector3, Zero};
use log::info;
#[cfg(feature = "tracing")]
use log::info;
use specs::{Builder, DispatcherBuilder, Read, ReadStorage, System, World, WorldExt, WriteStorage};
use tracing::{span, Level};
#[cfg(feature = "tracing")]
use tracing_subscriber::layer::SubscriberExt;
use triton::{
    app::App,
    game::{Position, Transform, TransformSystem, Velocity},
};

struct HelloWorld;

impl<'a> System<'a> for HelloWorld {
    type SystemData = (Read<'a, Phase>, ReadStorage<'a, Position>);

    fn run(&mut self, data: Self::SystemData) {
        use specs::Join;
        let (phase, position) = data;

        for position in position.join() {
            info!("Hello, {:?} - {:?}", phase.0, &position);
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

#[derive(Debug)]
enum UpdatePhase {
    PreUpdate,
    Update,
    PostUpate,
}

impl Default for UpdatePhase {
    fn default() -> Self {
        UpdatePhase::Update
    }
}

#[derive(Default)]
struct Phase(UpdatePhase);

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

    let mut dispatcher = DispatcherBuilder::new()
        .with(HelloWorld, "hello_world", &[])
        .with(UpdatePos, "update_pos", &["hello_world"])
        .with(HelloWorld, "hello_updated", &["update_pos"])
        .build();

    dispatcher.setup(&mut world);

    let mut fixed_update_dispatcher = DispatcherBuilder::new()
        .with(TransformSystem, "transform_system", &[])
        .build();

    fixed_update_dispatcher.setup(&mut world);

    world
        .create_entity()
        .with(Transform {
            position: Vector3::zero(),
            rotation: Quaternion::new(1.0, 0.0, 0.0, 0.0),
            scale: Vector3::new(1.0, 1.0, 1.0),
        })
        .build();

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

    world.insert(Phase(UpdatePhase::PreUpdate));

    dispatcher.dispatch(&mut world);

    *world.write_resource::<Phase>() = Phase(UpdatePhase::Update);

    dispatcher.dispatch(&mut world);

    {
        let mut phase = world.write_resource::<Phase>();
        *phase = Phase(UpdatePhase::PostUpate);
    }

    dispatcher.dispatch(&mut world);

    world.maintain();

    Ok(())

    // let app = App::new()?;

    // app.run()
}
