use crate::app::{
    marching_cubes::lookup_tables::{EDGE_TABLE, TRI_TABLE},
    mesh::{Mesh, Vertex},
    scalar_field::{Dimentions3D, ScalarField},
};
use glam::Vec3A;

const EDGE_MAPPING: [(usize, usize, usize, usize); 12] = [
    (0, 0, 0, 0),
    (1, 0, 0, 1),
    (0, 1, 0, 0),
    (0, 0, 0, 1),
    (0, 0, 1, 0),
    (1, 0, 1, 1),
    (0, 1, 1, 0),
    (0, 0, 1, 1),
    (0, 0, 0, 2),
    (1, 0, 0, 2),
    (1, 1, 0, 2),
    (0, 1, 0, 2),
];

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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum GenerationMethod {
    /// Generates only the interior mesh based strictly on the grid data.
    #[default]
    Standard,
    /// Adds virtual padding to cap the open boundary faces of the volume.
    WithBorders,
}

pub struct MarchingCubes;

impl MarchingCubes {
    pub fn generate_mesh(
        sdf: &ScalarField,
        isovalue: f32,
        method: GenerationMethod,
        mesh: &mut Mesh,
    ) {
        mesh.clear();
        match method {
            GenerationMethod::Standard => Self::generate_standard(sdf, isovalue, mesh),
            GenerationMethod::WithBorders => Self::generate_with_borders(sdf, isovalue, mesh),
        }
    }

    #[inline]
    fn get_edge_index(nx: usize, ny: usize, x: usize, y: usize, z: usize, dir: usize) -> usize {
        ((z * ny + y) * nx + x) * 3 + dir
    }

    #[inline]
    fn cube_index_standard(sdf: &ScalarField, isovalue: f32, x: usize, y: usize, z: usize) -> u8 {
        let mut idx = 0u8;
        let v = [
            sdf[(x, y, z)] >= isovalue,
            sdf[(x + 1, y, z)] >= isovalue,
            sdf[(x + 1, y + 1, z)] >= isovalue,
            sdf[(x, y + 1, z)] >= isovalue,
            sdf[(x, y, z + 1)] >= isovalue,
            sdf[(x + 1, y, z + 1)] >= isovalue,
            sdf[(x + 1, y + 1, z + 1)] >= isovalue,
            sdf[(x, y + 1, z + 1)] >= isovalue,
        ];

        for i in 0..8 {
            if v[i] {
                idx |= 1 << i;
            }
        }
        idx
    }

    fn generate_standard(sdf: &ScalarField, isovalue: f32, mesh: &mut Mesh) {
        let Dimentions3D {
            x: nx,
            y: ny,
            z: nz,
        } = sdf.dims;
        let total_edges = nx * ny * nz * 3;
        let mut vertex_cache = vec![u32::MAX; total_edges];

        for k in 0..nz - 1 {
            for j in 0..ny - 1 {
                for i in 0..nx - 1 {
                    let cube_idx = Self::cube_index_standard(sdf, isovalue, i, j, k) as usize;
                    let edges = EDGE_TABLE[cube_idx];

                    if edges == 0 {
                        continue;
                    }

                    let mut active_edge_indices = [0u32; 12];

                    for e in 0..12 {
                        if (edges & (1 << e)) != 0 {
                            let (ox, oy, oz, dir) = EDGE_MAPPING[e];
                            let edge_x = i + ox;
                            let edge_y = j + oy;
                            let edge_z = k + oz;

                            let cache_idx =
                                Self::get_edge_index(nx, ny, edge_x, edge_y, edge_z, dir);

                            if vertex_cache[cache_idx] != u32::MAX {
                                active_edge_indices[e] = vertex_cache[cache_idx];
                            } else {
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

                                let p1 = Vec3A::from_array(v1_coord.map(|x| x as f32));
                                let p2 = Vec3A::from_array(v2_coord.map(|x| x as f32));

                                let value_p1 = sdf[v1_coord.into()];
                                let value_p2 = sdf[v2_coord.into()];

                                let (vert_position, mu) =
                                    interpolate_vertex(isovalue, p1, p2, value_p1, value_p2);

                                let n1 = Vec3A::from_array(calculate_normal(
                                    sdf,
                                    v1_coord[0],
                                    v1_coord[1],
                                    v1_coord[2],
                                ));
                                let n2 = Vec3A::from_array(calculate_normal(
                                    sdf,
                                    v2_coord[0],
                                    v2_coord[1],
                                    v2_coord[2],
                                ));

                                let normal = (n1 + mu * (n2 - n1)).normalize().into();

                                let new_idx = mesh.vertices.len() as u32;
                                mesh.vertices
                                    .push(Vertex::new(vert_position.into(), normal));

                                vertex_cache[cache_idx] = new_idx;
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
    }

    #[inline]
    fn is_out_of_bounds(sdf: &ScalarField, x: isize, y: isize, z: isize) -> bool {
        let Dimentions3D {
            x: nx,
            y: ny,
            z: nz,
        } = sdf.dims;
        x < 0 || y < 0 || z < 0 || x >= nx as isize || y >= ny as isize || z >= nz as isize
    }

    #[inline]
    fn get_density_bordered(sdf: &ScalarField, x: isize, y: isize, z: isize, isovalue: f32) -> f32 {
        if Self::is_out_of_bounds(sdf, x, y, z) {
            isovalue - 1000.0
        } else {
            sdf[(x as usize, y as usize, z as usize)]
        }
    }

    #[inline]
    fn cube_index_bordered(sdf: &ScalarField, isovalue: f32, x: isize, y: isize, z: isize) -> u8 {
        let mut idx = 0u8;
        let v = [
            Self::get_density_bordered(sdf, x, y, z, isovalue) >= isovalue,
            Self::get_density_bordered(sdf, x + 1, y, z, isovalue) >= isovalue,
            Self::get_density_bordered(sdf, x + 1, y + 1, z, isovalue) >= isovalue,
            Self::get_density_bordered(sdf, x, y + 1, z, isovalue) >= isovalue,
            Self::get_density_bordered(sdf, x, y, z + 1, isovalue) >= isovalue,
            Self::get_density_bordered(sdf, x + 1, y, z + 1, isovalue) >= isovalue,
            Self::get_density_bordered(sdf, x + 1, y + 1, z + 1, isovalue) >= isovalue,
            Self::get_density_bordered(sdf, x, y + 1, z + 1, isovalue) >= isovalue,
        ];

        for i in 0..8 {
            if v[i] {
                idx |= 1 << i;
            }
        }
        idx
    }

    fn generate_with_borders(sdf: &ScalarField, isovalue: f32, mesh: &mut Mesh) {
        let Dimentions3D {
            x: nx,
            y: ny,
            z: nz,
        } = sdf.dims;

        let cache_nx = nx + 2;
        let cache_ny = ny + 2;
        let cache_nz = nz + 2;
        let total_edges = cache_nx * cache_ny * cache_nz * 3;
        let mut vertex_cache = vec![u32::MAX; total_edges];

        for k in 0..=nz {
            for j in 0..=ny {
                for i in 0..=nx {
                    let x_idx = i as isize - 1;
                    let y_idx = j as isize - 1;
                    let z_idx = k as isize - 1;

                    let cube_idx =
                        Self::cube_index_bordered(sdf, isovalue, x_idx, y_idx, z_idx) as usize;
                    let edges = EDGE_TABLE[cube_idx];

                    if edges == 0 {
                        continue;
                    }

                    let mut active_edge_indices = [0u32; 12];

                    for e in 0..12 {
                        if (edges & (1 << e)) != 0 {
                            let (ox, oy, oz, dir) = EDGE_MAPPING[e];
                            let edge_x = x_idx + ox as isize;
                            let edge_y = y_idx + oy as isize;
                            let edge_z = z_idx + oz as isize;

                            let cache_x = (edge_x + 1) as usize;
                            let cache_y = (edge_y + 1) as usize;
                            let cache_z = (edge_z + 1) as usize;

                            let cache_idx = Self::get_edge_index(
                                cache_nx, cache_ny, cache_x, cache_y, cache_z, dir,
                            );

                            if vertex_cache[cache_idx] != u32::MAX {
                                active_edge_indices[e] = vertex_cache[cache_idx];
                            } else {
                                let (start, end) = EDGE_VERTS[e];

                                let v1_x = x_idx + VERT_OFFSET[start][0] as isize;
                                let v1_y = y_idx + VERT_OFFSET[start][1] as isize;
                                let v1_z = z_idx + VERT_OFFSET[start][2] as isize;

                                let v2_x = x_idx + VERT_OFFSET[end][0] as isize;
                                let v2_y = y_idx + VERT_OFFSET[end][1] as isize;
                                let v2_z = z_idx + VERT_OFFSET[end][2] as isize;

                                let p1 = Vec3A::new(v1_x as f32, v1_y as f32, v1_z as f32);
                                let p2 = Vec3A::new(v2_x as f32, v2_y as f32, v2_z as f32);

                                let oob1 = Self::is_out_of_bounds(sdf, v1_x, v1_y, v1_z);
                                let oob2 = Self::is_out_of_bounds(sdf, v2_x, v2_y, v2_z);

                                let (vert_position, normal) = if oob1 && !oob2 {
                                    (p2, (p1 - p2).normalize())
                                } else if !oob1 && oob2 {
                                    (p1, (p2 - p1).normalize())
                                } else {
                                    let value_p1 =
                                        sdf[(v1_x as usize, v1_y as usize, v1_z as usize)];
                                    let value_p2 =
                                        sdf[(v2_x as usize, v2_y as usize, v2_z as usize)];
                                    let (pos, mu) =
                                        interpolate_vertex(isovalue, p1, p2, value_p1, value_p2);

                                    let n1 = Vec3A::from_array(calculate_normal(
                                        sdf,
                                        v1_x as usize,
                                        v1_y as usize,
                                        v1_z as usize,
                                    ));
                                    let n2 = Vec3A::from_array(calculate_normal(
                                        sdf,
                                        v2_x as usize,
                                        v2_y as usize,
                                        v2_z as usize,
                                    ));
                                    let norm = (n1 + mu * (n2 - n1)).normalize();

                                    (pos, norm)
                                };

                                let new_idx = mesh.vertices.len() as u32;
                                mesh.vertices
                                    .push(Vertex::new(vert_position.into(), normal.into()));

                                vertex_cache[cache_idx] = new_idx;
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
    }
}

#[inline]
fn interpolate_vertex(
    isovalue: f32,
    p1: Vec3A,
    p2: Vec3A,
    value_p1: f32,
    value_p2: f32,
) -> (Vec3A, f32) {
    const EPS: f32 = 1e-6;
    if (value_p2 - value_p1).abs() < EPS {
        return (p1, 0.0);
    }
    let mu = (isovalue - value_p1) / (value_p2 - value_p1);
    (p1 + mu * (p2 - p1), mu)
}

#[inline]
fn calculate_normal(sdf: &ScalarField, x: usize, y: usize, z: usize) -> [f32; 3] {
    let Dimentions3D {
        x: nx,
        y: ny,
        z: nz,
    } = sdf.dims;

    let x1 = (x + 1).min(nx - 1);
    let x2 = x.saturating_sub(1);
    let y1 = (y + 1).min(ny - 1);
    let y2 = y.saturating_sub(1);
    let z1 = (z + 1).min(nz - 1);
    let z2 = z.saturating_sub(1);

    let nx = sdf[(x1, y, z)] - sdf[(x2, y, z)];
    let ny = sdf[(x, y1, z)] - sdf[(x, y2, z)];
    let nz = sdf[(x, y, z1)] - sdf[(x, y, z2)];

    [nx, ny, nz]
}
