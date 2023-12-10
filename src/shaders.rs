use vulkano::{
    buffer::BufferContents, pipeline::graphics::vertex_input::Vertex, shader::EntryPoint,
};

#[derive(BufferContents, Vertex)]
#[repr(C)]
pub struct Position {
    #[format(R32G32B32_SFLOAT)]
    position: [f32; 3],
}

#[derive(BufferContents, Vertex)]
#[repr(C)]
pub struct Normal {
    #[format(R32G32B32_SFLOAT)]
    normal: [f32; 3],
}

pub struct Effect {
    pub vertex: EntryPoint,
    pub tess_control: Option<EntryPoint>,
    pub tess_evaluation: Option<EntryPoint>,
    pub fragment: EntryPoint,
}

impl Effect {
    pub fn builder(vertex_shader: EntryPoint, fragment_shader: EntryPoint) -> EffectBuilder {
        EffectBuilder::new(vertex_shader, fragment_shader)
    }

    pub fn is_tesselation(&self) -> bool {
        self.tess_control.is_some() && self.tess_evaluation.is_some()
    }
}

pub struct EffectBuilder {
    vertex: EntryPoint,
    tess_control: Option<EntryPoint>,
    tess_evaluation: Option<EntryPoint>,
    fragment: EntryPoint,
}

impl EffectBuilder {
    pub fn new(vertex_shader: EntryPoint, fragment_shader: EntryPoint) -> EffectBuilder {
        EffectBuilder {
            vertex: vertex_shader,
            fragment: fragment_shader,
            tess_control: None,
            tess_evaluation: None,
        }
    }

    pub fn tesselation_control_shader(mut self, shader: EntryPoint) -> EffectBuilder {
        self.tess_control = Some(shader);
        self
    }

    pub fn tesselation_evaluation_shader(mut self, shader: EntryPoint) -> EffectBuilder {
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
