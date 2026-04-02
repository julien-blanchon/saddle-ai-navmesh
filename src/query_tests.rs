use bevy::prelude::*;

use super::*;
use crate::{
    bake::{NavmeshBakeStats, NavmeshBasisData, NavmeshPolygon, NavmeshPortal},
    config::{NavmeshArea, NavmeshAreaCost, NavmeshAreaMask},
    geometry::{NavmeshBuildInput, NavmeshPrimitive, NavmeshSourceGeometry, NavmeshSourceKind},
    path::{NavmeshPathId, NavmeshPathStatus},
};

fn source_geometry(
    source_id: u64,
    area: NavmeshArea,
    primitive: NavmeshPrimitive,
    transform: Transform,
) -> NavmeshSourceGeometry {
    source_geometry_with_kind(
        source_id,
        NavmeshSourceKind::Walkable,
        area,
        primitive,
        transform,
    )
}

fn source_geometry_with_kind(
    source_id: u64,
    kind: NavmeshSourceKind,
    area: NavmeshArea,
    primitive: NavmeshPrimitive,
    transform: Transform,
) -> NavmeshSourceGeometry {
    NavmeshSourceGeometry {
        source_id,
        kind,
        area,
        mask: NavmeshAreaMask::from_area(area),
        triangles: primitive.triangles().transformed(transform),
    }
}

fn baked_quad_surface() -> NavmeshSurfaceData {
    crate::bake::bake_navmesh(
        &NavmeshBuildInput {
            sources: vec![source_geometry(
                1,
                NavmeshArea(0),
                NavmeshPrimitive::Quad {
                    size: Vec2::new(4.0, 4.0),
                },
                Transform::default(),
            )],
            links: Vec::new(),
        },
        &crate::config::NavmeshBakeSettings::default(),
    )
    .unwrap()
}

fn weighted_surface() -> NavmeshSurfaceData {
    let vertices = vec![
        Vec3::new(-6.0, 0.0, -1.5),
        Vec3::new(-6.0, 0.0, 1.5),
        Vec3::new(-2.0, 0.0, 0.0),
        Vec3::new(-2.0, 0.0, 0.0),
        Vec3::new(2.0, 0.0, 2.5),
        Vec3::new(2.0, 0.0, 0.4),
        Vec3::new(-2.0, 0.0, 0.0),
        Vec3::new(2.0, 0.0, -0.4),
        Vec3::new(2.0, 0.0, -2.5),
        Vec3::new(2.0, 0.0, 0.0),
        Vec3::new(6.0, 0.0, 1.5),
        Vec3::new(6.0, 0.0, -1.5),
    ];

    let portals = vec![
        NavmeshPortal {
            id: 0,
            polygons: [0, 1],
            edge: [Vec3::new(-2.0, 0.0, 0.4), Vec3::new(-2.0, 0.0, 0.0)],
        },
        NavmeshPortal {
            id: 1,
            polygons: [0, 2],
            edge: [Vec3::new(-2.0, 0.0, 0.0), Vec3::new(-2.0, 0.0, -0.4)],
        },
        NavmeshPortal {
            id: 2,
            polygons: [1, 3],
            edge: [Vec3::new(2.0, 0.0, 0.4), Vec3::new(2.0, 0.0, 0.0)],
        },
        NavmeshPortal {
            id: 3,
            polygons: [2, 3],
            edge: [Vec3::new(2.0, 0.0, 0.0), Vec3::new(2.0, 0.0, -0.4)],
        },
    ];

    let polygons = vec![
        NavmeshPolygon {
            id: 0,
            vertices: [0, 1, 2],
            area: NavmeshArea(0),
            mask: NavmeshAreaMask::all(),
            centroid: Vec3::new(-4.67, 0.0, 0.0),
            normal: Vec3::Y,
            source_id: 1,
            portal_indices: vec![0, 1],
        },
        NavmeshPolygon {
            id: 1,
            vertices: [3, 4, 5],
            area: NavmeshArea(1),
            mask: NavmeshAreaMask::all(),
            centroid: Vec3::new(0.67, 0.0, 0.97),
            normal: Vec3::Y,
            source_id: 2,
            portal_indices: vec![0, 2],
        },
        NavmeshPolygon {
            id: 2,
            vertices: [6, 7, 8],
            area: NavmeshArea(0),
            mask: NavmeshAreaMask::all(),
            centroid: Vec3::new(0.67, 0.0, -0.97),
            normal: Vec3::Y,
            source_id: 3,
            portal_indices: vec![1, 3],
        },
        NavmeshPolygon {
            id: 3,
            vertices: [9, 10, 11],
            area: NavmeshArea(0),
            mask: NavmeshAreaMask::all(),
            centroid: Vec3::new(4.67, 0.0, 0.0),
            normal: Vec3::Y,
            source_id: 4,
            portal_indices: vec![2, 3],
        },
    ];

    NavmeshSurfaceData {
        generation: 1,
        basis: NavmeshBasisData::default(),
        vertices,
        polygons,
        portals,
        links: Vec::new(),
        stats: NavmeshBakeStats::default(),
    }
}

fn two_polygon_surface() -> NavmeshSurfaceData {
    NavmeshSurfaceData {
        generation: 1,
        basis: NavmeshBasisData::default(),
        vertices: vec![
            Vec3::new(-2.0, 0.0, -1.0),
            Vec3::new(0.0, 0.0, -1.0),
            Vec3::new(0.0, 0.0, 1.0),
            Vec3::new(2.0, 0.0, -1.0),
        ],
        polygons: vec![
            NavmeshPolygon {
                id: 0,
                vertices: [0, 1, 2],
                area: NavmeshArea(0),
                mask: NavmeshAreaMask::from_area(NavmeshArea(0)),
                centroid: Vec3::new(-0.67, 0.0, -0.33),
                normal: Vec3::Y,
                source_id: 1,
                portal_indices: vec![0],
            },
            NavmeshPolygon {
                id: 1,
                vertices: [1, 3, 2],
                area: NavmeshArea(1),
                mask: NavmeshAreaMask::from_area(NavmeshArea(1)),
                centroid: Vec3::new(0.67, 0.0, -0.33),
                normal: Vec3::Y,
                source_id: 2,
                portal_indices: vec![0],
            },
        ],
        portals: vec![NavmeshPortal {
            id: 0,
            polygons: [0, 1],
            edge: [Vec3::new(0.0, 0.0, -1.0), Vec3::new(0.0, 0.0, 1.0)],
        }],
        links: Vec::new(),
        stats: NavmeshBakeStats::default(),
    }
}

fn open_grid_surface(size: i32) -> NavmeshSurfaceData {
    let mut sources = Vec::new();
    let mut source_id = 1_u64;
    for x in 0..size {
        for z in 0..size {
            sources.push(NavmeshSourceGeometry {
                source_id,
                kind: NavmeshSourceKind::Walkable,
                area: NavmeshArea(0),
                mask: NavmeshAreaMask::from_area(NavmeshArea(0)),
                triangles: NavmeshPrimitive::Quad {
                    size: Vec2::splat(1.0),
                }
                .triangles()
                .transformed(Transform::from_translation(Vec3::new(
                    x as f32 - size as f32 * 0.5,
                    0.0,
                    z as f32 - size as f32 * 0.5,
                ))),
            });
            source_id += 1;
        }
    }

    crate::bake::bake_navmesh(
        &NavmeshBuildInput {
            sources,
            links: Vec::new(),
        },
        &crate::config::NavmeshBakeSettings::default(),
    )
    .unwrap()
}

fn lab_lane_surface(blocked: bool) -> NavmeshSurfaceData {
    let mut sources = Vec::new();
    let mut source_id = 1_u64;

    for x in -3..=3 {
        for row in -1..=1 {
            let area = if row == 0 {
                NavmeshArea(1)
            } else {
                NavmeshArea(0)
            };
            sources.push(source_geometry(
                source_id,
                area,
                NavmeshPrimitive::Quad {
                    size: Vec2::splat(2.0),
                },
                Transform::from_translation(Vec3::new(x as f32 * 2.0, 0.0, row as f32 * 2.0)),
            ));
            source_id += 1;
        }
    }

    if blocked {
        sources.push(source_geometry_with_kind(
            source_id,
            NavmeshSourceKind::Obstacle,
            NavmeshArea(0),
            NavmeshPrimitive::Cuboid {
                size: Vec3::new(3.8, 1.6, 1.7),
            },
            Transform::from_translation(Vec3::new(0.0, 0.8, 0.0)),
        ));
    }

    crate::bake::bake_navmesh(
        &NavmeshBuildInput {
            sources,
            links: Vec::new(),
        },
        &crate::config::NavmeshBakeSettings {
            agent_radius: 0.0,
            async_baking: false,
            rebuild_debounce_seconds: 0.0,
            ..default()
        },
    )
    .unwrap()
}

#[test]
fn nearest_point_projects_to_walkable_space() {
    assert!(crate::math::point_in_triangle_2d(
        Vec2::new(0.0, 0.0),
        [
            Vec2::new(-1.0, -1.0),
            Vec2::new(1.0, -1.0),
            Vec2::new(0.0, 1.0)
        ],
        0.0001,
    ));

    let surface = baked_quad_surface();
    let hit = nearest_point_on_navmesh(
        &surface,
        Vec3::new(3.0, 0.0, 0.25),
        &crate::config::NavmeshQueryFilter::default(),
    )
    .unwrap();

    assert!(hit.position.x <= 2.0 + 0.0001);
    assert!((hit.distance - 1.0).abs() <= 0.001);
}

#[test]
fn require_on_mesh_rejects_projected_starts() {
    let surface = baked_quad_surface();
    let result = query_navmesh_path(
        &surface,
        NavmeshPathId(1),
        Vec3::new(3.0, 0.0, 0.0),
        Vec3::new(0.0, 0.0, 0.0),
        &crate::config::NavmeshQuerySettings {
            projection_policy: crate::config::NavmeshProjectionPolicy::RequireOnMesh,
            ..default()
        },
        &crate::config::NavmeshQueryFilter::default(),
    );

    assert_eq!(result.status, NavmeshPathStatus::StartOutside);
}

#[test]
fn area_masks_filter_disallowed_goals() {
    let surface = two_polygon_surface();
    let result = query_navmesh_path(
        &surface,
        NavmeshPathId(5),
        Vec3::new(-1.0, 0.0, 0.0),
        Vec3::new(1.0, 0.0, -0.5),
        &crate::config::NavmeshQuerySettings {
            projection_policy: crate::config::NavmeshProjectionPolicy::RequireOnMesh,
            ..default()
        },
        &crate::config::NavmeshQueryFilter {
            mask: NavmeshAreaMask::from_area(NavmeshArea(0)),
            ..default()
        },
    );

    assert_eq!(result.status, NavmeshPathStatus::GoalOutside);
}

#[test]
fn weighted_costs_choose_the_cheaper_branch() {
    let surface = weighted_surface();
    let result = query_navmesh_path(
        &surface,
        NavmeshPathId(2),
        Vec3::new(-5.0, 0.0, 0.0),
        Vec3::new(5.0, 0.0, 0.0),
        &crate::config::NavmeshQuerySettings {
            smoothing: crate::config::NavmeshPathSmoothing::None,
            ..default()
        },
        &crate::config::NavmeshQueryFilter {
            area_costs: vec![NavmeshAreaCost::new(NavmeshArea(1), 6.0)],
            ..default()
        },
    );

    let path = result.path.expect("weighted route should produce a path");
    assert_eq!(result.status, NavmeshPathStatus::Success);
    assert_eq!(path.polygons, vec![0, 2, 3]);
}

#[test]
fn unsmoothed_paths_keep_corridor_midpoints() {
    let surface = weighted_surface();
    let result = query_navmesh_path(
        &surface,
        NavmeshPathId(6),
        Vec3::new(-5.0, 0.0, 0.0),
        Vec3::new(5.0, 0.0, 0.0),
        &crate::config::NavmeshQuerySettings {
            smoothing: crate::config::NavmeshPathSmoothing::None,
            ..default()
        },
        &crate::config::NavmeshQueryFilter {
            area_costs: vec![NavmeshAreaCost::new(NavmeshArea(1), 6.0)],
            ..default()
        },
    );

    let path = result.path.expect("detour route should resolve");
    assert_eq!(path.points.len(), 4);
    assert!((path.points[1].position.x + 2.0).abs() <= 0.001);
    assert!((path.points[2].position.x - 2.0).abs() <= 0.001);
}

#[test]
fn weighted_area_costs_are_reported_in_total_cost() {
    let surface = two_polygon_surface();
    let result = query_navmesh_path(
        &surface,
        NavmeshPathId(7),
        Vec3::new(-1.0, 0.0, 0.0),
        Vec3::new(1.0, 0.0, 0.0),
        &crate::config::NavmeshQuerySettings {
            smoothing: crate::config::NavmeshPathSmoothing::None,
            ..default()
        },
        &crate::config::NavmeshQueryFilter {
            area_costs: vec![NavmeshAreaCost::new(NavmeshArea(1), 5.0)],
            ..default()
        },
    );

    let path = result.path.expect("two-polygon route should resolve");
    assert_eq!(result.status, NavmeshPathStatus::Success);
    assert!(path.total_cost > path.total_length);
}

#[test]
fn funnel_paths_complete_on_open_grid() {
    let surface = open_grid_surface(4);
    let result = query_navmesh_path(
        &surface,
        NavmeshPathId(9),
        Vec3::new(-1.5, 0.0, -1.5),
        Vec3::new(1.5, 0.0, 1.5),
        &crate::config::NavmeshQuerySettings::default(),
        &crate::config::NavmeshQueryFilter::default(),
    );

    let path = result.path.expect("open grid path should resolve");
    assert_eq!(result.status, NavmeshPathStatus::Success);
    assert!(path.points.len() >= 2);
}

#[test]
fn disconnected_surfaces_report_partial_or_unreachable() {
    let surface = crate::bake::bake_navmesh(
        &NavmeshBuildInput {
            sources: vec![
                source_geometry(
                    1,
                    NavmeshArea(0),
                    NavmeshPrimitive::Quad {
                        size: Vec2::new(2.0, 2.0),
                    },
                    Transform::from_xyz(-2.0, 0.0, 0.0),
                ),
                source_geometry(
                    2,
                    NavmeshArea(0),
                    NavmeshPrimitive::Quad {
                        size: Vec2::new(2.0, 2.0),
                    },
                    Transform::from_xyz(2.0, 0.0, 0.0),
                ),
            ],
            links: Vec::new(),
        },
        &crate::config::NavmeshBakeSettings::default(),
    )
    .unwrap();

    let unreachable = query_navmesh_path(
        &surface,
        NavmeshPathId(3),
        Vec3::new(-2.0, 0.0, 0.0),
        Vec3::new(2.0, 0.0, 0.0),
        &crate::config::NavmeshQuerySettings {
            allow_partial: false,
            ..default()
        },
        &crate::config::NavmeshQueryFilter::default(),
    );
    assert_eq!(unreachable.status, NavmeshPathStatus::Unreachable);

    let partial = query_navmesh_path(
        &surface,
        NavmeshPathId(4),
        Vec3::new(-2.0, 0.0, 0.0),
        Vec3::new(2.0, 0.0, 0.0),
        &crate::config::NavmeshQuerySettings {
            allow_partial: true,
            smoothing: crate::config::NavmeshPathSmoothing::None,
            ..default()
        },
        &crate::config::NavmeshQueryFilter::default(),
    );
    assert_eq!(partial.status, NavmeshPathStatus::Partial);
    assert_eq!(partial.path.unwrap().polygons.len(), 1);
}

#[test]
fn partial_fallback_can_be_disabled() {
    let surface = crate::bake::bake_navmesh(
        &NavmeshBuildInput {
            sources: vec![
                source_geometry(
                    1,
                    NavmeshArea(0),
                    NavmeshPrimitive::Quad {
                        size: Vec2::new(2.0, 2.0),
                    },
                    Transform::from_xyz(-2.0, 0.0, 0.0),
                ),
                source_geometry(
                    2,
                    NavmeshArea(0),
                    NavmeshPrimitive::Quad {
                        size: Vec2::new(2.0, 2.0),
                    },
                    Transform::from_xyz(2.0, 0.0, 0.0),
                ),
            ],
            links: Vec::new(),
        },
        &crate::config::NavmeshBakeSettings::default(),
    )
    .unwrap();

    let result = query_navmesh_path(
        &surface,
        NavmeshPathId(8),
        Vec3::new(-2.0, 0.0, 0.0),
        Vec3::new(2.0, 0.0, 0.0),
        &crate::config::NavmeshQuerySettings {
            allow_partial: true,
            nearest_reachable_fallback: false,
            ..default()
        },
        &crate::config::NavmeshQueryFilter::default(),
    );

    assert_eq!(result.status, NavmeshPathStatus::Unreachable);
}

#[test]
fn lane_obstacle_forces_a_detour_in_lab_layout() {
    let baseline_surface = lab_lane_surface(false);
    let blocked_surface = lab_lane_surface(true);

    let baseline = query_navmesh_path(
        &baseline_surface,
        NavmeshPathId(17),
        Vec3::new(-6.0, 0.0, 0.0),
        Vec3::new(6.0, 0.0, 0.0),
        &crate::config::NavmeshQuerySettings::default(),
        &crate::config::NavmeshQueryFilter::default(),
    );
    let detour = query_navmesh_path(
        &blocked_surface,
        NavmeshPathId(18),
        Vec3::new(-6.0, 0.0, 0.0),
        Vec3::new(6.0, 0.0, 0.0),
        &crate::config::NavmeshQuerySettings::default(),
        &crate::config::NavmeshQueryFilter::default(),
    );

    assert_eq!(baseline.status, NavmeshPathStatus::Success);
    assert_eq!(detour.status, NavmeshPathStatus::Success);

    let baseline_cost = baseline.path.unwrap().total_cost;
    let detour_path = detour.path.unwrap();
    assert!(
        detour_path.total_cost > baseline_cost,
        "expected detour cost {} to exceed baseline {}",
        detour_path.total_cost,
        baseline_cost,
    );
    assert!(
        detour_path
            .points
            .iter()
            .any(|point| point.position.z.abs() > 0.5),
        "expected detour path to leave the center lane: {:?}",
        detour_path
            .points
            .iter()
            .map(|point| point.position)
            .collect::<Vec<_>>(),
    );
}

#[test]
fn line_of_sight_traverses_straight_corridors_across_multiple_polygons() {
    let surface = open_grid_surface(4);
    let result = query_navmesh_path(
        &surface,
        NavmeshPathId(99),
        Vec3::new(-1.5, 0.0, -1.5),
        Vec3::new(1.5, 0.0, 1.5),
        &crate::config::NavmeshQuerySettings::default(),
        &crate::config::NavmeshQueryFilter::default(),
    );

    assert!(
        line_of_sight(
            &surface,
            Vec3::new(-1.5, 0.0, -1.5),
            Vec3::new(1.5, 0.0, 1.5),
            &crate::config::NavmeshQueryFilter::default(),
        ),
        "expected straight-corridor LOS, got status {:?} and path {:?}",
        result.status,
        result.path.as_ref().map(|path| {
            path.points
                .iter()
                .map(|point| (point.position, &point.transition))
                .collect::<Vec<_>>()
        }),
    );
}

#[test]
fn line_of_sight_rejects_routes_that_need_a_detour() {
    let surface = lab_lane_surface(true);

    assert!(!line_of_sight(
        &surface,
        Vec3::new(-6.0, 0.0, 0.0),
        Vec3::new(6.0, 0.0, 0.0),
        &crate::config::NavmeshQueryFilter::default(),
    ));
}
