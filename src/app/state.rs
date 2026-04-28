use std::{f32::consts::FRAC_PI_2, path::PathBuf, time::Instant};

use glam::{Mat4, Quat, Vec3, Vec3A};
use winit::{
    event::{ElementState, KeyEvent},
    keyboard::{KeyCode, PhysicalKey},
};

use crate::app::{
    marching_cubes::{GenerationMethod, MarchingCubes},
    mesh::Mesh,
    scalar_field::{Dimentions3D, ScalarField},
    sdf_functions::SdfFuction,
    spinoidal_decomposition::SpinodalDecomposition,
    uniforms::Uniforms,
};

pub struct State {
    pub camera: Camera,
    pub camera_controller: CameraController,
    pub camera_mode_on: bool,
    pub delta_time: f32,
    pub keys_pressed: KeysPressed,
    pub last_update: Instant,
    pub mesh: Mesh,
    pub mesh_generation_method: GenerationMethod,
    pub sdf: ScalarField,
    pub selected_grid_dims: Dimentions3D,
    pub visualization_mode: VisualizationMode,
    pub spin_decomp: SpinodalDecomposition,
    pub time: f32,
    pub uniforms: Uniforms,
    pub simulation_running: bool,
    pub isovalue: f32,
    pub selected_sdf_file_path: Option<PathBuf>,
    pub mesh_changed: bool,
    pub fn_grid_settings: FnGridSettings,
    pub selected_sdf_fn: SdfFuction,
}

impl State {
    pub fn new(screen_aspect_ratio: f32) -> Self {
        let grid_dims = Dimentions3D::square(20);

        let mut sdf = ScalarField::zeroed(grid_dims);
        sdf.randomze(-1.0..=1.0);

        let mesh = Mesh::new();
        let spin_decomp = SpinodalDecomposition::new(&sdf, 10.0, 0.01, 1.0);
        let camera_controller = CameraController::new(1.0, 0.002);
        let camera = Camera::new(2.0 * Vec3A::NEG_Y, screen_aspect_ratio);
        let model = model_transform_for_centered_grid_of_size(grid_dims);
        let view_proj = camera.view_proj_matrix();
        let uniforms = Uniforms::new(model, view_proj, camera.position);

        Self {
            camera,
            camera_controller,
            camera_mode_on: false,
            delta_time: 0.0,
            keys_pressed: KeysPressed::default(),
            last_update: Instant::now(),
            mesh,
            mesh_generation_method: GenerationMethod::Standard,
            sdf,
            selected_grid_dims: grid_dims,
            visualization_mode: VisualizationMode::Fn,
            spin_decomp,
            time: 0.0,
            uniforms,
            simulation_running: false,
            isovalue: 0.5,
            selected_sdf_file_path: None,
            mesh_changed: false,
            fn_grid_settings: FnGridSettings::default(),
            selected_sdf_fn: SdfFuction::default(),
        }
    }

    pub fn update_time(&mut self) {
        let now = Instant::now();
        self.delta_time = (now - self.last_update).as_secs_f32();
        self.time += self.delta_time;
        self.last_update = now;
    }

    pub fn update(&mut self) {
        self.update_time();

        if self.visualization_mode == VisualizationMode::SpinodalDecomposition
            && self.simulation_running
        {
            self.spin_decomp.step(&mut self.sdf, self.delta_time);
            self.generate_mesh(0.0);
        }

        self.camera_controller
            .update_camera(&mut self.camera, self.delta_time, &self.keys_pressed);

        self.uniforms.camera_pos = self.camera.position;
        let view_proj = self.camera.view_proj_matrix();
        self.uniforms.view_proj = view_proj;
    }

    pub fn generate_mesh(&mut self, isovalue: f32) {
        self.mesh_changed = true;
        MarchingCubes::generate_mesh(
            &self.sdf,
            isovalue,
            self.mesh_generation_method,
            &mut self.mesh,
        );
    }

    pub fn handle_mouse_motion(&mut self, (dx, dy): (f64, f64)) {
        if self.camera_mode_on {
            self.camera_controller
                .process_mouse(dx as f32, dy as f32, &mut self.camera);
        }
    }

    pub fn handle_key_event(&mut self, key_event: &KeyEvent) {
        // log::debug!("key event: {key_event:?}");

        if self.camera_mode_on
            && let PhysicalKey::Code(key_code) = key_event.physical_key
        {
            let state = key_event.state == ElementState::Pressed;
            match key_code {
                KeyCode::KeyW => self.keys_pressed.forward = state,
                KeyCode::KeyA => self.keys_pressed.left = state,
                KeyCode::KeyS => self.keys_pressed.backward = state,
                KeyCode::KeyD => self.keys_pressed.right = state,
                KeyCode::Space => self.keys_pressed.up = state,
                KeyCode::ControlLeft => self.keys_pressed.down = state,
                KeyCode::ShiftLeft => self.keys_pressed.sprint = state,
                _ => {}
            }
        }
    }

    pub fn update_grid_dims(&mut self) {
        let Dimentions3D { x, y, z } = self.selected_grid_dims;

        self.sdf.resize(self.selected_grid_dims);
        self.spin_decomp.mu.resize(self.selected_grid_dims);

        let max_scale = x.max(y).max(z) as f32;
        let max_scale_twice = max_scale * 2.0;
        let translation = -Vec3::new(
            (x as f32) / max_scale_twice,
            (y as f32) / max_scale_twice,
            (z as f32) / max_scale_twice,
        );
        let model = Mat4::from_scale_rotation_translation(
            Vec3::splat(1. / max_scale),
            Quat::IDENTITY,
            translation,
        );
        self.uniforms.set_model_transform(model);
    }

    pub fn update_grid_dims_from_sdf_fn_settings(&mut self) {
        let dims = self.fn_grid_settings.get_dimensions();
        self.selected_grid_dims = dims;
        self.update_grid_dims();
    }

    pub fn randomize_grid(&mut self) {
        self.sdf.randomze(-1.0..=1.0);
    }

    pub fn randomize_grid_and_generate_mesh(&mut self) {
        self.randomize_grid();
        self.generate_mesh(0.0);
    }

    pub fn select_sdf_file_path(&mut self, path: PathBuf) {
        self.selected_sdf_file_path = Some(path);
    }

    pub fn construct_sdf_from_file(&mut self) -> std::io::Result<()> {
        self.update_grid_dims();
        self.mesh.clear();

        if let Some(path) = &self.selected_sdf_file_path {
            let bytes = std::fs::read(path)?;

            let Dimentions3D { x, y, z } = self.selected_grid_dims;

            for j in 0..y {
                for k in 0..z {
                    for i in 0..x {
                        let val = bytes[i + k * x + j * x * z];
                        self.sdf[(i, j, k)] = val as f32 / 255.0;
                    }
                }
            }
        }

        self.generate_mesh(self.isovalue);

        Ok(())
    }

    pub fn construct_sdf_from_fn(&mut self) {
        self.update_grid_dims_from_sdf_fn_settings();

        let sdf_fn = self.selected_sdf_fn.sdf_fn();
        let (dx, dy, dz) = self.fn_grid_settings.delta;
        let from_x = self.fn_grid_settings.x_range.0;
        let from_y = self.fn_grid_settings.y_range.0;
        let from_z = self.fn_grid_settings.z_range.0;

        for i in 0..self.sdf.dims.x {
            for j in 0..self.sdf.dims.y {
                for k in 0..self.sdf.dims.z {
                    let x = from_x + i as f32 * dx;
                    let y = from_y + j as f32 * dy;
                    let z = from_z + k as f32 * dz;

                    let val = sdf_fn(x, y, z);
                    self.sdf[(i, j, k)] = val;
                }
            }
        }

        self.generate_mesh(self.isovalue);
    }
}

#[derive(Debug, Default)]
pub struct KeysPressed {
    pub forward: bool,
    pub backward: bool,
    pub left: bool,
    pub right: bool,
    pub up: bool,
    pub down: bool,
    pub sprint: bool,
}

pub struct Camera {
    pub position: Vec3A,
    pub yaw: f32,
    pub pitch: f32,
    pub fov_y: f32,
    pub aspect_ratio: f32,
    pub z_near: f32,
    pub z_far: f32,
}

impl Camera {
    pub fn new(position: Vec3A, aspect_ratio: f32) -> Self {
        Self {
            position,
            yaw: 0.0,
            pitch: 0.0,
            fov_y: 75.0_f32.to_radians(),
            aspect_ratio,
            z_near: 0.1,
            z_far: 1000.0,
        }
    }

    pub fn view_matrix(&self) -> Mat4 {
        Mat4::look_to_rh(self.position.into(), self.forward().into(), Vec3::Z)
    }

    pub fn projection_matrix(&self) -> Mat4 {
        Mat4::perspective_rh(self.fov_y, self.aspect_ratio, self.z_near, self.z_far)
    }

    pub fn view_proj_matrix(&self) -> Mat4 {
        let view = self.view_matrix();
        let proj = self.projection_matrix();
        proj * view
    }

    pub fn forward(&self) -> Vec3A {
        let yaw_sin = self.yaw.sin();
        let yaw_cos = self.yaw.cos();
        let pitch_sin = self.pitch.sin();
        let pitch_cos = self.pitch.cos();

        Vec3A::new(yaw_sin * pitch_cos, yaw_cos * pitch_cos, pitch_sin).normalize()
    }

    #[inline]
    pub fn right(&self) -> Vec3A {
        self.forward().cross(Vec3A::Z).normalize()
    }
}

pub struct CameraController {
    pub speed: f32,
    pub sensitivity: f32,
}

impl CameraController {
    pub fn new(speed: f32, sensitivity: f32) -> Self {
        Self { speed, sensitivity }
    }

    pub fn process_mouse(&mut self, mouse_dx: f32, mouse_dy: f32, camera: &mut Camera) {
        camera.yaw += mouse_dx * self.sensitivity;
        camera.pitch -= mouse_dy * self.sensitivity;

        let limit = FRAC_PI_2 - 0.001;
        camera.pitch = camera.pitch.clamp(-limit, limit);
    }

    pub fn update_camera(&self, camera: &mut Camera, dt: f32, keys: &KeysPressed) {
        let forward = camera.forward();
        let right = camera.right();

        let forward_flat = Vec3A::new(forward.x, forward.y, 0.0).normalize_or_zero();

        let mut velocity = Vec3A::ZERO;

        if keys.forward {
            velocity += forward_flat;
        }
        if keys.backward {
            velocity -= forward_flat;
        }
        if keys.right {
            velocity += right;
        }
        if keys.left {
            velocity -= right;
        }
        if keys.up {
            velocity += Vec3A::Z;
        }
        if keys.down {
            velocity -= Vec3A::Z;
        }

        let speed = if keys.sprint {
            self.speed * 2.0
        } else {
            self.speed
        };

        if velocity.length_squared() > 0.0 {
            camera.position += velocity.normalize() * speed * dt;
        }
    }
}

#[derive(Debug, Default, PartialEq)]
pub enum VisualizationMode {
    #[default]
    Fn,
    LoadFromFile,
    SpinodalDecomposition,
}

pub fn model_transform_for_centered_grid_of_size(grid_dims: Dimentions3D) -> Mat4 {
    let Dimentions3D { x, y, z } = grid_dims;
    let max_scale = x.max(y).max(z) as f32;
    let max_scale_twice = max_scale * 2.0;

    let translation = -Vec3::new(
        (x as f32) / max_scale_twice,
        (y as f32) / max_scale_twice,
        (z as f32) / max_scale_twice,
    );

    Mat4::from_scale_rotation_translation(Vec3::splat(1.0 / max_scale), Quat::IDENTITY, translation)
}

#[derive(Debug, Clone, Copy)]
pub struct FnGridSettings {
    pub x_range: (f32, f32),
    pub y_range: (f32, f32),
    pub z_range: (f32, f32),
    pub delta: (f32, f32, f32),
}

impl Default for FnGridSettings {
    fn default() -> Self {
        Self {
            x_range: (-1.1, 1.1),
            y_range: (-1.1, 1.1),
            z_range: (-1.1, 1.1),
            delta: (0.02, 0.02, 0.02),
        }
    }
}

impl FnGridSettings {
    pub fn get_dimensions(&self) -> Dimentions3D {
        let nx = ((self.x_range.1 - self.x_range.0) / self.delta.0) as usize + 1;
        let ny = ((self.y_range.1 - self.y_range.0) / self.delta.1) as usize + 1;
        let nz = ((self.z_range.1 - self.z_range.0) / self.delta.2) as usize + 1;

        Dimentions3D::new(nx, ny, nz)
    }
}
