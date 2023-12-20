use std::sync::Arc;

use super::game::Key as MyKey;
use anyhow::Context;
use log::warn;
use vulkano::swapchain::Surface;
use winit::{
    event::{DeviceEvent, ElementState, Event, MouseButton, WindowEvent},
    event_loop::{ControlFlow, EventLoop, EventLoopWindowTarget},
    keyboard::{Key, NamedKey},
    platform::modifier_supplement::KeyEventExtModifierSupplement,
    window::{Window, WindowBuilder},
};

use crate::game::{
    GameLoop, MouseButton as MyMouseButton, SystemEvent, SystemEventKind, SystemEventState,
};

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

                    Event::WindowEvent {
                        event: WindowEvent::KeyboardInput { event, .. },
                        ..
                    } => {
                        let result: Result<MyKey, String> =
                            event.key_without_modifiers().try_into();

                        if let Ok(my_key) = result {
                            let system_event = SystemEvent {
                                kind: SystemEventKind::Key,
                                repeated: event.repeat,
                                state: if event.state == ElementState::Pressed {
                                    Some(SystemEventState::Pressed)
                                } else {
                                    Some(SystemEventState::Released)
                                },
                                key: Some(my_key),
                                ..Default::default()
                            };
                            self.game.process_system_event(system_event);
                        } else if let Err(e) = result {
                            warn!("{}", e);
                        }
                    }

                    Event::WindowEvent {
                        event: WindowEvent::MouseInput { state, button, .. },
                        ..
                    } => match (state, button) {
                        (ElementState::Released, MouseButton::Left) => {
                            self.window
                                .set_cursor_grab(winit::window::CursorGrabMode::Confined)
                                .unwrap();
                            self.mouse_captured = true;
                            self.window.set_cursor_visible(false);
                            let system_event = SystemEvent {
                                kind: SystemEventKind::MouseButton,
                                state: Some(SystemEventState::Released),
                                mouse_button: Some(MyMouseButton::Left),
                                ..Default::default()
                            };
                            self.game.process_system_event(system_event);
                        }
                        (ElementState::Released, MouseButton::Right) => {
                            self.window
                                .set_cursor_grab(winit::window::CursorGrabMode::None)
                                .unwrap();
                            self.window.set_cursor_visible(true);
                            self.mouse_captured = false;
                            let system_event = SystemEvent {
                                kind: SystemEventKind::MouseButton,
                                state: Some(SystemEventState::Released),
                                mouse_button: Some(MyMouseButton::Right),
                                ..Default::default()
                            };
                            self.game.process_system_event(system_event);
                        }
                        _ => (),
                    },

                    Event::DeviceEvent { event, .. } => match event {
                        DeviceEvent::MouseMotion { delta } => {
                            if self.mouse_captured {
                                let system_event = SystemEvent {
                                    kind: SystemEventKind::MouseMotion,
                                    value: Some(delta),
                                    ..Default::default()
                                };
                                self.game.process_system_event(system_event);
                            }
                        }
                        _ => {}
                    },

                    _ => (),
                }
            })
            .context("Processing EventLoop")
    }
}

impl TryInto<MyKey> for Key {
    type Error = String;
    fn try_into(self) -> Result<MyKey, Self::Error> {
        match self.as_ref() {
            Key::Character("-") => Ok(MyKey::KeyMinus),
            Key::Character("`") => Ok(MyKey::KeyBackTick),
            Key::Character("1") => Ok(MyKey::Key1),
            Key::Character("2") => Ok(MyKey::Key2),
            Key::Character("3") => Ok(MyKey::Key3),
            Key::Character("4") => Ok(MyKey::Key4),
            Key::Character("5") => Ok(MyKey::Key5),
            Key::Character("6") => Ok(MyKey::Key6),
            Key::Character("7") => Ok(MyKey::Key7),
            Key::Character("8") => Ok(MyKey::Key8),
            Key::Character("9") => Ok(MyKey::Key9),
            Key::Character("0") => Ok(MyKey::Key0),
            Key::Character("=") => Ok(MyKey::KeyEquals),
            Key::Character("*") => Ok(MyKey::KeyStar),
            Key::Character("q") => Ok(MyKey::Q),
            Key::Character("w") => Ok(MyKey::W),
            Key::Character("e") => Ok(MyKey::E),
            Key::Character("r") => Ok(MyKey::R),
            Key::Character("t") => Ok(MyKey::T),
            Key::Character("y") => Ok(MyKey::Y),
            Key::Character("u") => Ok(MyKey::U),
            Key::Character("i") => Ok(MyKey::I),
            Key::Character("o") => Ok(MyKey::O),
            Key::Character("p") => Ok(MyKey::P),
            Key::Character("[") => Ok(MyKey::KeyLeftBracket),
            Key::Character("]") => Ok(MyKey::KeyRightBracket),
            Key::Character("a") => Ok(MyKey::A),
            Key::Character("s") => Ok(MyKey::S),
            Key::Character("d") => Ok(MyKey::D),
            Key::Character("f") => Ok(MyKey::F),
            Key::Character("g") => Ok(MyKey::G),
            Key::Character("h") => Ok(MyKey::H),
            Key::Character("j") => Ok(MyKey::J),
            Key::Character("k") => Ok(MyKey::K),
            Key::Character("l") => Ok(MyKey::L),
            Key::Character(";") => Ok(MyKey::KeySemicolon),
            Key::Character("'") => Ok(MyKey::KeySingleQuote),
            Key::Character("+") => Ok(MyKey::KeyPlus),
            Key::Character("z") => Ok(MyKey::Z),
            Key::Character("x") => Ok(MyKey::X),
            Key::Character("c") => Ok(MyKey::C),
            Key::Character("v") => Ok(MyKey::V),
            Key::Character("b") => Ok(MyKey::B),
            Key::Character("n") => Ok(MyKey::N),
            Key::Character("m") => Ok(MyKey::M),
            Key::Character("),") => Ok(MyKey::KeyComma),
            Key::Character(".") => Ok(MyKey::KeyFullStop),
            Key::Character("/") => Ok(MyKey::KeyForwardSlash),
            Key::Character("\\") => Ok(MyKey::KeyBackSlash),

            Key::Named(NamedKey::Escape) => Ok(MyKey::Escape),
            Key::Named(NamedKey::F1) => Ok(MyKey::F1),
            Key::Named(NamedKey::F2) => Ok(MyKey::F2),
            Key::Named(NamedKey::F3) => Ok(MyKey::F3),
            Key::Named(NamedKey::F4) => Ok(MyKey::F4),
            Key::Named(NamedKey::F5) => Ok(MyKey::F5),
            Key::Named(NamedKey::F6) => Ok(MyKey::F6),
            Key::Named(NamedKey::F7) => Ok(MyKey::F7),
            Key::Named(NamedKey::F8) => Ok(MyKey::F8),
            Key::Named(NamedKey::F9) => Ok(MyKey::F9),
            Key::Named(NamedKey::F10) => Ok(MyKey::F10),
            Key::Named(NamedKey::F11) => Ok(MyKey::F11),
            Key::Named(NamedKey::F12) => Ok(MyKey::F12),
            Key::Named(NamedKey::F13) => Ok(MyKey::F13),
            Key::Named(NamedKey::F14) => Ok(MyKey::F14),
            Key::Named(NamedKey::F15) => Ok(MyKey::F15),
            Key::Named(NamedKey::F16) => Ok(MyKey::F16),
            Key::Named(NamedKey::F17) => Ok(MyKey::F17),
            Key::Named(NamedKey::F18) => Ok(MyKey::F18),
            Key::Named(NamedKey::F19) => Ok(MyKey::F19),
            Key::Named(NamedKey::Backspace) => Ok(MyKey::Backspace),
            Key::Named(NamedKey::Home) => Ok(MyKey::Home),
            Key::Named(NamedKey::PageUp) => Ok(MyKey::PageUp),
            Key::Named(NamedKey::NumLock) => Ok(MyKey::NumLock),
            Key::Named(NamedKey::Clear) => Ok(MyKey::Clear),
            Key::Named(NamedKey::Tab) => Ok(MyKey::Tab),
            Key::Named(NamedKey::Delete) => Ok(MyKey::Delete),
            Key::Named(NamedKey::End) => Ok(MyKey::End),
            Key::Named(NamedKey::PageDown) => Ok(MyKey::PageDown),
            Key::Named(NamedKey::ArrowUp) => Ok(MyKey::ArrowUp),
            Key::Named(NamedKey::CapsLock) => Ok(MyKey::CapsLock),
            Key::Named(NamedKey::Enter) => Ok(MyKey::Enter),
            Key::Named(NamedKey::ArrowLeft) => Ok(MyKey::ArrowLeft),
            Key::Named(NamedKey::ArrowRight) => Ok(MyKey::ArrowRight),
            Key::Named(NamedKey::Shift) => Ok(MyKey::Shift),
            Key::Named(NamedKey::Alt) => Ok(MyKey::Alt),
            Key::Named(NamedKey::Super) => Ok(MyKey::Super),
            Key::Named(NamedKey::Space) => Ok(MyKey::Space),
            Key::Named(NamedKey::Control) => Ok(MyKey::Control),
            Key::Named(NamedKey::ArrowDown) => Ok(MyKey::ArrowDown),
            Key::Named(NamedKey::Insert) => Ok(MyKey::Insert),

            _ => Err(format!("Unsupported Key: {:?}", self)),
        }
    }
}
