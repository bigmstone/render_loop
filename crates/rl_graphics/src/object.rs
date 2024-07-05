use bytemuck::{Pod, Zeroable};

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct Vertex {
    pub _pos: [f32; 4],
    pub _tex_coord: [f32; 2],
}

pub struct Object {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u16>,
}

impl Object {
    pub fn new(vertices: Vec<Vertex>, indices: Vec<u16>) -> Self {
        Self { vertices, indices }
    }
}
