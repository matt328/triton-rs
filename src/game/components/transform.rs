use cgmath::{Deg, Matrix4, Quaternion, Rotation3, Vector3};
use specs::{Component, System, VecStorage, WriteStorage};

#[repr(C)]
#[derive(Component, Debug, Clone, Copy)]
#[storage(VecStorage)]
pub struct Transform {
    pub position: Vector3<f32>,
    pub rotation: Quaternion<f32>,
    pub scale: Vector3<f32>,
}

impl Transform {
    pub fn model(&self) -> Matrix4<f32> {
        let scale_matrix = Matrix4::from_nonuniform_scale(self.scale.x, self.scale.y, self.scale.z);
        let rotation_matrix = Matrix4::from(self.rotation);
        let translation_matrix = Matrix4::from_translation(self.position);
        translation_matrix * rotation_matrix * scale_matrix
    }
}

pub struct TransformSystem;

impl<'a> System<'a> for TransformSystem {
    type SystemData = WriteStorage<'a, Transform>;

    fn run(&mut self, mut transforms: Self::SystemData) {
        use specs::Join;
        for transform in (&mut transforms).join() {
            // TODO: this is hardcoded for now.
            // Eventually have some controller component or system
            let axis = Vector3::new(0.0, 1.0, 0.0);
            let angle = Deg(0.5);
            let new_rotation = Quaternion::from_axis_angle(axis, angle);

            let rot = Quaternion::from(transform.rotation) * new_rotation;

            transform.rotation = rot.into();
        }
    }
}
