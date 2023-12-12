use cgmath::Matrix4;

pub trait Camera {
    fn calculate_matrices(&self) -> (Matrix4<f32>, Matrix4<f32>);
}
