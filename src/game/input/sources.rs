use gilrs::{Axis, Button};
use winit::keyboard::KeyCode;

use super::SystemMouseButton;

#[derive(Eq, Hash, PartialEq, Debug)]
pub enum Source {
    Keyboard(KeyCode),
    Mouse(MouseSource),
    Gamepad(GamepadSource),
}

#[derive(Eq, Hash, PartialEq, Copy, Clone, Debug)]
pub enum GamepadSource {
    Axis(Axis),
    Button(Button),
}

#[derive(Eq, Hash, PartialEq, Debug)]
pub enum MouseSource {
    Button(SystemMouseButton),
    Move(MouseAxis),
    Scroll(MouseAxis),
}
#[derive(Eq, Hash, PartialEq, Copy, Clone, Debug)]
pub enum MouseAxis {
    MouseX,
    MouseY,
}

#[derive(Debug, Clone)]
pub struct ActionState {
    pub name: String,
    pub active: bool,
    pub active_state_changed_this_frame: bool,
    pub value: Option<f32>,
}

pub enum ActionKind {
    Button,
    Axis,
}

// Maybe make this an enum
pub struct ActionDescriptor {
    pub kind: ActionKind,
}