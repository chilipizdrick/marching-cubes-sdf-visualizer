use glam::{Mat3A, Mat4, Vec3A};

#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::NoUninit)]
pub struct Uniforms {
    pub model: Mat4,
    pub view_proj: Mat4,
    pub normal: Mat3A,
    pub camera_pos: Vec3A,
}

impl Uniforms {
    pub fn new(model: Mat4, view_proj: Mat4, camera_pos: Vec3A) -> Self {
        let normal = normal_transform(model);

        Self {
            model,
            view_proj,
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

pub fn normal_transform(model: Mat4) -> Mat3A {
    Mat3A::from_mat4(model.inverse().transpose())
}
