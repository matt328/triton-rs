use std::fmt;

use cgmath::{Matrix4, SquareMatrix};

use super::{mesh::BasicMesh, shaders::vs_position_color::ObjectData};

pub struct RenderData {
    meshes: Vec<BasicMesh>,
    object_data: Vec<(usize, ObjectData)>,
    cam_matrices: (Matrix4<f32>, Matrix4<f32>),
}

impl RenderData {
    pub fn mesh_position(&self) -> usize {
        self.meshes.len()
    }

    pub fn add_mesh(&mut self, mesh: BasicMesh) {
        self.meshes.push(mesh);
    }

    pub fn reset_object_data(&mut self) {
        self.object_data = vec![];
    }

    pub fn add_object_data(&mut self, mesh_id: usize, object_data: ObjectData) {
        self.object_data.push((mesh_id, object_data));
    }

    pub fn update_cam_matrices(&mut self, matrices: (Matrix4<f32>, Matrix4<f32>)) {
        self.cam_matrices = matrices;
    }

    pub fn cam_matrices(&self) -> (Matrix4<f32>, Matrix4<f32>) {
        self.cam_matrices
    }

    pub fn object_data(&self) -> Vec<ObjectData> {
        self.object_data.iter().map(|a| a.1).collect()
    }

    /// Produces a vector containing a tuple of the ObjectData's index, and the mesh itself.
    pub fn render_iter<'a>(&'a self) -> impl Iterator<Item = (u32, &'a BasicMesh)> {
        self.object_data
            .iter()
            .enumerate()
            .map(|(index, (mesh_index, _))| (index as u32, &self.meshes[*mesh_index]))
    }
}

impl fmt::Debug for RenderData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "RenderData mesh_count: {}, object_data_count: {}",
            self.meshes.len(),
            self.object_data.len(),
        )
    }
}

impl Default for RenderData {
    fn default() -> Self {
        RenderData {
            meshes: vec![],
            object_data: vec![],
            cam_matrices: (Matrix4::identity(), Matrix4::identity()),
        }
    }
}
