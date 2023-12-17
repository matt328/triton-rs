use cgmath::{perspective, Deg, Matrix4, Point3, Vector3};
use specs::{Component, Read, System, VecStorage, WriteStorage};
use tracing::{event, Level};

use super::ResizeEvents;

#[derive(Component, Debug, Clone, Copy)]
#[storage(VecStorage)]
pub struct Camera {
    pub fov: Deg<f32>,
    pub aspect_ratio: f32,
    pub near: f32,
    pub far: f32,
    pub eye: Point3<f32>,
    pub center: Point3<f32>,
    pub up: Vector3<f32>,
}

impl Camera {
    pub fn calculate_matrices(&self) -> (Matrix4<f32>, Matrix4<f32>) {
        (
            perspective(self.fov, self.aspect_ratio, self.near, self.far),
            Matrix4::look_at_rh(self.eye, self.center, self.up),
        )
    }
}

impl Default for Camera {
    fn default() -> Self {
        Camera {
            fov: Deg(60.0),
            aspect_ratio: 800.0 / 600.0,
            near: 0.1,
            far: 100.0,
            eye: Point3::new(3.0, -3.0, -10.0),
            center: Point3::new(0.0, 0.0, 0.0),
            up: Vector3::new(0.0, 1.0, 0.0),
        }
    }
}

pub struct CameraSystem;

impl<'a> System<'a> for CameraSystem {
    type SystemData = (Read<'a, ResizeEvents>, WriteStorage<'a, Camera>);

    fn run(&mut self, data: Self::SystemData) {
        let (resize_events, mut cameras) = data;

        let aspect = {
            if resize_events.0.len() > 0 {
                Some(resize_events.0[0].width as f32 / resize_events.0[0].height as f32)
            } else {
                None
            }
        };

        use specs::Join;

        for camera in (&mut cameras).join() {
            if let Some(value) = aspect {
                event!(Level::INFO, "aspect ratio: {}", value);
                camera.aspect_ratio = value;
            }
        }
    }
}
