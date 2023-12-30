// Note to self: all the pub use statements here define the public api of the 'graphics' module
pub use self::coordinator::RenderCoordinator;
pub use self::shaders::{CUBE_INDICES, CUBE_VERTICES};
// Note to self: this entire module is not public, only structs called out above are
// usable outside this module.

mod basic_renderer;
mod coordinator;
mod helpers;
mod imgui;
mod mesh;
mod render_data;
mod renderer;
mod shaders;
