use std::ops::{Index, IndexMut, RangeInclusive};

#[derive(Debug, Clone)]
pub struct ScalarField {
    pub data: Vec<f32>,
    pub dims: Dimentions3D,
}

impl ScalarField {
    #[allow(unused)]
    pub fn square(side_length: usize) -> Self {
        let data = vec![0.0; side_length * side_length * side_length];
        let dims = Dimentions3D::square(side_length);
        Self { data, dims }
    }

    pub fn zeroed(dims: Dimentions3D) -> Self {
        let Dimentions3D { x, y, z } = dims;
        let data = vec![0.0; x * y * z];
        Self { data, dims }
    }

    pub fn randomze(&mut self, range: RangeInclusive<f32>) {
        for i in 0..self.data.len() {
            self.data[i] = rand::random_range(range.clone());
        }
    }

    #[inline]
    pub fn index(&self, x: usize, y: usize, z: usize) -> usize {
        x + y * self.dims.x + z * self.dims.x * self.dims.y
    }

    pub fn linearize_index(&self, index: usize) -> (usize, usize, usize) {
        let Dimentions3D {
            x: nx,
            y: ny,
            z: nz,
        } = self.dims;

        let x = index % nx;
        let y = (index / nx) % ny;
        let z = (index / nx / ny) % nz;

        (x, y, z)
    }

    #[inline]
    pub fn wrapping_index(&self, x: isize, y: isize, z: isize) -> usize {
        let Dimentions3D {
            x: nx,
            y: ny,
            z: nz,
        } = self.dims;

        let nx = nx as isize;
        let ny = ny as isize;
        let nz = nz as isize;

        let x = (x % nx + nx) % nx;
        let y = (y % ny + ny) % ny;
        let z = (z % nz + nz) % nz;

        self.index(x as usize, y as usize, z as usize)
    }

    pub fn resize(&mut self, dims: Dimentions3D) {
        let Dimentions3D { x, y, z } = dims;
        self.dims = dims;
        self.data.resize(x * y * z, 0.0);
    }
}

impl Index<(usize, usize, usize)> for ScalarField {
    type Output = f32;

    #[inline]
    fn index(&self, index: (usize, usize, usize)) -> &Self::Output {
        let (x, y, z) = index;
        &self.data[self.index(x, y, z)]
    }
}

impl IndexMut<(usize, usize, usize)> for ScalarField {
    #[inline]
    fn index_mut(&mut self, index: (usize, usize, usize)) -> &mut Self::Output {
        let (x, y, z) = index;
        let index = self.index(x, y, z);
        &mut self.data[index]
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Dimentions3D {
    pub x: usize,
    pub y: usize,
    pub z: usize,
}

impl Dimentions3D {
    #[allow(unused)]
    pub fn new(x: usize, y: usize, z: usize) -> Self {
        Self { x, y, z }
    }

    pub fn square(side_length: usize) -> Self {
        Self {
            x: side_length,
            y: side_length,
            z: side_length,
        }
    }
}
