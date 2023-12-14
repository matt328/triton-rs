use std::sync::Arc;

use cgmath::{Quaternion, Vector3, Zero};
use log::info;
use specs::{Builder, Dispatcher, DispatcherBuilder, World, WorldExt};
use tracing::{span, Level};
use vulkano::instance::InstanceExtensions;
use winit::{dpi::PhysicalSize, window::Window};

use crate::graphics::{Camera, Renderer, CUBE_INDICES, CUBE_VERTICES};

use super::{
    camera::MouseLookCamera,
    components::{BlendFactor, RenderSystem, Renderable, ResizeEvents},
    state::next_state,
    State, Transform, TransformSystem,
};

pub struct Context<'a, 'b> {
    camera: Arc<Box<dyn Camera>>,
    world: World,
    state: State,
    previous_state: State,
    fixed_update_dispatcher: Dispatcher<'a, 'b>,
    render_dispatcher: Dispatcher<'a, 'b>,
}

impl<'a, 'b> Context<'a, 'b> {
    pub fn new(
        required_extensions: InstanceExtensions,
        window: Arc<Window>,
    ) -> anyhow::Result<Self> {
        let extent: [f32; 2] = window.inner_size().into();

        let camera: Arc<Box<dyn Camera>> = Arc::new(Box::new(MouseLookCamera::new_with_aspect(
            extent[0] / extent[1],
        )));

        let mut renderer = Renderer::new(required_extensions, window.clone(), camera.clone())?;

        let state = State::default();

        let mut world = World::new();

        world.insert(ResizeEvents(Vec::new()));

        let mesh_id = renderer.create_mesh(CUBE_VERTICES.into(), CUBE_INDICES.into())?;

        let mut fixed_update_dispatcher = DispatcherBuilder::new()
            .with(TransformSystem, "transform_system", &[])
            .build();

        let mut render_dispatcher = DispatcherBuilder::new()
            .with_thread_local(RenderSystem::new(renderer))
            .build();

        fixed_update_dispatcher.setup(&mut world);
        render_dispatcher.setup(&mut world);

        world
            .create_entity()
            .with(Transform {
                position: Vector3::zero(),
                rotation: Quaternion::new(1.0, 0.0, 0.0, 0.0),
                scale: Vector3::new(1.0, 1.0, 1.0),
            })
            .with(Renderable { mesh_id })
            .build();

        Ok(Context {
            camera,
            state,
            previous_state: State::default(),
            world,
            fixed_update_dispatcher,
            render_dispatcher,
        })
    }

    pub fn update(&mut self) {
        let _span = span!(Level::INFO, "fixed_update").entered();
        self.fixed_update_dispatcher.dispatch(&self.world);
        self.previous_state = self.state;
        self.state = next_state(&self.state);
    }

    pub fn render(&mut self, blending_factor: f32) -> anyhow::Result<()> {
        let _span = span!(Level::INFO, "render").entered();
        self.world.insert(BlendFactor(blending_factor));
        self.render_dispatcher.dispatch(&self.world);
        Ok(())
    }

    pub fn window_resized(&mut self, new_size: PhysicalSize<u32>) {
        self.world.write_resource::<ResizeEvents>().0.push(new_size);
    }
}
