#[derive(Copy, Clone, Debug)]
pub struct State {
    pub triangle_rotation: f32,
}

impl Default for State {
    fn default() -> Self {
        State {
            triangle_rotation: 0.0,
        }
    }
}

/// Simulates one 'tick' in the evolution of the state
pub fn next_state(state: &State) -> State {
    let rot = state.triangle_rotation + 0.25;

    State {
        triangle_rotation: rot,
    }
}

/// Performs linear interpolation over the entire state
pub fn blend_state(prev: &State, next: &State, blending_factor: f32) -> State {
    let result =
        prev.triangle_rotation * (1.0 - blending_factor) + next.triangle_rotation * blending_factor;
    State {
        triangle_rotation: result,
    }
}
