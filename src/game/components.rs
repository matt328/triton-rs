use cgmath::{Deg, Quaternion, Rotation3, Vector3};
use log::info;
use specs::{Component, System, VecStorage, WriteStorage};

#[derive(Component, Debug)]
#[storage(VecStorage)]
pub struct Transform {
    pub position: Vector3<f32>,
    pub rotation: Quaternion<f32>,
    pub scale: Vector3<f32>,
}

pub struct TransformSystem;

impl<'a> System<'a> for TransformSystem {
    type SystemData = WriteStorage<'a, Transform>;

    fn run(&mut self, mut transforms: Self::SystemData) {
        use specs::Join;
        for transform in (&mut transforms).join() {
            info!("Transform: {:?}", transform);
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
