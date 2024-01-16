use specs::Entity;
use winit::{dpi::PhysicalSize, window::WindowId};

#[derive(Default)]
pub struct ResizeEvents(pub bool);

#[derive(Default)]
pub struct BlendFactor(pub f32);

pub struct ActiveCamera(pub Entity);

#[derive(Default)]
pub struct CurrentWindowId(pub Option<WindowId>);

#[derive(Default)]
pub struct CurrentWindowSize(pub Option<PhysicalSize<u32>>);

#[derive(Default)]
pub struct CursorCaptured(pub Option<bool>);
