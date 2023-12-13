use std::sync::Arc;

use cgmath::{Quaternion, Vector3, Zero};
use specs::{Builder, Dispatcher, DispatcherBuilder, World, WorldExt};
use vulkano::instance::InstanceExtensions;
use winit::{dpi::PhysicalSize, window::Window};

use crate::graphics::{Camera, Renderer};

use super::{
    camera::MouseLookCamera,
    state::{blend_state, next_state},
    State, Transform, TransformSystem,
};

pub struct Context<'a, 'b> {
    renderer: Renderer,
    camera: Arc<Box<dyn Camera>>,
    world: World,
    state: State,
    previous_state: State,
    fixed_update_dispatcher: Dispatcher<'a, 'b>,
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

        let renderer = Renderer::new(required_extensions, window.clone(), camera.clone())?;
        let state = State::default();

        let mut world = World::new();

        world
            .create_entity()
            .with(Transform {
                position: Vector3::zero(),
                rotation: Quaternion::new(1.0, 0.0, 0.0, 0.0),
                scale: Vector3::new(1.0, 1.0, 1.0),
            })
            .build();

        let mut fixed_update_dispatcher = DispatcherBuilder::new().build();

        fixed_update_dispatcher.setup(&mut world);

        Ok(Context {
            camera,
            renderer,
            state,
            previous_state: State::default(),
            world,
            fixed_update_dispatcher,
        })
    }

    pub fn pre_update(&mut self) {
        self.previous_state = self.state;
    }

    pub fn update(&mut self) {
        self.previous_state = self.state;
        self.state = next_state(&self.state);
    }

    pub fn post_update(&mut self, blending_factor: f32) {
        self.state = blend_state(&self.previous_state, &self.state, blending_factor);
    }

    pub fn render(&mut self) -> anyhow::Result<()> {
        self.renderer.draw(self.state)
    }

    pub fn window_resized(&mut self, new_size: PhysicalSize<u32>) {
        self.renderer.window_resized(new_size);
    }
}
