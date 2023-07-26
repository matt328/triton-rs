use std::sync::Arc;

use vulkano::{
    buffer::BufferContents, pipeline::graphics::vertex_input::Vertex, shader::ShaderModule,
};

#[derive(BufferContents, Vertex)]
#[repr(C)]
pub struct Vertex2 {
    #[format(R32G32B32_SFLOAT)]
    position: [f32; 3],
}

#[repr(C)]
#[derive(BufferContents, Vertex)]
struct Vertex3 {
    #[format(R32G32B32_SFLOAT)]
    normal: [f32; 3],
}

pub struct Effect {
    pub vertex: Arc<ShaderModule>,
    pub tess_control: Option<Arc<ShaderModule>>,
    pub tess_evaluation: Option<Arc<ShaderModule>>,
    pub fragment: Arc<ShaderModule>,
}

impl Effect {
    pub fn builder(
        vertex_shader: Arc<ShaderModule>,
        fragment_shader: Arc<ShaderModule>,
    ) -> EffectBuilder {
        EffectBuilder::new(vertex_shader, fragment_shader)
    }

    pub fn is_tesselation(&self) -> bool {
        self.tess_control.is_some() && self.tess_evaluation.is_some()
    }
}

pub struct EffectBuilder {
    vertex: Arc<ShaderModule>,
    tess_control: Option<Arc<ShaderModule>>,
    tess_evaluation: Option<Arc<ShaderModule>>,
    fragment: Arc<ShaderModule>,
}

impl EffectBuilder {
    pub fn new(
        vertex_shader: Arc<ShaderModule>,
        fragment_shader: Arc<ShaderModule>,
    ) -> EffectBuilder {
        EffectBuilder {
            vertex: vertex_shader,
            fragment: fragment_shader,
            tess_control: None,
            tess_evaluation: None,
        }
    }

    pub fn tesselation_control_shader(mut self, shader: Arc<ShaderModule>) -> EffectBuilder {
        self.tess_control = Some(shader);
        self
    }

    pub fn tesselation_evaluation_shader(mut self, shader: Arc<ShaderModule>) -> EffectBuilder {
        self.tess_evaluation = Some(shader);
        self
    }

    pub fn build(self) -> Effect {
        Effect {
            vertex: self.vertex,
            tess_control: self.tess_control,
            tess_evaluation: self.tess_evaluation,
            fragment: self.fragment,
        }
    }
}

pub mod vs {
    vulkano_shaders::shader! {
        ty: "vertex",
        path: "./assets/shaders/basic/vert.glsl",
    }
}

pub mod fs {
    vulkano_shaders::shader! {
        ty: "fragment",
        path: "./assets/shaders/basic/frag.glsl"
    }
}
