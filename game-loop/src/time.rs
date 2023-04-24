pub trait TimeTrait: Copy {
    fn now() -> Self;
    fn sub(&self, other: &Self) -> f64;
}

pub use time::*;

#[cfg(not(target_arch = "wasm32"))]
mod time {
    use super::*;
    use std::time::Instant;

    #[derive(Copy, Clone)]
    pub struct Time(Instant);

    impl TimeTrait for Time {
        fn now() -> Self {
            Self(Instant::now())
        }

        fn sub(&self, other: &Self) -> f64 {
            self.0.duration_since(other.0).as_secs_f64()
        }
    }
}
