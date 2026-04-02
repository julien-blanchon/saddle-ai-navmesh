use bevy::{
    mesh::{Indices, VertexAttributeValues},
    prelude::*,
};

use crate::{
    config::{NavmeshArea, NavmeshAreaMask},
    path::NavmeshOffMeshLink,
};

#[derive(Debug, Clone, PartialEq, Reflect, Default)]
#[reflect(Default)]
pub struct NavmeshTriangleSoup {
    pub vertices: Vec<Vec3>,
    pub indices: Vec<[u32; 3]>,
}

impl NavmeshTriangleSoup {
    pub fn transformed(&self, transform: Transform) -> Self {
        let vertices = self
            .vertices
            .iter()
            .map(|vertex| transform.transform_point(*vertex))
            .collect();
        Self {
            vertices,
            indices: self.indices.clone(),
        }
    }

    pub fn aabb(&self) -> Option<(Vec3, Vec3)> {
        let mut iter = self.vertices.iter().copied();
        let first = iter.next()?;
        let mut min = first;
        let mut max = first;
        for vertex in iter {
            min = min.min(vertex);
            max = max.max(vertex);
        }
        Some((min, max))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Reflect, Default)]
pub enum NavmeshSourceKind {
    #[default]
    Walkable,
    Obstacle,
    Ignore,
}

#[derive(Debug, Clone, PartialEq, Reflect)]
#[reflect(Default)]
pub enum NavmeshPrimitive {
    Quad { size: Vec2 },
    Cuboid { size: Vec3 },
    Ramp { size: Vec3 },
    Disc { radius: f32, sides: u8 },
    ConvexPrism { footprint: Vec<Vec2>, height: f32 },
}

impl Default for NavmeshPrimitive {
    fn default() -> Self {
        Self::Quad {
            size: Vec2::splat(1.0),
        }
    }
}

impl NavmeshPrimitive {
    pub fn triangles(&self) -> NavmeshTriangleSoup {
        match self {
            NavmeshPrimitive::Quad { size } => {
                let hx = size.x * 0.5;
                let hz = size.y * 0.5;
                NavmeshTriangleSoup {
                    vertices: vec![
                        Vec3::new(-hx, 0.0, -hz),
                        Vec3::new(hx, 0.0, -hz),
                        Vec3::new(hx, 0.0, hz),
                        Vec3::new(-hx, 0.0, hz),
                    ],
                    indices: vec![[0, 1, 2], [0, 2, 3]],
                }
            }
            NavmeshPrimitive::Cuboid { size } => {
                let hx = size.x * 0.5;
                let hy = size.y * 0.5;
                let hz = size.z * 0.5;
                let vertices = vec![
                    Vec3::new(-hx, -hy, -hz),
                    Vec3::new(hx, -hy, -hz),
                    Vec3::new(hx, -hy, hz),
                    Vec3::new(-hx, -hy, hz),
                    Vec3::new(-hx, hy, -hz),
                    Vec3::new(hx, hy, -hz),
                    Vec3::new(hx, hy, hz),
                    Vec3::new(-hx, hy, hz),
                ];
                let indices = vec![
                    [0, 2, 1],
                    [0, 3, 2],
                    [4, 5, 6],
                    [4, 6, 7],
                    [0, 1, 5],
                    [0, 5, 4],
                    [1, 2, 6],
                    [1, 6, 5],
                    [2, 3, 7],
                    [2, 7, 6],
                    [3, 0, 4],
                    [3, 4, 7],
                ];
                NavmeshTriangleSoup { vertices, indices }
            }
            NavmeshPrimitive::Ramp { size } => {
                let hx = size.x * 0.5;
                let hy = size.y * 0.5;
                let hz = size.z * 0.5;
                let vertices = vec![
                    Vec3::new(-hx, -hy, -hz),
                    Vec3::new(hx, -hy, -hz),
                    Vec3::new(hx, -hy, hz),
                    Vec3::new(-hx, -hy, hz),
                    Vec3::new(-hx, hy, -hz),
                    Vec3::new(hx, hy, -hz),
                ];
                let indices = vec![
                    [0, 2, 1],
                    [0, 3, 2],
                    [0, 1, 5],
                    [0, 5, 4],
                    [1, 2, 5],
                    [2, 3, 4],
                    [2, 4, 5],
                    [3, 0, 4],
                ];
                NavmeshTriangleSoup { vertices, indices }
            }
            NavmeshPrimitive::Disc { radius, sides } => {
                let sides = (*sides).max(3);
                let mut vertices = vec![Vec3::ZERO];
                let mut indices = Vec::new();
                for side in 0..sides {
                    let angle = side as f32 / sides as f32 * std::f32::consts::TAU;
                    vertices.push(Vec3::new(angle.cos() * *radius, 0.0, angle.sin() * *radius));
                }
                for side in 1..sides {
                    indices.push([0, side as u32, side as u32 + 1]);
                }
                indices.push([0, sides as u32, 1]);
                NavmeshTriangleSoup { vertices, indices }
            }
            NavmeshPrimitive::ConvexPrism { footprint, height } => {
                if footprint.len() < 3 {
                    return NavmeshTriangleSoup::default();
                }

                let mut vertices = Vec::with_capacity(footprint.len() * 2);
                let top_y = *height * 0.5;
                let bottom_y = -*height * 0.5;
                for point in footprint {
                    vertices.push(Vec3::new(point.x, bottom_y, point.y));
                }
                for point in footprint {
                    vertices.push(Vec3::new(point.x, top_y, point.y));
                }

                let mut indices = Vec::new();
                for index in 1..(footprint.len() - 1) {
                    indices.push([0, index as u32 + 1, index as u32]);
                    indices.push([
                        footprint.len() as u32,
                        footprint.len() as u32 + index as u32,
                        footprint.len() as u32 + index as u32 + 1,
                    ]);
                }

                for index in 0..footprint.len() {
                    let next = (index + 1) % footprint.len();
                    let bottom_a = index as u32;
                    let bottom_b = next as u32;
                    let top_a = (footprint.len() + index) as u32;
                    let top_b = (footprint.len() + next) as u32;
                    indices.push([bottom_a, bottom_b, top_b]);
                    indices.push([bottom_a, top_b, top_a]);
                }

                NavmeshTriangleSoup { vertices, indices }
            }
        }
    }
}

#[derive(Component, Debug, Clone, PartialEq, Reflect)]
#[reflect(Component, Default)]
pub struct NavmeshPrimitiveSource {
    pub primitive: NavmeshPrimitive,
}

impl NavmeshPrimitiveSource {
    pub fn new(primitive: NavmeshPrimitive) -> Self {
        Self { primitive }
    }

    pub fn triangles(&self) -> NavmeshTriangleSoup {
        self.primitive.triangles()
    }
}

impl Default for NavmeshPrimitiveSource {
    fn default() -> Self {
        Self::new(NavmeshPrimitive::default())
    }
}

#[derive(Debug, Clone, PartialEq, Reflect)]
#[reflect(Default)]
pub struct NavmeshSourceGeometry {
    pub source_id: u64,
    pub kind: NavmeshSourceKind,
    pub area: NavmeshArea,
    pub mask: NavmeshAreaMask,
    pub triangles: NavmeshTriangleSoup,
}

impl Default for NavmeshSourceGeometry {
    fn default() -> Self {
        Self {
            source_id: 0,
            kind: NavmeshSourceKind::Walkable,
            area: NavmeshArea::default(),
            mask: NavmeshAreaMask::all(),
            triangles: NavmeshTriangleSoup::default(),
        }
    }
}

impl NavmeshSourceGeometry {
    pub fn aabb(&self) -> Option<(Vec3, Vec3)> {
        self.triangles.aabb()
    }
}

#[derive(Debug, Clone, Default)]
pub struct NavmeshBuildInput {
    pub sources: Vec<NavmeshSourceGeometry>,
    pub links: Vec<NavmeshOffMeshLink>,
}

pub fn triangle_soup_from_mesh(mesh: &Mesh) -> Option<NavmeshTriangleSoup> {
    let positions = mesh.attribute(Mesh::ATTRIBUTE_POSITION)?;
    let positions = match positions {
        VertexAttributeValues::Float32x3(values) => values
            .iter()
            .map(|value| Vec3::new(value[0], value[1], value[2]))
            .collect::<Vec<_>>(),
        _ => return None,
    };

    let indices = match mesh.indices()? {
        Indices::U16(values) => values
            .chunks_exact(3)
            .map(|tri| [tri[0] as u32, tri[1] as u32, tri[2] as u32])
            .collect(),
        Indices::U32(values) => values
            .chunks_exact(3)
            .map(|tri| [tri[0], tri[1], tri[2]])
            .collect(),
    };

    Some(NavmeshTriangleSoup {
        vertices: positions,
        indices,
    })
}
