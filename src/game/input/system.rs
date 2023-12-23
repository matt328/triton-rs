/*
    things:
        - ActionState - name, a kind, and an optional value f64
        - Source - An abstraction over Key, Button, Axis
        - ActionMap - Mapping of Sources to ActionStates.
            - when the binding is triggered, it translates into the ActionState
            - puts/updates an ActionState in the current_state
    System either polls a gamepad with gilrs or recieves events from winit
    - these SystemEvents get are mapped into Sources
    - get the current layout
    - look over the layout to see if any bindings exist containing said source
    - put the action into the State
    - state gets put into a Resource in the ECS
*/

use std::collections::HashMap;

use anyhow::anyhow;
use gilrs::{Axis, GamepadId, Gilrs};
use winit::event::Event;
use winit_input_helper::WinitInputHelper;

use crate::game::input::{sources::ActionState, MouseAxis};

use super::{
    map::ActionMap,
    sources::{ActionDescriptor, Source},
    GamepadSource, MouseSource,
};

pub struct InputSystem {
    action_descriptor_map: HashMap<String, ActionDescriptor>,
    action_map_map: HashMap<String, ActionMap>,
    current_action_map: String,
    action_state_map: HashMap<String, ActionState>,
    input_helper: WinitInputHelper,
    gilrs: Gilrs,
    current_gamepad: Option<GamepadId>,
}

impl Default for InputSystem {
    fn default() -> Self {
        Self::new()
    }
}

// TODO: pull this out into preferences
const RIGHT_STICK_MULTIPLIER: f32 = 6.0;
const INVERT_RIGHT_STICK_Y: f32 = 1.0;

impl InputSystem {
    pub fn new() -> Self {
        InputSystem {
            action_map_map: HashMap::new(),
            action_descriptor_map: HashMap::new(),
            current_action_map: "".to_string(),
            action_state_map: HashMap::new(),
            input_helper: WinitInputHelper::new(),
            gilrs: Gilrs::new().unwrap(),
            current_gamepad: None,
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

    pub fn update(&mut self) {
        self.action_state_map.clear();
    }

    pub fn update_gamepads(&mut self) {
        while let Some(event) = self.gilrs.next_event() {
            if self.current_gamepad.is_none() {
                self.current_gamepad = Some(event.id);
            }
        }

        if let Some(gamepad_id) = self.current_gamepad {
            let gamepad = self.gilrs.gamepad(gamepad_id);
            if let Some(action_map) = self.action_map_map.get(&self.current_action_map) {
                for (source, name) in action_map.map.iter() {
                    match source {
                        Source::Gamepad(GamepadSource::Axis(
                            axis @ Axis::LeftStickY | axis @ Axis::LeftStickX,
                        )) => {
                            if let Some(axis_data) =
                                gamepad.axis_data(*axis).filter(|v| v.value() != 0.0)
                            {
                                let name_clone = name.to_string();
                                self.action_state_map.insert(
                                    name_clone.clone(),
                                    ActionState {
                                        name: name_clone,
                                        active: true,
                                        active_state_changed_this_frame: false,
                                        value: Some(axis_data.value()),
                                    },
                                );
                            }
                        }

                        Source::Gamepad(GamepadSource::Axis(
                            axis @ Axis::RightStickY | axis @ Axis::RightStickX,
                        )) => {
                            if let Some(axis_data) =
                                gamepad.axis_data(*axis).filter(|v| v.value() != 0.0)
                            {
                                let name_clone = name.to_string();
                                self.action_state_map.insert(
                                    name_clone.clone(),
                                    ActionState {
                                        name: name_clone,
                                        active: true,
                                        active_state_changed_this_frame: false,
                                        value: Some(
                                            axis_data.value()
                                                * RIGHT_STICK_MULTIPLIER
                                                * if *axis == Axis::RightStickY {
                                                    INVERT_RIGHT_STICK_Y
                                                } else {
                                                    1.0
                                                },
                                        ),
                                    },
                                );
                            }
                        }

                        _ => {}
                    }
                }
            }
        }
    }

    pub fn process_winit_event(&mut self, event: &Event<()>, mouse_captured: bool) -> bool {
        if self.input_helper.update(event) {
            if let Some(action_map) = self.action_map_map.get(&self.current_action_map) {
                for (source, name) in action_map.map.iter() {
                    match source {
                        Source::Keyboard(keycode) => {
                            if self.input_helper.key_held(*keycode) {
                                self.action_state_map.insert(
                                    name.to_string(),
                                    ActionState {
                                        name: name.to_string(),
                                        active: true,
                                        active_state_changed_this_frame: false,
                                        value: None,
                                    },
                                );
                            }
                        }

                        Source::Mouse(MouseSource::Move(axis)) => {
                            if mouse_captured {
                                let mouse_diff = self.input_helper.mouse_diff();
                                match axis {
                                    MouseAxis::MouseX => {
                                        self.action_state_map.insert(
                                            name.to_string(),
                                            ActionState {
                                                name: name.to_string(),
                                                active: true,
                                                active_state_changed_this_frame: false,
                                                value: Some(mouse_diff.0),
                                            },
                                        );
                                    }
                                    MouseAxis::MouseY => {
                                        self.action_state_map.insert(
                                            name.to_string(),
                                            ActionState {
                                                name: name.to_string(),
                                                active: true,
                                                active_state_changed_this_frame: false,
                                                value: Some(mouse_diff.1),
                                            },
                                        );
                                    }
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
        true
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
