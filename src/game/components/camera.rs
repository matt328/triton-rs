use cgmath::{
    perspective, Deg, EuclideanSpace, Euler, Matrix4, Point3, Quaternion, Rad, Rotation, Vector3,
    Zero,
};
use specs::{Component, Read, System, VecStorage, WriteStorage};
use tracing::{event, Level};

use crate::game::context::InputStateResource;

use super::CurrentWindowSize;

#[derive(Component, Debug, Clone, Copy)]
#[storage(VecStorage)]
pub struct Camera {
    pub fov: Deg<f32>,
    pub aspect_ratio: f32,
    pub near: f32,
    pub far: f32,

    pub position: Vector3<f32>,
    pub rotation: Quaternion<f32>,
    pub velocity: Vector3<f32>,
    pub y_velocity: f32,
}

impl Camera {
    pub fn calculate_matrices(&self) -> (Matrix4<f32>, Matrix4<f32>) {
        (
            perspective(self.fov, self.aspect_ratio, self.near, self.far),
            Matrix4::look_at_rh(
                Point3::from_vec(self.position),
                Point3::from_vec(self.position) + self.rotation.rotate_vector(Vector3::unit_z()),
                Vector3::unit_y(),
            ),
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
            position: Vector3::new(3.0, 0.0, -10.0),
            rotation: Quaternion::new(1.0, 0.0, 0.0, 0.0),
            velocity: Vector3::zero(),
            y_velocity: 0.0,
        }
    }
}

pub struct CameraSystem;

impl<'a> System<'a> for CameraSystem {
    type SystemData = (
        Read<'a, CurrentWindowSize>,
        Read<'a, InputStateResource>,
        WriteStorage<'a, Camera>,
    );

    fn run(&mut self, data: Self::SystemData) {
        let (current_window_size, input_state, mut cameras) = data;

        let aspect = current_window_size
            .0
            .map(|ps| ps.width as f32 / ps.height as f32);

        let delta_x = input_state
            .0
            .get("look_vertical_action")
            .and_then(|dx| dx.value);

        let delta_y = input_state
            .0
            .get("look_horizontal_action")
            .and_then(|dy| dy.value);

        use specs::Join;

        for camera in (&mut cameras).join() {
            let pitch_quat = {
                if let Some(y) = delta_y {
                    Quaternion::from(Euler {
                        x: Rad(0.0),
                        y: Rad(-y * 0.001),
                        z: Rad(0.0),
                    })
                } else {
                    Quaternion::new(1.0, 0.0, 0.0, 0.0)
                }
            };

            let yaw_quat: Quaternion<f32> = {
                if let Some(x) = delta_x {
                    Quaternion::from(Euler {
                        x: Rad(-x * 0.001),
                        y: Rad(0.0),
                        z: Rad(0.0),
                    })
                } else {
                    Quaternion::new(1.0, 0.0, 0.0, 0.0)
                }
            };

            camera.rotation = camera.rotation * (pitch_quat * yaw_quat);

            if let Some(state) = input_state.0.get("walk_forward") {
                let direction = camera.rotation.rotate_vector(Vector3::new(0.0, 0.0, 1.0));
                camera.velocity += direction * state.value.unwrap_or(0.5);
            }

            if input_state.0.get("walk_backward").is_some() {
                let direction = camera.rotation.rotate_vector(Vector3::new(0.0, 0.0, -1.0));
                camera.velocity += direction * 0.5;
            }

            if let Some(state) = input_state.0.get("strafe_right") {
                let direction = camera.rotation.rotate_vector(Vector3::new(0.0, 0.0, -1.0));
                let right = direction.cross(Vector3::unit_y());
                camera.velocity -= right * state.value.unwrap_or(0.5);
            }

            if input_state.0.get("strafe_left").is_some() {
                let direction = camera.rotation.rotate_vector(Vector3::new(0.0, 0.0, -1.0));
                let left = direction.cross(Vector3::unit_y());
                camera.velocity += left * 0.5;
            }

            if input_state.0.get("move_up").is_some() {
                camera.y_velocity -= 0.5;
            }

            if input_state.0.get("move_down").is_some() {
                camera.y_velocity += 0.5;
            }

            if let Some(value) = aspect {
                event!(Level::INFO, "aspect ratio: {}", value);
                camera.aspect_ratio = value;
            }

            camera.position += camera.velocity * 0.16;
            camera.position.y += camera.y_velocity * 0.16;

            camera.velocity = Vector3::zero();
            camera.y_velocity = 0.0;
        }
    }
}
