use bytemuck::Pod;
use cgmath::{Deg, Matrix4, Quaternion, Rotation3, SquareMatrix, Vector3};

use log::{error, info};
use specs::{Component, Read, ReadStorage, System, VecStorage, Write, WriteStorage};
use tracing::{event, Level};
use vulkano::buffer::BufferContents;
use winit::dpi::PhysicalSize;

use crate::graphics::Renderer;

#[derive(Component, Debug)]
#[storage(VecStorage)]
pub struct Renderable {
    pub mesh_id: usize,
}

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
        ReadStorage<'a, Renderable>,
    );

    fn run(&mut self, data: Self::SystemData) {
        let (blending_factor, mut resize_events, transforms, meshes) = data;

        if resize_events.0.len() > 0 {
            event!(Level::INFO, "render system resize event");
            self.renderer.window_resized(resize_events.0[0]);

            resize_events.0.clear();
        }
        // Consider accumulating all the renderables into a list here
        // and just passing them to renderer.draw()
        // profile and see if that even has an impact
        use specs::Join;
        for (transform, mesh) in (&transforms, &meshes).join() {
            // Apply blending_factor to Transforms before passing them to renderer
            self.renderer.enqueue_mesh(mesh.mesh_id, *transform);
        }
        let result: anyhow::Result<()> = self.renderer.draw();
        match result {
            Ok(_) => {}
            Err(e) => {
                error!("Error drawing: {:#?}", e);
            }
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

            let rot = Quaternion::from(transform.rotation) * new_rotation;

            transform.rotation = rot.into();
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
