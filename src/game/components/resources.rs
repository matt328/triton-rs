use specs::Entity;
use winit::dpi::PhysicalSize;

#[derive(Default)]
pub struct ResizeEvents(pub Vec<PhysicalSize<u32>>);

#[derive(Default)]
pub struct BlendFactor(pub f32);

pub struct ActiveCamera(pub Entity);
