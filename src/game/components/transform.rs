use cgmath::{Deg, Matrix4, Quaternion, Rotation3, Vector3};
use specs::{Component, System, VecStorage, WriteStorage};
use vulkano::buffer::BufferContents;

#[repr(C)]
#[derive(BufferContents, Component, Debug, Clone, Copy)]
#[storage(VecStorage)]
pub struct Transform {
    pub position: [f32; 3],
    pub rotation: [f32; 4],
    pub scale: [f32; 3],
}

impl Transform {
    pub fn model(&self) -> Matrix4<f32> {
        let scale_matrix =
            Matrix4::from_nonuniform_scale(self.scale[0], self.scale[1], self.scale[2]);
        let rotation_matrix = Matrix4::from(Quaternion::new(
            self.rotation[0],
            self.rotation[1],
            self.rotation[2],
            self.rotation[3],
        ));
        let translation_matrix = Matrix4::from_translation(Vector3::new(
            self.position[0],
            self.position[1],
            self.position[2],
        ));
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
