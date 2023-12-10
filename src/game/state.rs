#[derive(Copy, Clone, Debug)]
pub struct State {
    triangle_rotation: f64,
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
    let rot = state.triangle_rotation + 1.0;

    State {
        triangle_rotation: rot,
    }
}

/// Performs linear interpolation over the entire state
pub fn blend_state(prev: &State, next: &State, blending_factor: f64) -> State {
    let result = prev.triangle_rotation
        + (next.triangle_rotation - prev.triangle_rotation) * blending_factor;
    State {
        triangle_rotation: result,
    }
}
