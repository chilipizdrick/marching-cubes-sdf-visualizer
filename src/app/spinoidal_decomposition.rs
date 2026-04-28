use rayon::prelude::*;

use crate::app::scalar_field::ScalarField;

pub struct SpinodalDecomposition {
    pub mu: ScalarField,
    pub gamma: f32,
    pub dt: f32,
    pub dx: f32,
}

impl SpinodalDecomposition {
    pub fn new(sdf: &ScalarField, gamma: f32, dt: f32, dx: f32) -> Self {
        let mu = ScalarField::zeroed(sdf.dims);

        Self { mu, gamma, dt, dx }
    }

    pub fn step(&mut self, sdf: &mut ScalarField, delta_time: f32) {
        let dx = self.dx;
        let dt = self.dt * delta_time;

        self.mu
            .data
            .par_iter_mut()
            .enumerate()
            .for_each(|(idx, mu)| {
                let c = sdf.data[idx];
                let (x, y, z) = sdf.linearize_index(idx);
                let lap = Self::laplacian(sdf, x, y, z, dx);
                *mu = c.powi(3) - c - self.gamma * lap;
            });

        sdf.data.par_iter_mut().enumerate().for_each(|(idx, sdf)| {
            let (x, y, z) = self.mu.linearize_index(idx);
            let lap_mu = Self::laplacian(&self.mu, x, y, z, dx);
            *sdf += dt * lap_mu;
        });
    }

    fn laplacian(sdf: &ScalarField, x: usize, y: usize, z: usize, dx: f32) -> f32 {
        let center = sdf[(x, y, z)];

        let x = x as isize;
        let y = y as isize;
        let z = z as isize;

        let left = sdf.data[sdf.wrapping_index(x - 1, y, z)];
        let right = sdf.data[sdf.wrapping_index(x + 1, y, z)];
        let top = sdf.data[sdf.wrapping_index(x, y + 1, z)];
        let bottom = sdf.data[sdf.wrapping_index(x, y - 1, z)];
        let front = sdf.data[sdf.wrapping_index(x, y, z + 1)];
        let back = sdf.data[sdf.wrapping_index(x, y, z - 1)];

        (left + right + bottom + top + back + front - 6.0 * center) / (dx * dx)
    }
}
