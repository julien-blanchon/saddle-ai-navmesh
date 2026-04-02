use std::{collections::HashMap, time::Instant};

use bevy::prelude::*;

use crate::{
    config::{NavmeshArea, NavmeshAreaMask, NavmeshBakeSettings, NavmeshQueryFilter},
    geometry::{NavmeshBuildInput, NavmeshSourceGeometry, NavmeshSourceKind},
    math::{
        NavmeshBasis, convex_hull, point_in_triangle_2d, point_near_polygon, quantize_vec2,
        segments_intersect_2d, triangle_normal,
    },
    path::{NavmeshLinkId, NavmeshProjectionHit},
};

#[derive(Debug, Clone, PartialEq, Reflect)]
#[reflect(Default)]
pub struct NavmeshBasisData {
    pub right: Vec3,
    pub forward: Vec3,
    pub up: Vec3,
}

impl Default for NavmeshBasisData {
    fn default() -> Self {
        Self {
            right: Vec3::X,
            forward: Vec3::Z,
            up: Vec3::Y,
        }
    }
}

impl From<NavmeshBasis> for NavmeshBasisData {
    fn from(value: NavmeshBasis) -> Self {
        Self {
            right: value.right,
            forward: value.forward,
            up: value.up,
        }
    }
}

impl NavmeshBasisData {
    pub fn basis(&self) -> NavmeshBasis {
        NavmeshBasis {
            right: self.right,
            forward: self.forward,
            up: self.up,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Reflect)]
#[reflect(Default)]
pub struct NavmeshPortal {
    pub id: u32,
    pub polygons: [u32; 2],
    pub edge: [Vec3; 2],
}

impl Default for NavmeshPortal {
    fn default() -> Self {
        Self {
            id: 0,
            polygons: [0, 0],
            edge: [Vec3::ZERO, Vec3::ZERO],
        }
    }
}

#[derive(Debug, Clone, PartialEq, Reflect)]
#[reflect(Default)]
pub struct NavmeshPolygon {
    pub id: u32,
    pub vertices: [u32; 3],
    pub area: NavmeshArea,
    pub mask: NavmeshAreaMask,
    pub centroid: Vec3,
    pub normal: Vec3,
    pub source_id: u64,
    pub portal_indices: Vec<u32>,
}

impl Default for NavmeshPolygon {
    fn default() -> Self {
        Self {
            id: 0,
            vertices: [0, 0, 0],
            area: NavmeshArea::default(),
            mask: NavmeshAreaMask::all(),
            centroid: Vec3::ZERO,
            normal: Vec3::Y,
            source_id: 0,
            portal_indices: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Reflect)]
#[reflect(Default)]
pub struct NavmeshBakedLink {
    pub id: NavmeshLinkId,
    pub from_polygon: u32,
    pub to_polygon: u32,
    pub start: Vec3,
    pub end: Vec3,
    pub bidirectional: bool,
    pub mask: NavmeshAreaMask,
    pub cost_multiplier: f32,
}

impl Default for NavmeshBakedLink {
    fn default() -> Self {
        Self {
            id: NavmeshLinkId::default(),
            from_polygon: 0,
            to_polygon: 0,
            start: Vec3::ZERO,
            end: Vec3::ZERO,
            bidirectional: true,
            mask: NavmeshAreaMask::all(),
            cost_multiplier: 1.0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Reflect)]
#[reflect(Default)]
pub struct NavmeshBakeStats {
    pub source_count: u32,
    pub walkable_triangle_count: u32,
    pub obstacle_count: u32,
    pub polygon_count: u32,
    pub portal_count: u32,
    pub link_count: u32,
    pub last_bake_ms: f32,
    pub bounds_min: Vec3,
    pub bounds_max: Vec3,
}

impl Default for NavmeshBakeStats {
    fn default() -> Self {
        Self {
            source_count: 0,
            walkable_triangle_count: 0,
            obstacle_count: 0,
            polygon_count: 0,
            portal_count: 0,
            link_count: 0,
            last_bake_ms: 0.0,
            bounds_min: Vec3::ZERO,
            bounds_max: Vec3::ZERO,
        }
    }
}

#[derive(Component, Debug, Clone, PartialEq, Reflect, Default)]
#[reflect(Component, Default)]
pub struct NavmeshSurfaceData {
    pub generation: u64,
    pub basis: NavmeshBasisData,
    pub vertices: Vec<Vec3>,
    pub polygons: Vec<NavmeshPolygon>,
    pub portals: Vec<NavmeshPortal>,
    pub links: Vec<NavmeshBakedLink>,
    pub stats: NavmeshBakeStats,
}

impl NavmeshSurfaceData {
    pub fn polygon_vertices(&self, polygon_index: u32) -> Option<[Vec3; 3]> {
        let polygon = self.polygons.get(polygon_index as usize)?;
        Some([
            *self.vertices.get(polygon.vertices[0] as usize)?,
            *self.vertices.get(polygon.vertices[1] as usize)?,
            *self.vertices.get(polygon.vertices[2] as usize)?,
        ])
    }

    pub fn projected_polygon(&self, polygon_index: u32) -> Option<[Vec2; 3]> {
        let basis = self.basis.basis();
        self.polygon_vertices(polygon_index)
            .map(|vertices| vertices.map(|vertex| basis.project(vertex)))
    }

    pub fn nearest_point(
        &self,
        point: Vec3,
        filter: &NavmeshQueryFilter,
    ) -> Option<NavmeshProjectionHit> {
        crate::query::nearest_point_on_navmesh(self, point, filter)
    }

    pub fn query_path(
        &self,
        request_id: crate::path::NavmeshPathId,
        start: Vec3,
        goal: Vec3,
        settings: &crate::config::NavmeshQuerySettings,
        filter: &NavmeshQueryFilter,
    ) -> crate::path::NavmeshPathQueryResult {
        crate::query::query_navmesh_path(self, request_id, start, goal, settings, filter)
    }

    pub fn line_of_sight(&self, start: Vec3, goal: Vec3, filter: &NavmeshQueryFilter) -> bool {
        crate::query::line_of_sight(self, start, goal, filter)
    }
}

#[derive(Debug, Clone)]
pub struct NavmeshBakeError {
    pub message: String,
}

impl NavmeshBakeError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl std::fmt::Display for NavmeshBakeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for NavmeshBakeError {}

pub fn bake_navmesh(
    input: &NavmeshBuildInput,
    settings: &NavmeshBakeSettings,
) -> Result<NavmeshSurfaceData, NavmeshBakeError> {
    bake_navmesh_with_generation(input, settings, 0)
}

pub(crate) fn bake_navmesh_with_generation(
    input: &NavmeshBuildInput,
    settings: &NavmeshBakeSettings,
    generation: u64,
) -> Result<NavmeshSurfaceData, NavmeshBakeError> {
    let started = Instant::now();
    let basis = NavmeshBasis::from_up(settings.up);
    let slope_cos = settings.max_slope_degrees.to_radians().cos();

    let obstacle_footprints = build_obstacle_footprints(&input.sources, basis);
    let mut bounds: Option<(Vec3, Vec3)> = None;
    let mut vertices = Vec::new();
    let mut polygons = Vec::new();
    let mut walkable_triangle_count = 0_u32;
    let mut obstacle_count = 0_u32;

    for source in &input.sources {
        if let Some((min, max)) = source.aabb() {
            bounds = Some(match bounds {
                Some((current_min, current_max)) => (current_min.min(min), current_max.max(max)),
                None => (min, max),
            });
        }

        if source.kind == NavmeshSourceKind::Obstacle {
            obstacle_count += 1;
            continue;
        }
        if source.kind != NavmeshSourceKind::Walkable {
            continue;
        }

        for triangle in &source.triangles.indices {
            let a = source.triangles.vertices[triangle[0] as usize];
            let b = source.triangles.vertices[triangle[1] as usize];
            let c = source.triangles.vertices[triangle[2] as usize];
            let mut normal = triangle_normal(a, b, c);
            if normal.length_squared() <= f32::EPSILON {
                continue;
            }
            let alignment = normal.dot(basis.up);
            if alignment.abs() < slope_cos {
                continue;
            }
            if alignment < 0.0 {
                normal = -normal;
            }

            let projected = [basis.project(a), basis.project(b), basis.project(c)];
            if obstacle_footprints.iter().any(|footprint| {
                triangle_overlaps_footprint(projected, footprint, settings.agent_radius)
            }) {
                continue;
            }

            let centroid = (a + b + c) / 3.0;
            let base = vertices.len() as u32;
            vertices.extend([a, b, c]);
            polygons.push(NavmeshPolygon {
                id: polygons.len() as u32,
                vertices: [base, base + 1, base + 2],
                area: source.area,
                mask: source.mask,
                centroid,
                normal,
                source_id: source.source_id,
                portal_indices: Vec::new(),
            });
            walkable_triangle_count += 1;
        }
    }

    if polygons.is_empty() {
        return Err(NavmeshBakeError::new(
            "navmesh bake produced no walkable polygons",
        ));
    }

    let mut portals = Vec::new();
    let mut edge_map: HashMap<((i64, i64), (i64, i64)), (u32, [Vec3; 2])> = HashMap::new();
    for polygon_index in 0..polygons.len() {
        let polygon = polygons[polygon_index].clone();
        let polygon_vertices = [
            vertices[polygon.vertices[0] as usize],
            vertices[polygon.vertices[1] as usize],
            vertices[polygon.vertices[2] as usize],
        ];
        for edge in 0..3 {
            let start = polygon_vertices[edge];
            let end = polygon_vertices[(edge + 1) % 3];
            let start_2d = quantize_vec2(basis.project(start), settings.quantization);
            let end_2d = quantize_vec2(basis.project(end), settings.quantization);
            let key = if start_2d <= end_2d {
                (start_2d, end_2d)
            } else {
                (end_2d, start_2d)
            };

            if let Some((other_polygon, other_edge)) = edge_map.remove(&key) {
                let other_midpoint = (other_edge[0] + other_edge[1]) * 0.5;
                let midpoint = (start + end) * 0.5;
                let midpoint_delta = other_midpoint.distance(midpoint);
                if midpoint_delta <= settings.max_step_height.max(settings.quantization) {
                    let portal_id = portals.len() as u32;
                    portals.push(NavmeshPortal {
                        id: portal_id,
                        polygons: [other_polygon, polygon.id],
                        edge: [start, end],
                    });
                    polygons[other_polygon as usize]
                        .portal_indices
                        .push(portal_id);
                    polygons[polygon.id as usize].portal_indices.push(portal_id);
                }
            } else {
                edge_map.insert(key, (polygon.id, [start, end]));
            }
        }
    }

    let mut surface = NavmeshSurfaceData {
        generation,
        basis: basis.into(),
        vertices,
        polygons,
        portals,
        links: Vec::new(),
        stats: NavmeshBakeStats {
            source_count: input.sources.len() as u32,
            walkable_triangle_count,
            obstacle_count,
            polygon_count: 0,
            portal_count: 0,
            link_count: 0,
            last_bake_ms: 0.0,
            bounds_min: bounds.map(|(min, _)| min).unwrap_or(Vec3::ZERO),
            bounds_max: bounds.map(|(_, max)| max).unwrap_or(Vec3::ZERO),
        },
    };

    let mut baked_links = Vec::new();
    for (index, link) in input.links.iter().enumerate() {
        let Some(start_hit) = surface.nearest_point(link.start, &NavmeshQueryFilter::default())
        else {
            continue;
        };
        let Some(end_hit) = surface.nearest_point(link.end, &NavmeshQueryFilter::default()) else {
            continue;
        };
        baked_links.push(NavmeshBakedLink {
            id: NavmeshLinkId(index as u32),
            from_polygon: start_hit.polygon,
            to_polygon: end_hit.polygon,
            start: start_hit.position,
            end: end_hit.position,
            bidirectional: link.bidirectional,
            mask: link.mask,
            cost_multiplier: link.cost_multiplier.max(1.0),
        });
    }
    surface.links = baked_links;
    surface.stats.polygon_count = surface.polygons.len() as u32;
    surface.stats.portal_count = surface.portals.len() as u32;
    surface.stats.link_count = surface.links.len() as u32;
    surface.stats.last_bake_ms = started.elapsed().as_secs_f32() * 1000.0;

    Ok(surface)
}

fn build_obstacle_footprints(
    sources: &[NavmeshSourceGeometry],
    basis: NavmeshBasis,
) -> Vec<Vec<Vec2>> {
    sources
        .iter()
        .filter(|source| source.kind == NavmeshSourceKind::Obstacle)
        .filter_map(|source| {
            let points = source
                .triangles
                .vertices
                .iter()
                .map(|vertex| basis.project(*vertex))
                .collect::<Vec<_>>();
            if points.len() < 3 {
                return None;
            }
            Some(convex_hull(&points))
        })
        .collect()
}

fn triangle_overlaps_footprint(projected: [Vec2; 3], footprint: &[Vec2], radius: f32) -> bool {
    let centroid = (projected[0] + projected[1] + projected[2]) / 3.0;
    if point_near_polygon(centroid, footprint, radius) {
        return true;
    }

    if projected
        .iter()
        .copied()
        .any(|vertex| point_near_polygon(vertex, footprint, radius))
    {
        return true;
    }

    let epsilon = 0.0001_f32.max(radius * 0.01);
    if footprint
        .iter()
        .copied()
        .any(|vertex| point_in_triangle_2d(vertex, projected, epsilon))
    {
        return true;
    }

    let triangle_edges = [
        (projected[0], projected[1]),
        (projected[1], projected[2]),
        (projected[2], projected[0]),
    ];
    let mut previous = *footprint.last().unwrap_or(&Vec2::ZERO);
    for current in footprint.iter().copied() {
        if triangle_edges
            .iter()
            .any(|(start, end)| segments_intersect_2d(*start, *end, previous, current, epsilon))
        {
            return true;
        }
        previous = current;
    }

    false
}

#[cfg(test)]
#[path = "bake_tests.rs"]
mod tests;
