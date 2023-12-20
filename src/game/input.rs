/*
    things:
        - ActionState - name, a kind, and an optional value f64
        - Source - An abstraction over Key, Button, Axis
        - Binding - Contains a Source and logic to turn it into an ActionState
        - Layout/ActionSet - Mapping of Bindings to ActionStates.
            - when the binding is triggered, it translates into the ActionState
            - puts/updates an ActionState in the current_state
    System either polls a gamepad with gilrs or recieves events from winit
    - these SystemEvents get are mapped into Sources
    - get the current layout
    - look over the layout to see if any bindings exist containing said source
    - if they do, execute the binding logic to produce an Action
    - put the action into the State
    - state gets put into a Resource in the specs
*/

use log::info;

#[derive(Debug)]
pub enum SystemEventKind {
    Key,
    MouseMotion,
    MouseButton,
    MouseScroll,
}

#[derive(Debug)]
pub enum SystemEventState {
    Pressed,
    Released,
}

#[derive(Debug)]
pub struct SystemEvent {
    pub kind: SystemEventKind,
    pub state: Option<SystemEventState>,
    pub value: Option<(f64, f64)>,
    pub key: Option<Key>,
    pub mouse_button: Option<MouseButton>,
    pub repeated: bool,
}

impl Default for SystemEvent {
    fn default() -> Self {
        SystemEvent {
            kind: SystemEventKind::Key,
            state: None,
            value: None,
            key: None,
            mouse_button: None,
            repeated: false,
        }
    }
}

struct ActionState {
    name: String,
    active: bool,
    active_state_changed_this_frame: bool,
    value: Option<(f64, f64)>,
}

pub struct InputSystem {}

impl InputSystem {
    pub fn new() -> Self {
        InputSystem {}
    }
    pub fn process_system_event(&mut self, system_event: SystemEvent) {
        info!("{system_event:?}");
    }
}

#[derive(Debug)]
pub enum MouseButton {
    Left,
    Right,
}

#[derive(Debug)]
pub enum Key {
    A,
    Alt,
    Insert,
    ArrowDown,
    ArrowLeft,
    ArrowRight,
    ArrowUp,
    B,
    Backspace,
    C,
    CapsLock,
    Clear,
    Control,
    D,
    Delete,
    E,
    End,
    Enter,
    Escape,
    F,
    F1,
    F2,
    F3,
    F4,
    F5,
    F6,
    F7,
    F8,
    F9,
    F10,
    F11,
    F12,
    F13,
    F14,
    F15,
    F16,
    F17,
    F18,
    F19,
    G,
    H,
    Home,
    I,
    J,
    K,
    Key0,
    Key1,
    Key2,
    Key3,
    Key4,
    Key5,
    Key6,
    Key7,
    Key8,
    Key9,
    KeyBackSlash,
    KeyBackTick,
    KeyComma,
    KeyEquals,
    KeyForwardSlash,
    KeyFullStop,
    KeyLeftBracket,
    KeyMinus,
    KeyPlus,
    KeyRightBracket,
    KeySemicolon,
    KeySingleQuote,
    KeyStar,
    L,
    M,
    N,
    NumLock,
    O,
    P,
    PageDown,
    PageUp,
    Q,
    R,
    S,
    Shift,
    Space,
    Super,
    T,
    Tab,
    U,
    V,
    W,
    X,
    Y,
    Z,
}
