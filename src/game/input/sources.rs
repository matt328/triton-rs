use super::{SystemKey, SystemMouseButton};

#[derive(Eq, Hash, PartialEq)]
pub enum Source {
    Keyboard(SystemKey),
    Mouse(MouseSource),
}

#[derive(Eq, Hash, PartialEq)]
pub enum MouseSource {
    Button(SystemMouseButton),
    Move(MouseAxis),
    Scroll(MouseAxis),
}
#[derive(Eq, Hash, PartialEq)]
pub enum MouseAxis {
    MouseX,
    MouseY,
}

#[derive(Debug)]
pub struct ActionState {
    pub name: String,
    pub active: bool,
    pub active_state_changed_this_frame: bool,
    pub value: Option<(f64, f64)>,
}

pub enum ActionKind {
    Button,
    Axis,
}

// Maybe make this an enum
pub struct ActionDescriptor {
    pub kind: ActionKind,
}
