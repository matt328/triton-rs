use log::error;
use specs::{Component, Read, ReadStorage, System, VecStorage, Write};
use tracing::{event, Level};

use crate::graphics::RenderCoordinator;

use super::{
    resources::{BlendFactor, ResizeEvents},
    transform::Transform,
    ActiveCamera, Camera,
};

#[derive(Component, Debug)]
#[storage(VecStorage)]
pub struct Renderable {
    pub mesh_id: usize,
}

pub struct RenderSystem {
    coordinator: RenderCoordinator,
}

impl RenderSystem {
    pub fn new(coordinator: RenderCoordinator) -> Self {
        RenderSystem { coordinator }
    }
}

impl<'a> System<'a> for RenderSystem {
    type SystemData = (
        Read<'a, BlendFactor>,
        Option<Read<'a, ActiveCamera>>,
        Write<'a, ResizeEvents>,
        ReadStorage<'a, Transform>,
        ReadStorage<'a, Camera>,
        ReadStorage<'a, Renderable>,
    );

    fn run(&mut self, data: Self::SystemData) {
        let (_blending_factor, active_camera, mut resize_events, transforms, cameras, meshes) =
            data;

        // Handle Resize Events
        if !resize_events.0.is_empty() {
            event!(Level::INFO, "render system resize event");
            self.coordinator.window_resized(resize_events.0[0]);
            resize_events.0.clear();
        }

        // Apply Active Camera's matrices
        if let Some(active_cam) = active_camera {
            let camera = cameras.get(active_cam.0).unwrap();
            self.coordinator
                .set_camera_params(camera.calculate_matrices());
        }

        // Consider accumulating all the renderables into a list here
        // and just passing them to renderer.draw()
        // profile and see if that even has an impact
        use specs::Join;
        for (transform, mesh) in (&transforms, &meshes).join() {
            // Apply blending_factor to Transforms before passing them to renderer
            self.coordinator.enqueue_mesh(mesh.mesh_id, *transform);
        }
        let result: anyhow::Result<()> = self.coordinator.draw();
        match result {
            Ok(_) => {}
            Err(e) => {
                error!("Error drawing: {:#?}", e);
            }
        }
    }
}
