pub use system::{
    InputSystem, Key as SystemKey, MouseButton as SystemMouseButton, SystemEvent, SystemEventKind,
    SystemEventState,
};

pub use map::ActionMap;
pub use sources::{ActionDescriptor, ActionKind, ActionState, MouseAxis, MouseSource, Source};

mod map;
mod sources;
mod system;
