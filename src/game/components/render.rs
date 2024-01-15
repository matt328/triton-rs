use log::error;
use specs::{Component, Read, ReadStorage, System, VecStorage, Write};
use tracing::{event, Level};

use crate::Renderer;

use super::{
    resources::{BlendFactor, ResizeEvents},
    transform::Transform,
    ActiveCamera, Camera, CurrentWindowId, CurrentWindowSize,
};

#[derive(Component, Debug)]
#[storage(VecStorage)]
pub struct Renderable {
    pub mesh_id: usize,
}

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
        Option<Read<'a, ActiveCamera>>,
        Write<'a, ResizeEvents>,
        Write<'a, CurrentWindowSize>,
        Write<'a, CurrentWindowId>,
        ReadStorage<'a, Transform>,
        ReadStorage<'a, Camera>,
        ReadStorage<'a, Renderable>,
    );

    fn run(&mut self, data: Self::SystemData) {
        let (
            _blending_factor,
            active_camera,
            mut resize_events,
            mut current_window_size,
            mut current_window_id,
            transforms,
            cameras,
            meshes,
        ) = data;

        // Handle Resize Events
        if !resize_events.0 {
            event!(Level::INFO, "render system resize event");
            let _ = self.renderer.resize();
            resize_events.0 = false;
        }

        current_window_size.0 = self.renderer.window_size();
        current_window_id.0 = self.renderer.window_id();

        // Apply Active Camera's matrices
        if let Some(active_cam) = active_camera {
            let camera = cameras.get(active_cam.0).unwrap();
            self.renderer.set_camera_params(camera.calculate_matrices());
        }

        // Consider accumulating all the renderables into a list here
        // and just passing them to renderer.draw()
        // profile and see if that even has an impact
        use specs::Join;
        for (transform, mesh) in (&transforms, &meshes).join() {
            // Apply blending_factor to Transforms before passing them to renderer
            self.renderer.enqueue_mesh(mesh.mesh_id, *transform);
        }
        let result: anyhow::Result<()> = self.renderer.render();
        match result {
            Ok(_) => {}
            Err(e) => {
                error!("Error drawing: {:#?}", e);
            }
        }
    }
}
