use std::{sync::Arc, time::Instant};

use log::info;
use tracing::{event, span, Level};
use vulkano::instance::InstanceExtensions;
use winit::{dpi::PhysicalSize, window::Window};

#[cfg(feature = "tracing")]
use tracing_tracy::client::frame_mark;

use crate::{game::state::blend_state, graphics::Renderer};

use super::state::{next_state, State};

pub struct GameLoop {
    previous_instant: Instant,
    accumulated_time: f32,
    renderer: Renderer,
    state: State,
}

const FPS: f32 = 60.0;
// Note to self: Updates per second is number of times update is called per second
// at 60 frames, this works out to a 4 updates each frame, time permitting
const UPS: f32 = 240.0;
const MAX_FRAME_TIME: f32 = 1.0 / FPS;
const FIXED_TIME_STEP: f32 = 1.0 / UPS;

impl GameLoop {
    pub fn new(
        required_extensions: InstanceExtensions,
        window: Arc<Window>,
    ) -> anyhow::Result<Self> {
        let renderer = Renderer::new(required_extensions, window.clone())?;

        info!("Initialized Game");

        Ok(GameLoop {
            previous_instant: Instant::now(),
            accumulated_time: 0.0,
            renderer,
            state: State::default(),
        })
    }

    pub fn resize(&mut self, new_size: PhysicalSize<u32>) {
        self.renderer.window_resized(new_size);
    }

    /// Implements fixed timestep game loop https://gafferongames.com/post/fix_your_timestep/
    pub fn update(&mut self) -> anyhow::Result<()> {
        let _update = span!(Level::INFO, "game update", self.accumulated_time).entered();

        let current_instant = Instant::now();

        let mut elapsed = current_instant
            .duration_since(self.previous_instant)
            .as_secs_f32();

        event!(Level::INFO, elapsed);

        if elapsed > MAX_FRAME_TIME {
            event!(Level::WARN, "elapsed > MAX_FRAME_TIME");
            elapsed = MAX_FRAME_TIME;
        }

        self.accumulated_time += elapsed;

        let update_loop = span!(Level::INFO, "update loop").entered();

        let mut previous_state = self.state;

        while self.accumulated_time >= FIXED_TIME_STEP {
            previous_state = self.state;
            self.state = next_state(&self.state);
            self.accumulated_time -= FIXED_TIME_STEP;
        }

        update_loop.exit();

        let blending_factor = self.accumulated_time / FIXED_TIME_STEP;

        event!(Level::INFO, blending_factor);

        self.state = blend_state(&previous_state, &self.state, blending_factor);

        let _rendered = self.render_game();

        #[cfg(feature = "tracing")]
        frame_mark();

        self.previous_instant = current_instant;

        Ok(())
    }

    pub fn render_game(&mut self) -> anyhow::Result<()> {
        let _span = span!(Level::INFO, "render_game").entered();
        let f = format!("{:?}", self.state);
        event!(Level::INFO, f);
        self.renderer.draw(self.state)
    }
}
