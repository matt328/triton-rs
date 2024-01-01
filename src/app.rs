use std::sync::Arc;

use anyhow::Context;
use vulkano::swapchain::Surface;
use winit::{
    event::{ElementState, Event, MouseButton, WindowEvent},
    event_loop::{ControlFlow, EventLoop, EventLoopWindowTarget},
    keyboard::{KeyCode, PhysicalKey},
    window::{Window, WindowBuilder},
};

use crate::game::GameLoop;

pub struct App<'a, 'b> {
    event_loop: EventLoop<()>,
    game: GameLoop<'a, 'b>,
    window: Arc<Window>,
    mouse_captured: bool,
}

impl<'a, 'b> App<'a, 'b> {
    pub fn new() -> anyhow::Result<Self> {
        let event_loop = EventLoop::new().context("Creating event loop")?;
        let required_extensions = Surface::required_extensions(&event_loop);

        let window = Arc::new(
            WindowBuilder::new()
                .with_title("Triton")
                .build(&event_loop)?,
        );

        let game = GameLoop::new(required_extensions, window.clone())?;

        Ok(App {
            event_loop,
            game,
            window,
            mouse_captured: false,
        })
    }

    pub fn run(mut self) -> anyhow::Result<()> {
        self.event_loop
            .run(move |event, elwt: &EventLoopWindowTarget<()>| {
                elwt.set_control_flow(ControlFlow::Poll);

                self.game.process_winit_event(&event, self.mouse_captured);

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
                    } => match self.game.update() {
                        Ok(()) => {}
                        Err(error) => {
                            log::error!("{error}");
                        }
                    },

                    Event::WindowEvent {
                        event:
                            WindowEvent::KeyboardInput {
                                device_id: _,
                                event,
                                is_synthetic: _,
                            },
                        ..
                    } => {
                        if event.physical_key == PhysicalKey::Code(KeyCode::Escape) {
                            self.window
                                .set_cursor_grab(winit::window::CursorGrabMode::None)
                                .unwrap();
                            self.window.set_cursor_visible(true);
                            self.mouse_captured = false;
                        }
                    }

                    // Eventually Move this inside the engine itself.
                    Event::WindowEvent {
                        event: WindowEvent::MouseInput { state, button, .. },
                        ..
                    } => match (state, button) {
                        (ElementState::Released, MouseButton::Left) => {
                            #[cfg(not(target_os = "macos"))]
                            self.window
                                .set_cursor_grab(winit::window::CursorGrabMode::Confined)
                                .unwrap();

                            #[cfg(target_os = "macos")]
                            self.window
                                .set_cursor_grab(winit::window::CursorGrabMode::Locked)
                                .unwrap();

                            self.mouse_captured = true;
                            self.window.set_cursor_visible(false);
                        }
                        (ElementState::Released, MouseButton::Right) => {
                            self.window
                                .set_cursor_grab(winit::window::CursorGrabMode::None)
                                .unwrap();
                            self.window.set_cursor_visible(true);
                            self.mouse_captured = false;
                        }
                        _ => (),
                    },

                    _ => (),
                }
            })
            .context("Processing EventLoop")
    }
}
