use std::{ops::Index, path::Path};

pub struct ScalarField {
    pub x_len: usize,
    pub y_len: usize,
    pub z_len: usize,
    pub data: Vec<f32>,
}

impl ScalarField {
    pub fn read_from_u8_yzx_file_with_size(
        path: impl AsRef<Path>,
        x_len: usize,
        y_len: usize,
        z_len: usize,
    ) -> std::io::Result<Self> {
        let bytes = std::fs::read(path)?;

        let mut data = vec![0.0; x_len * y_len * z_len];

        for j in 0..y_len {
            for k in 0..z_len {
                for i in 0..x_len {
                    let val = bytes[(i + j * x_len + k * x_len * y_len) as usize];
                    data[i + j * x_len + k * x_len * y_len] = val as f32 / 255.0;
                }
            }
        }

        Ok(Self {
            x_len,
            y_len,
            z_len,
            data,
        })
    }
}

impl Index<(usize, usize, usize)> for ScalarField {
    type Output = f32;

    fn index(&self, index: (usize, usize, usize)) -> &Self::Output {
        &self.data[index.0 + index.1 * self.x_len + index.2 * self.x_len * self.y_len]
    }
}
