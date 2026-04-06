// The layout of one cube
//
//     +-----e7-----+
//    /|v8         /|v7
//   e12          e11
//  /  e8        /  e6
// +-----e3-----+   |
// |v4 |        |v3 |
// |   +-----e5-|---+
// e4 / v5      e2 / v6
// | e9         | e10      y
// |/           |/         ^ z
// +-----e1-----+          |/
//  v1           v2        + -> x
//
// v1 = (i,   j,   k  )
// v2 = (i+1, j,   k  )
// v3 = (i+1, j+1, k  )
// v4 = (i,   j+1, k  )
// v5 = (i,   j,   k+1)
// v6 = (i+1, j,   k+1)
// v7 = (i+1, j+1, k+1)
// v8 = (i,   j+1, k+1)

mod lookup_tables;

use std::collections::HashMap;

use glam::Vec3A;

use crate::app::{
    raw_loader::ScalarField,
    vertex::{MeshData, Vertex},
};

use self::lookup_tables::*;

const EDGE_VERTS: [(usize, usize); 12] = [
    (0, 1),
    (1, 2),
    (2, 3),
    (3, 0),
    (4, 5),
    (5, 6),
    (6, 7),
    (7, 4),
    (0, 4),
    (1, 5),
    (2, 6),
    (3, 7),
];

const VERT_OFFSET: [[usize; 3]; 8] = [
    [0, 0, 0],
    [1, 0, 0],
    [1, 1, 0],
    [0, 1, 0],
    [0, 0, 1],
    [1, 0, 1],
    [1, 1, 1],
    [0, 1, 1],
];

pub type SdfFn<'a> = &'a mut dyn FnMut(f32, f32, f32) -> f32;

#[derive(Default)]
pub struct GridBuilder {
    x_range: Option<(f32, f32)>,
    y_range: Option<(f32, f32)>,
    z_range: Option<(f32, f32)>,

    delta: Option<(f32, f32, f32)>,
}

impl GridBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn x_range(mut self, x_range: (f32, f32)) -> Self {
        self.x_range = Some(x_range);
        self
    }

    pub fn y_range(mut self, y_range: (f32, f32)) -> Self {
        self.y_range = Some(y_range);
        self
    }

    pub fn z_range(mut self, z_range: (f32, f32)) -> Self {
        self.z_range = Some(z_range);
        self
    }

    pub fn xyz_delta(mut self, delta: (f32, f32, f32)) -> Self {
        self.delta = Some(delta);
        self
    }

    pub fn build(self) -> Option<Grid> {
        Some(Grid::new(
            self.x_range?,
            self.y_range?,
            self.z_range?,
            self.delta?,
        ))
    }
}

#[derive(Debug)]
pub struct Grid {
    x_range: (f32, f32),
    y_range: (f32, f32),
    z_range: (f32, f32),
    delta: (f32, f32, f32),

    x_len: usize,
    y_len: usize,
    z_len: usize,

    // A grid containing the cube vertices as one bit per voxel
    grid: Vec<u8>,
}

impl Grid {
    pub fn builder() -> GridBuilder {
        GridBuilder::new()
    }

    pub fn new(
        x_range: (f32, f32),
        y_range: (f32, f32),
        z_range: (f32, f32),
        delta: (f32, f32, f32),
    ) -> Self {
        let x_len = ((x_range.1 - x_range.0) / delta.0).ceil() as usize + 1;
        let y_len = ((y_range.1 - y_range.0) / delta.1).ceil() as usize + 1;
        let z_len = ((z_range.1 - z_range.0) / delta.2).ceil() as usize + 1;

        let grid_len = (x_len * y_len * z_len).div_ceil(8);

        let grid = vec![0; grid_len];

        Self {
            x_range,
            y_range,
            z_range,
            delta,

            x_len,
            y_len,
            z_len,

            grid,
        }
    }

    #[inline]
    fn index(&self, x: usize, y: usize, z: usize) -> usize {
        x + y * self.x_len + z * self.x_len * self.y_len
    }

    #[inline]
    fn get_voxel(&self, x: usize, y: usize, z: usize) -> bool {
        let idx = self.index(x, y, z);
        let byte_idx = idx >> 3;
        let bit_mask = 1 << (idx & 7);

        (self.grid[byte_idx] & bit_mask) != 0
    }

    #[inline]
    fn set_voxel(&mut self, x: usize, y: usize, z: usize, value: bool) {
        let idx = self.index(x, y, z);
        let byte_idx = idx >> 3;
        let bit_mask = 1 << (idx & 7);

        if value {
            self.grid[byte_idx] |= bit_mask;
        } else {
            self.grid[byte_idx] &= !bit_mask;
        }
    }

    #[inline]
    fn x_value(&self, x: usize) -> f32 {
        self.x_range.0 + (x as f32 * self.delta.0)
    }

    #[inline]
    fn y_value(&self, y: usize) -> f32 {
        self.y_range.0 + (y as f32 * self.delta.1)
    }

    #[inline]
    fn z_value(&self, z: usize) -> f32 {
        self.z_range.0 + (z as f32 * self.delta.2)
    }

    fn cube_index(&self, x: usize, y: usize, z: usize) -> u8 {
        let mut idx = 0u8;

        let v = [
            self.get_voxel(x, y, z),
            self.get_voxel(x + 1, y, z),
            self.get_voxel(x + 1, y + 1, z),
            self.get_voxel(x, y + 1, z),
            self.get_voxel(x, y, z + 1),
            self.get_voxel(x + 1, y, z + 1),
            self.get_voxel(x + 1, y + 1, z + 1),
            self.get_voxel(x, y + 1, z + 1),
        ];

        for i in 0..8 {
            if v[i] {
                idx |= 1 << i;
            }
        }

        idx
    }

    fn populate_from_fn(&mut self, sdf: SdfFn, isovalue: f32) {
        for k in 0..self.z_len {
            for j in 0..self.y_len {
                for i in 0..self.x_len {
                    let val = sdf(self.x_value(i), self.y_value(j), self.z_value(k));
                    self.set_voxel(i, j, k, val >= isovalue);
                }
            }
        }
    }

    fn populate_from_scalar_field(&mut self, field: &ScalarField, isovalue: f32) {
        for k in 0..self.z_len {
            for j in 0..self.y_len {
                for i in 0..self.x_len {
                    let val = field[(i, j, k)];
                    self.set_voxel(i, j, k, val >= isovalue);
                }
            }
        }
    }
}

impl Grid {
    pub fn generate_mesh_from_fn(&mut self, sdf: SdfFn, isovalue: f32) -> MeshData {
        self.populate_from_fn(sdf, isovalue);

        let mut mesh = MeshData::new();

        let mut vertex_cache: HashMap<([usize; 3], [usize; 3]), u32> = HashMap::new();

        for k in 0..self.z_len - 1 {
            for j in 0..self.y_len - 1 {
                for i in 0..self.x_len - 1 {
                    let cube_idx = self.cube_index(i, j, k) as usize;
                    let edges = EDGE_TABLE[cube_idx];

                    if edges == 0 {
                        continue;
                    }

                    let mut active_edge_indices = [0u32; 12];

                    for e in 0..12 {
                        if (edges & (1 << e)) != 0 {
                            let (start, end) = EDGE_VERTS[e];

                            let v1_coord = [
                                i + VERT_OFFSET[start][0],
                                j + VERT_OFFSET[start][1],
                                k + VERT_OFFSET[start][2],
                            ];
                            let v2_coord = [
                                i + VERT_OFFSET[end][0],
                                j + VERT_OFFSET[end][1],
                                k + VERT_OFFSET[end][2],
                            ];

                            let edge_key = if v1_coord < v2_coord {
                                (v1_coord, v2_coord)
                            } else {
                                (v2_coord, v1_coord)
                            };

                            if let Some(&idx) = vertex_cache.get(&edge_key) {
                                active_edge_indices[e] = idx;
                            } else {
                                let p1 = Vec3A::new(
                                    self.x_value(v1_coord[0]),
                                    self.y_value(v1_coord[1]),
                                    self.z_value(v1_coord[2]),
                                );
                                let p2 = Vec3A::new(
                                    self.x_value(v2_coord[0]),
                                    self.y_value(v2_coord[1]),
                                    self.z_value(v2_coord[2]),
                                );

                                let vert_position = interpolate_vertex(
                                    isovalue,
                                    p1,
                                    p2,
                                    sdf(p1.x, p1.y, p1.z),
                                    sdf(p2.x, p2.y, p2.z),
                                );
                                let normal = calculate_normal(sdf, vert_position);

                                let new_idx = mesh.vertices.len() as u32;
                                let vertex = Vertex::new(vert_position, normal);
                                mesh.vertices.push(vertex);

                                vertex_cache.insert(edge_key, new_idx);
                                active_edge_indices[e] = new_idx;
                            }
                        }
                    }

                    let triangles = &TRI_TABLE[cube_idx];
                    let mut tri_idx = 0;

                    while triangles[tri_idx] != -1 {
                        mesh.indices
                            .push(active_edge_indices[triangles[tri_idx] as usize]);
                        mesh.indices
                            .push(active_edge_indices[triangles[tri_idx + 1] as usize]);
                        mesh.indices
                            .push(active_edge_indices[triangles[tri_idx + 2] as usize]);
                        tri_idx += 3;
                    }
                }
            }
        }

        mesh
    }

    pub fn generate_mesh_from_scalar_field(
        &mut self,
        field: ScalarField,
        isovalue: f32,
    ) -> MeshData {
        self.populate_from_scalar_field(&field, isovalue);

        let mut mesh = MeshData::new();

        let mut vertex_cache: HashMap<([usize; 3], [usize; 3]), u32> = HashMap::new();

        for k in 0..self.z_len - 1 {
            for j in 0..self.y_len - 1 {
                for i in 0..self.x_len - 1 {
                    let cube_idx = self.cube_index(i, j, k) as usize;
                    let edges = EDGE_TABLE[cube_idx];

                    if edges == 0 {
                        continue;
                    }

                    let mut active_edge_indices = [0u32; 12];

                    for e in 0..12 {
                        if (edges & (1 << e)) != 0 {
                            let (start, end) = EDGE_VERTS[e];

                            let v1_coord = [
                                i + VERT_OFFSET[start][0],
                                j + VERT_OFFSET[start][1],
                                k + VERT_OFFSET[start][2],
                            ];
                            let v2_coord = [
                                i + VERT_OFFSET[end][0],
                                j + VERT_OFFSET[end][1],
                                k + VERT_OFFSET[end][2],
                            ];

                            let edge_key = if v1_coord < v2_coord {
                                (v1_coord, v2_coord)
                            } else {
                                (v2_coord, v1_coord)
                            };

                            if let Some(&idx) = vertex_cache.get(&edge_key) {
                                active_edge_indices[e] = idx;
                            } else {
                                let p1 = Vec3A::new(
                                    self.x_value(v1_coord[0]),
                                    self.y_value(v1_coord[1]),
                                    self.z_value(v1_coord[2]),
                                );
                                let p2 = Vec3A::new(
                                    self.x_value(v2_coord[0]),
                                    self.y_value(v2_coord[1]),
                                    self.z_value(v2_coord[2]),
                                );

                                let value_p1 = field[v1_coord.into()];
                                let value_p2 = field[v2_coord.into()];

                                let vert_position =
                                    interpolate_vertex(isovalue, p1, p2, value_p1, value_p2);
                                let normal = calculate_normal_scalar_field(
                                    &field,
                                    v1_coord.into(),
                                    self.delta,
                                );

                                let new_idx = mesh.vertices.len() as u32;
                                let vertex = Vertex::new(vert_position, normal);
                                mesh.vertices.push(vertex);

                                vertex_cache.insert(edge_key, new_idx);
                                active_edge_indices[e] = new_idx;
                            }
                        }
                    }

                    let triangles = &TRI_TABLE[cube_idx];
                    let mut tri_idx = 0;

                    while triangles[tri_idx] != -1 {
                        mesh.indices
                            .push(active_edge_indices[triangles[tri_idx] as usize]);
                        mesh.indices
                            .push(active_edge_indices[triangles[tri_idx + 1] as usize]);
                        mesh.indices
                            .push(active_edge_indices[triangles[tri_idx + 2] as usize]);
                        tri_idx += 3;
                    }
                }
            }
        }

        mesh
    }
}

#[inline]
fn interpolate_vertex(isovalue: f32, p1: Vec3A, p2: Vec3A, value_p1: f32, value_p2: f32) -> Vec3A {
    const EPS: f32 = 1e-6;
    if (value_p2 - value_p1).abs() < EPS {
        return p1;
    }
    let mu = (isovalue - value_p1) / (value_p2 - value_p1);
    p1 + mu * (p2 - p1)
}

#[inline]
fn calculate_normal(sdf: SdfFn, p: Vec3A) -> Vec3A {
    const EPS: f32 = 0.001; // A small delta for the finite difference

    let nx = sdf(p.x + EPS, p.y, p.z) - sdf(p.x - EPS, p.y, p.z);
    let ny = sdf(p.x, p.y + EPS, p.z) - sdf(p.x, p.y - EPS, p.z);
    let nz = sdf(p.x, p.y, p.z + EPS) - sdf(p.x, p.y, p.z - EPS);

    Vec3A::new(nx, ny, nz).normalize_or_zero()
}

#[inline]
fn calculate_normal_scalar_field(
    field: &ScalarField,
    index: (usize, usize, usize),
    delta: (f32, f32, f32),
) -> Vec3A {
    let index_x1 = (index.0.saturating_add(1), index.1, index.2);
    let index_x2 = (index.0.saturating_sub(1), index.1, index.2);
    let index_y1 = (index.0, index.1.saturating_add(1), index.2);
    let index_y2 = (index.0, index.1.saturating_sub(1), index.2);
    let index_z1 = (index.0, index.1, index.2.saturating_add(1));
    let index_z2 = (index.0, index.1, index.2.saturating_sub(1));

    let nx = (field[index_x1] - field[index_x2]) / delta.0;
    let ny = (field[index_y1] - field[index_y2]) / delta.1;
    let nz = (field[index_z1] - field[index_z2]) / delta.2;

    Vec3A::new(nx, ny, nz).normalize_or_zero()
}
