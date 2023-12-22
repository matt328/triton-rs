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

use std::collections::HashMap;

use anyhow::anyhow;
use winit::event::Event;
use winit_input_helper::WinitInputHelper;

use crate::game::input::{sources::ActionState, MouseAxis};

use super::{
    map::ActionMap,
    sources::{ActionDescriptor, Source},
};

#[derive(Debug, Copy, Clone)]
pub enum SystemEventKind {
    Key,
    MouseMotion(MouseAxis),
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
    pub value: Option<f64>,
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

impl TryInto<Source> for SystemEvent {
    type Error = String;

    fn try_into(self) -> Result<Source, Self::Error> {
        // Create a Source that matches this SystemEvent
        match self.kind {
            SystemEventKind::Key => Ok(Source::Keyboard(self.key.unwrap())),
            SystemEventKind::MouseMotion(MouseAxis::MouseX) => {
                Ok(Source::Mouse(super::MouseSource::Move(MouseAxis::MouseX)))
            }
            SystemEventKind::MouseMotion(MouseAxis::MouseY) => {
                Ok(Source::Mouse(super::MouseSource::Move(MouseAxis::MouseY)))
            }
            _ => Err("no".to_string()),
        }
    }
}

pub struct InputSystem {
    action_descriptor_map: HashMap<String, ActionDescriptor>,
    action_map_map: HashMap<String, ActionMap>,
    current_action_map: String,
    action_state_map: HashMap<String, ActionState>,
    action_state_cache: HashMap<String, ActionState>,
    input_helper: WinitInputHelper,
}

impl InputSystem {
    pub fn new() -> Self {
        InputSystem {
            action_map_map: HashMap::new(),
            action_descriptor_map: HashMap::new(),
            current_action_map: "".to_string(),
            action_state_map: HashMap::new(),
            action_state_cache: HashMap::new(),
            input_helper: WinitInputHelper::new(),
        }
    }

    pub fn add_action(mut self, name: &str, action_descriptor: ActionDescriptor) -> Self {
        self.action_descriptor_map
            .insert(name.to_string(), action_descriptor);
        self
    }

    pub fn add_action_map(mut self, name: &str, action_map: ActionMap) -> Self {
        self.action_map_map.insert(name.to_string(), action_map);
        self.current_action_map = name.to_string();
        self
    }

    /// Clears last frame's state and queries gamepad state and adds actions to the state map.  Call
    /// this at the beginning of a frame and call process_system_event after this.
    pub fn update(&mut self) {
        /*TODO: incorporate winit_input_helper

        */
        self.action_state_map.clear();

        for (key, value) in self.action_state_cache.drain() {
            self.action_state_map.insert(key, value);
        }
        // TODO Add gamepad items into action_state_map
    }

    pub fn process_winit_event(&mut self, event: &Event<()>) -> bool {
        self.input_helper.update(event)
    }

    pub fn process_system_event(&mut self, system_event: SystemEvent) {
        let kind = system_event.kind;
        let value = system_event.value;
        if let Ok(source) = system_event.try_into() {
            if let Some(action_map) = self.action_map_map.get(&self.current_action_map) {
                if let Some(action) = action_map.map.get(&source) {
                    match kind {
                        SystemEventKind::Key => {
                            self.action_state_cache.insert(
                                action.to_string(),
                                ActionState {
                                    name: action.to_string(),
                                    active: true,
                                    active_state_changed_this_frame: false,
                                    value: None,
                                },
                            );
                        }
                        SystemEventKind::MouseMotion(_) => {
                            self.action_state_cache.insert(
                                action.to_string(),
                                ActionState {
                                    name: action.to_string(),
                                    active: true,
                                    active_state_changed_this_frame: false,
                                    value,
                                },
                            );
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    pub fn get_action_state(&self, action_name: &str) -> Option<&ActionState> {
        self.action_state_map.get(action_name)
    }

    pub fn get_action_state_map(&self) -> &HashMap<String, ActionState> {
        &self.action_state_map
    }

    pub fn activate_action_map(mut self, name: &str) -> anyhow::Result<()> {
        if self.action_map_map.contains_key(name) {
            self.current_action_map = name.to_string();
            Ok(())
        } else {
            Err(anyhow!("No action map registered"))
        }
    }
}

#[derive(Debug, Eq, Hash, PartialEq)]
pub enum MouseButton {
    Left,
    Right,
}

#[derive(Debug, Eq, Hash, PartialEq)]
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
