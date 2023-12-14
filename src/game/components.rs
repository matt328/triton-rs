use std::sync::Arc;

use cgmath::{Deg, Quaternion, Rotation3, Vector3};
use log::{debug, info};
use specs::{Component, Read, ReadStorage, System, VecStorage, Write, WriteStorage};
use winit::dpi::PhysicalSize;

use crate::graphics::Renderer;

#[derive(Component, Debug)]
#[storage(VecStorage)]
pub struct Transform {
    pub position: Vector3<f32>,
    pub rotation: Quaternion<f32>,
    pub scale: Vector3<f32>,
}

#[derive(Default)]
pub struct ResizeEvents(pub Vec<PhysicalSize<u32>>);

#[derive(Default)]
pub struct BlendFactor(pub f32);

pub struct RenderSystem {
    renderer: Renderer,
}

impl RenderSystem {
    pub fn new(renderer: Renderer) -> Self {
        RenderSystem { renderer }
    }
}

impl<'a> System<'a> for RenderSystem {
    type SystemData = (
        Read<'a, BlendFactor>,
        Write<'a, ResizeEvents>,
        ReadStorage<'a, Transform>,
    );

    fn run(&mut self, data: Self::SystemData) {
        let (blending_factor, mut resize_events, transforms) = data;

        if resize_events.0.len() > 0 {
            self.renderer.window_resized(resize_events.0[0]);
            resize_events.0.clear();
        }
        use specs::Join;
        for transform in transforms.join() {
            // debug!(
            //     "Rendering with blending factor: {}, transform: {:?}",
            //     blending_factor.0, transform
            // );
        }
    }
}

pub struct TransformSystem;

impl<'a> System<'a> for TransformSystem {
    type SystemData = WriteStorage<'a, Transform>;

    fn run(&mut self, mut transforms: Self::SystemData) {
        use specs::Join;
        for transform in (&mut transforms).join() {
            // info!("Transform: {:?}", transform);
            // TODO: this is hardcoded for now.
            // Eventually have some controller component or system
            let axis = Vector3::new(0.0, 1.0, 0.0);
            let angle = Deg(0.5);
            let new_rotation = Quaternion::from_axis_angle(axis, angle);
            transform.rotation = transform.rotation * new_rotation;
        }
    }
}

#[derive(Component, Debug)]
#[storage(VecStorage)]
pub struct Position {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

#[derive(Component, Debug)]
#[storage(VecStorage)]
pub struct Velocity {
    pub x: f32,
    pub y: f32,
}
