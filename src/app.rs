use std::sync::Arc;

use anyhow::Context;
use log::info;
use vulkano::swapchain::Surface;
use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop, EventLoopWindowTarget},
    window::{Window, WindowBuilder},
};

use crate::game::Game;

pub struct App {
    event_loop: EventLoop<()>,
    game: Game,
    window: Arc<Window>,
}

impl App {
    pub fn new() -> anyhow::Result<Self> {
        let event_loop = EventLoop::new().unwrap();
        let required_extensions = Surface::required_extensions(&event_loop);

        let window = Arc::new(WindowBuilder::new().build(&event_loop).unwrap());

        let game = Game::new(required_extensions, window.clone())?;

        Ok(App {
            event_loop,
            game,
            window,
        })
    }

    pub fn run(mut self) -> anyhow::Result<()> {
        self.event_loop
            .run(move |event, elwt: &EventLoopWindowTarget<()>| {
                elwt.set_control_flow(ControlFlow::Poll);
                match event {
                    Event::WindowEvent {
                        event: WindowEvent::CloseRequested,
                        ..
                    } => {
                        elwt.exit();
                    }
                    Event::WindowEvent {
                        event: WindowEvent::Resized(new_size),
                        ..
                    } => {
                        self.game.resize(new_size);
                    }
                    Event::AboutToWait => {
                        self.window.request_redraw();
                    }
                    Event::WindowEvent {
                        event: WindowEvent::RedrawRequested,
                        ..
                    } => {
                        let _ = self.game.update();
                    }
                    _ => (),
                }
            })
            .context("Processing EventLoop")
    }
}
