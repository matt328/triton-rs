use std::collections::HashMap;

use super::sources::Source;

pub struct ActionMap {
    pub map: HashMap<Source, String>,
}

impl ActionMap {
    pub fn new() -> Self {
        ActionMap {
            map: HashMap::new(),
        }
    }

    pub fn bind(mut self, source: Source, action_name: &str) -> Self {
        self.map.insert(source, action_name.to_string());
        self
    }
}
