use std::fmt::Display;

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub enum SelectedSdf {
    #[default]
    Sphere,
    Plane,
    Octahedron,
    CoolSdf,
}

impl Display for SelectedSdf {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl SelectedSdf {
    pub fn sdf_fn(&self) -> fn(f32, f32, f32) -> f32 {
        match self {
            SelectedSdf::Sphere => sphere,
            SelectedSdf::Plane => plane,
            SelectedSdf::Octahedron => cube,
            SelectedSdf::CoolSdf => cool_sdf,
        }
    }
}

pub fn sphere(x: f32, y: f32, z: f32) -> f32 {
    x * x + y * y + z * z
}

pub fn plane(x: f32, y: f32, z: f32) -> f32 {
    x + y + z
}

pub fn cube(x: f32, y: f32, z: f32) -> f32 {
    x.abs() + y.abs() + z.abs()
}

pub fn cool_sdf(x: f32, y: f32, z: f32) -> f32 {
    ((x * x + y * y - 0.852).powi(2) + (z * z - 1.0).powi(2))
        * ((y * y + z * z - 0.852).powi(2) + (x * x - 1.0).powi(2))
        * ((z * z + x * x - 0.852).powi(2) + (y * y - 1.0).powi(2))
        - 0.001
}
