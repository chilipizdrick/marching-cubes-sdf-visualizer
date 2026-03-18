use glam::{Mat3A, Mat4, Quat, Vec3A, Vec4};

pub fn model_transform(scale: Vec3A, position: Vec3A, rotation: Quat) -> Mat4 {
    Mat4::from_scale_rotation_translation(scale.into(), rotation, position.into())
}

pub fn view_transform(camera_pos: Vec3A, target: Vec3A, up: Vec3A) -> Mat4 {
    let f = (target - camera_pos).normalize();
    let r = f.cross(up).normalize();
    let u = r.cross(f);

    Mat4::from_cols(
        Vec4::new(r.x, u.x, -f.x, 0.0),
        Vec4::new(r.y, u.y, -f.y, 0.0),
        Vec4::new(r.z, u.z, -f.z, 0.0),
        Vec4::new(
            -r.dot(camera_pos),
            -u.dot(camera_pos),
            f.dot(camera_pos),
            1.0,
        ),
    )
}

pub fn projection_transform(
    fov_y_radians: f32,
    aspect_ratio: f32,
    z_near: f32,
    z_far: f32,
) -> Mat4 {
    let t = (fov_y_radians / 2.0).tan();

    Mat4::from_cols(
        Vec4::new(1.0 / (aspect_ratio * t), 0.0, 0.0, 0.0),
        Vec4::new(0.0, 1.0 / t, 0.0, 0.0),
        Vec4::new(0.0, 0.0, (z_far + z_near) / (z_near - z_far), -1.0),
        Vec4::new(0.0, 0.0, (2.0 * z_far * z_near) / (z_near - z_far), 0.0),
    )
}

pub fn normal_transform(model: Mat4) -> Mat3A {
    Mat3A::from_mat4(model.inverse().transpose())
}
