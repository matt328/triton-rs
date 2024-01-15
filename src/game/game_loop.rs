use anyhow::Context;
use std::time::Instant;
use tracing::{span, Level};
use winit::{dpi::PhysicalSize, event::Event, event_loop::EventLoop, window::WindowId};

#[cfg(feature = "tracing")]
use tracing_tracy::client::frame_mark;

use super::context::GameContext;

pub struct GameLoop {
    previous_instant: Instant,
    accumulated_time: f32,
    context: GameContext,
}

const FPS: f32 = 60.0;
// Note to self: Updates per second is number of times update is called per second
// at 60 frames, this works out to a 4 updates each frame, time permitting
const UPS: f32 = 240.0;
const MAX_FRAME_TIME: f32 = 1.0 / FPS;
const FIXED_TIME_STEP: f32 = 1.0 / UPS;

impl GameLoop {
    pub fn new(event_loop: &EventLoop<()>) -> anyhow::Result<Self> {
        let context = GameContext::new(event_loop).context("creating game context")?;
        Ok(GameLoop {
            previous_instant: Instant::now(),
            accumulated_time: 0.0,
            context,
        })
    }

    pub fn window_size(&self) -> Option<PhysicalSize<u32>> {
        self.context.window_size()
    }

    pub fn window_id(&self) -> Option<WindowId> {
        self.context.window_id()
    }

    pub fn resize(&mut self) -> anyhow::Result<()> {
        self.context.resize()
    }

    pub fn process_winit_event(&mut self, event: &Event<()>, mouse_captured: bool) -> bool {
        self.context.process_winit_event(event, mouse_captured)
    }

    /// Implements fixed timestep game loop https://gafferongames.com/post/fix_your_timestep/
    pub fn update(&mut self) -> anyhow::Result<()> {
        let _update = span!(Level::INFO, "game update", self.accumulated_time).entered();

        let current_instant = Instant::now();

        let mut elapsed = current_instant
            .duration_since(self.previous_instant)
            .as_secs_f32();

        if elapsed > MAX_FRAME_TIME {
            elapsed = MAX_FRAME_TIME;
        }

        self.accumulated_time += elapsed;

        let update_loop = span!(Level::INFO, "update loop").entered();

        self.context.pre_update();

        while self.accumulated_time >= FIXED_TIME_STEP {
            self.context.update();
            self.accumulated_time -= FIXED_TIME_STEP;
        }

        update_loop.exit();

        let blending_factor = self.accumulated_time / FIXED_TIME_STEP;

        self.context.render(blending_factor)?;

        #[cfg(feature = "tracing")]
        frame_mark();

        self.previous_instant = current_instant;

        Ok(())
    }
}
