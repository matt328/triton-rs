use std::sync::Arc;

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

use crate::game::input::{
    MouseAxis, SystemEvent, SystemEventKind, SystemEventState, SystemKey, SystemMouseButton,
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
                        let result: Result<SystemKey, String> =
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
                                mouse_button: Some(SystemMouseButton::Left),
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
                                mouse_button: Some(SystemMouseButton::Right),
                                ..Default::default()
                            };
                            self.game.process_system_event(system_event);
                        }
                        _ => (),
                    },

                    Event::DeviceEvent { event, .. } => match event {
                        DeviceEvent::Motion { axis, value } => {
                            if self.mouse_captured {
                                let system_event = SystemEvent {
                                    kind: if axis == 0 {
                                        SystemEventKind::MouseMotion(MouseAxis::MouseX)
                                    } else {
                                        SystemEventKind::MouseMotion(MouseAxis::MouseY)
                                    },
                                    value: Some(value),
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

impl TryInto<SystemKey> for Key {
    type Error = String;
    fn try_into(self) -> Result<SystemKey, Self::Error> {
        match self.as_ref() {
            Key::Character("-") => Ok(SystemKey::KeyMinus),
            Key::Character("`") => Ok(SystemKey::KeyBackTick),
            Key::Character("1") => Ok(SystemKey::Key1),
            Key::Character("2") => Ok(SystemKey::Key2),
            Key::Character("3") => Ok(SystemKey::Key3),
            Key::Character("4") => Ok(SystemKey::Key4),
            Key::Character("5") => Ok(SystemKey::Key5),
            Key::Character("6") => Ok(SystemKey::Key6),
            Key::Character("7") => Ok(SystemKey::Key7),
            Key::Character("8") => Ok(SystemKey::Key8),
            Key::Character("9") => Ok(SystemKey::Key9),
            Key::Character("0") => Ok(SystemKey::Key0),
            Key::Character("=") => Ok(SystemKey::KeyEquals),
            Key::Character("*") => Ok(SystemKey::KeyStar),
            Key::Character("q") => Ok(SystemKey::Q),
            Key::Character("w") => Ok(SystemKey::W),
            Key::Character("e") => Ok(SystemKey::E),
            Key::Character("r") => Ok(SystemKey::R),
            Key::Character("t") => Ok(SystemKey::T),
            Key::Character("y") => Ok(SystemKey::Y),
            Key::Character("u") => Ok(SystemKey::U),
            Key::Character("i") => Ok(SystemKey::I),
            Key::Character("o") => Ok(SystemKey::O),
            Key::Character("p") => Ok(SystemKey::P),
            Key::Character("[") => Ok(SystemKey::KeyLeftBracket),
            Key::Character("]") => Ok(SystemKey::KeyRightBracket),
            Key::Character("a") => Ok(SystemKey::A),
            Key::Character("s") => Ok(SystemKey::S),
            Key::Character("d") => Ok(SystemKey::D),
            Key::Character("f") => Ok(SystemKey::F),
            Key::Character("g") => Ok(SystemKey::G),
            Key::Character("h") => Ok(SystemKey::H),
            Key::Character("j") => Ok(SystemKey::J),
            Key::Character("k") => Ok(SystemKey::K),
            Key::Character("l") => Ok(SystemKey::L),
            Key::Character(";") => Ok(SystemKey::KeySemicolon),
            Key::Character("'") => Ok(SystemKey::KeySingleQuote),
            Key::Character("+") => Ok(SystemKey::KeyPlus),
            Key::Character("z") => Ok(SystemKey::Z),
            Key::Character("x") => Ok(SystemKey::X),
            Key::Character("c") => Ok(SystemKey::C),
            Key::Character("v") => Ok(SystemKey::V),
            Key::Character("b") => Ok(SystemKey::B),
            Key::Character("n") => Ok(SystemKey::N),
            Key::Character("m") => Ok(SystemKey::M),
            Key::Character("),") => Ok(SystemKey::KeyComma),
            Key::Character(".") => Ok(SystemKey::KeyFullStop),
            Key::Character("/") => Ok(SystemKey::KeyForwardSlash),
            Key::Character("\\") => Ok(SystemKey::KeyBackSlash),

            Key::Named(NamedKey::Escape) => Ok(SystemKey::Escape),
            Key::Named(NamedKey::F1) => Ok(SystemKey::F1),
            Key::Named(NamedKey::F2) => Ok(SystemKey::F2),
            Key::Named(NamedKey::F3) => Ok(SystemKey::F3),
            Key::Named(NamedKey::F4) => Ok(SystemKey::F4),
            Key::Named(NamedKey::F5) => Ok(SystemKey::F5),
            Key::Named(NamedKey::F6) => Ok(SystemKey::F6),
            Key::Named(NamedKey::F7) => Ok(SystemKey::F7),
            Key::Named(NamedKey::F8) => Ok(SystemKey::F8),
            Key::Named(NamedKey::F9) => Ok(SystemKey::F9),
            Key::Named(NamedKey::F10) => Ok(SystemKey::F10),
            Key::Named(NamedKey::F11) => Ok(SystemKey::F11),
            Key::Named(NamedKey::F12) => Ok(SystemKey::F12),
            Key::Named(NamedKey::F13) => Ok(SystemKey::F13),
            Key::Named(NamedKey::F14) => Ok(SystemKey::F14),
            Key::Named(NamedKey::F15) => Ok(SystemKey::F15),
            Key::Named(NamedKey::F16) => Ok(SystemKey::F16),
            Key::Named(NamedKey::F17) => Ok(SystemKey::F17),
            Key::Named(NamedKey::F18) => Ok(SystemKey::F18),
            Key::Named(NamedKey::F19) => Ok(SystemKey::F19),
            Key::Named(NamedKey::Backspace) => Ok(SystemKey::Backspace),
            Key::Named(NamedKey::Home) => Ok(SystemKey::Home),
            Key::Named(NamedKey::PageUp) => Ok(SystemKey::PageUp),
            Key::Named(NamedKey::NumLock) => Ok(SystemKey::NumLock),
            Key::Named(NamedKey::Clear) => Ok(SystemKey::Clear),
            Key::Named(NamedKey::Tab) => Ok(SystemKey::Tab),
            Key::Named(NamedKey::Delete) => Ok(SystemKey::Delete),
            Key::Named(NamedKey::End) => Ok(SystemKey::End),
            Key::Named(NamedKey::PageDown) => Ok(SystemKey::PageDown),
            Key::Named(NamedKey::ArrowUp) => Ok(SystemKey::ArrowUp),
            Key::Named(NamedKey::CapsLock) => Ok(SystemKey::CapsLock),
            Key::Named(NamedKey::Enter) => Ok(SystemKey::Enter),
            Key::Named(NamedKey::ArrowLeft) => Ok(SystemKey::ArrowLeft),
            Key::Named(NamedKey::ArrowRight) => Ok(SystemKey::ArrowRight),
            Key::Named(NamedKey::Shift) => Ok(SystemKey::Shift),
            Key::Named(NamedKey::Alt) => Ok(SystemKey::Alt),
            Key::Named(NamedKey::Super) => Ok(SystemKey::Super),
            Key::Named(NamedKey::Space) => Ok(SystemKey::Space),
            Key::Named(NamedKey::Control) => Ok(SystemKey::Control),
            Key::Named(NamedKey::ArrowDown) => Ok(SystemKey::ArrowDown),
            Key::Named(NamedKey::Insert) => Ok(SystemKey::Insert),

            _ => Err(format!("Unsupported Key: {:?}", self)),
        }
    }
}
