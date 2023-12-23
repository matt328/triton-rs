use log::info;
use std::{sync::Arc, time::Instant};
use tracing::{event, span, Level};
use vulkano::instance::InstanceExtensions;
use winit::{dpi::PhysicalSize, event::Event, window::Window};

#[cfg(feature = "tracing")]
use tracing_tracy::client::frame_mark;

use crate::game::context::Context;

use super::input::SystemEvent;

pub struct GameLoop<'a, 'b> {
    previous_instant: Instant,
    accumulated_time: f32,
    context: Context<'a, 'b>,
}

const FPS: f32 = 60.0;
// Note to self: Updates per second is number of times update is called per second
// at 60 frames, this works out to a 4 updates each frame, time permitting
const UPS: f32 = 240.0;
const MAX_FRAME_TIME: f32 = 1.0 / FPS;
const FIXED_TIME_STEP: f32 = 1.0 / UPS;

impl<'a, 'b> GameLoop<'a, 'b> {
    pub fn new(
        required_extensions: InstanceExtensions,
        window: Arc<Window>,
    ) -> anyhow::Result<Self> {
        let context = Context::new(required_extensions, window.clone())?;

        info!("Initialized Game");

        Ok(GameLoop {
            previous_instant: Instant::now(),
            accumulated_time: 0.0,
            context,
        })
    }

    pub fn resize(&mut self, new_size: PhysicalSize<u32>) {
        self.context.window_resized(new_size);
    }

    pub fn process_winit_event(&mut self, event: &Event<()>, mouse_captured: bool) -> bool {
        self.context.process_winit_event(event, mouse_captured)
    }

    pub fn process_system_event(&mut self, system_event: SystemEvent) {
        self.context.process_system_event(system_event);
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

        self.context.pre_update();

        while self.accumulated_time >= FIXED_TIME_STEP {
            event!(Level::INFO, "calling context.upadte()");
            self.context.update();
            self.accumulated_time -= FIXED_TIME_STEP;
        }

        update_loop.exit();

        let blending_factor = self.accumulated_time / FIXED_TIME_STEP;

        event!(Level::INFO, blending_factor);

        self.context.render(blending_factor)?;

        #[cfg(feature = "tracing")]
        frame_mark();

        self.previous_instant = current_instant;

        Ok(())
    }
}
