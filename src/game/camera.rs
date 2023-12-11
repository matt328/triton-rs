use cgmath::{perspective, Deg, Matrix4, Point3, Vector3};

use crate::graphics::Camera;

pub struct MouseLookCamera {
    fov: Deg<f32>,
    aspect_ratio: f32,
    near: f32,
    far: f32,
    eye: Point3<f32>,
    center: Point3<f32>,
    up: Vector3<f32>,
}

impl MouseLookCamera {
    pub fn new_with_aspect(aspect: f32) -> Self {
        MouseLookCamera {
            fov: Deg(60.0),
            aspect_ratio: aspect,
            near: 0.1,
            far: 100.0,
            eye: Point3::new(3.0, -3.0, 3.0),
            center: Point3::new(0.0, 0.0, 0.0),
            up: Vector3::new(0.0, 1.0, 0.0),
        }
    }
}

impl Camera for MouseLookCamera {
    fn calculate_matrices(&self) -> (Matrix4<f32>, Matrix4<f32>) {
        (
            perspective(self.fov, self.aspect_ratio, self.near, self.far),
            Matrix4::look_at_rh(self.eye, self.center, self.up),
        )
    }
}
