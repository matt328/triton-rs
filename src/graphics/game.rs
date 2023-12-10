use std::{sync::Arc, time::Instant};

use vulkano::instance::InstanceExtensions;
use winit::{dpi::PhysicalSize, window::Window};

use super::Renderer;

pub struct Game {
    previous_instant: Instant,
    accumulated_time: f64,
    renderer: Renderer,
}

const FPS: f64 = 60.0;
const MAX_FRAME_TIME: f64 = 1.0 / FPS;
const FIXED_TIME_STEP: f64 = 1.0 / 240.0;

impl Game {
    pub fn new(
        required_extensions: InstanceExtensions,
        window: Arc<Window>,
    ) -> anyhow::Result<Self> {
        let renderer = Renderer::new(required_extensions, window.clone())?;

        Ok(Game {
            previous_instant: Instant::now(),
            accumulated_time: 0.0,
            renderer,
        })
    }

    pub fn resize(&mut self, new_size: PhysicalSize<u32>) {
        self.renderer.window_resized(new_size);
    }

    /// Implements fixed timestep game loop
    pub fn update(&mut self) -> anyhow::Result<()> {
        let current_instant = Instant::now();

        let mut elapsed = current_instant
            .duration_since(self.previous_instant)
            .as_secs_f64();

        if elapsed > MAX_FRAME_TIME {
            elapsed = MAX_FRAME_TIME;
        }

        self.accumulated_time += elapsed;

        while self.accumulated_time >= FIXED_TIME_STEP {
            let _ = self.update_game_state();
            self.accumulated_time -= FIXED_TIME_STEP;
        }

        let blending_factor = self.accumulated_time / FIXED_TIME_STEP;

        let _current_state = self.blend_game_state(blending_factor);

        let _rendered = self.render_game();

        self.previous_instant = current_instant;

        Ok(())
    }

    pub fn update_game_state(&mut self) -> anyhow::Result<()> {
        Ok(())
    }

    pub fn blend_game_state(&mut self, _blending_factor: f64) -> anyhow::Result<()> {
        Ok(())
    }

    pub fn render_game(&mut self) -> anyhow::Result<()> {
        self.renderer.draw()
    }
}
