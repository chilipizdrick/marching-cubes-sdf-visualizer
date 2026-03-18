use glam::{Mat3A, Mat4, Vec3A};

use crate::app::transforms::normal_transform;

#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::NoUninit)]
pub struct Uniforms {
    pub model: Mat4,
    pub view: Mat4,
    pub proj: Mat4,
    pub normal: Mat3A,
    pub camera_pos: Vec3A,
}

impl Uniforms {
    pub fn new(model: Mat4, view: Mat4, proj: Mat4, camera_pos: Vec3A) -> Self {
        let normal = normal_transform(model);

        Self {
            model,
            view,
            proj,
            normal,
            camera_pos,
        }
    }

    pub fn set_model_transform(&mut self, model: Mat4) {
        let normal = normal_transform(model);

        self.model = model;
        self.normal = normal;
    }
}
