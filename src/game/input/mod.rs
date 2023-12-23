pub use system::{InputSystem, MouseButton as SystemMouseButton};

pub use map::ActionMap;
pub use sources::{
    ActionDescriptor, ActionKind, ActionState, GamepadSource, MouseAxis, MouseSource, Source,
};

mod map;
mod sources;
mod system;
