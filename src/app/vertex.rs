use glam::Vec3A;

#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::NoUninit)]
pub struct Vertex {
    pub position: Vec3A,
    pub normal: Vec3A,
}

impl Vertex {
    pub fn new(position: Vec3A, normal: Vec3A) -> Self {
        Self { position, normal }
    }
}

pub struct MeshData {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
}

impl MeshData {
    pub fn new() -> Self {
        Self {
            vertices: Vec::new(),
            indices: Vec::new(),
        }
    }
}
