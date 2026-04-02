use bevy::prelude::*;

use super::*;
use crate::{
    config::{NavmeshArea, NavmeshAreaMask},
    geometry::{NavmeshPrimitive, NavmeshSourceGeometry},
    path::{NavmeshOffMeshLink, NavmeshPathId, NavmeshPathTransition},
};

fn source_geometry(
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

#[test]
fn bake_connects_adjacent_triangles_with_a_portal() {
    let input = NavmeshBuildInput {
        sources: vec![source_geometry(
            1,
            NavmeshSourceKind::Walkable,
            NavmeshArea(0),
            NavmeshPrimitive::Quad {
                size: Vec2::new(4.0, 4.0),
            },
            Transform::default(),
        )],
        links: Vec::new(),
    };

    let baked = bake_navmesh(&input, &NavmeshBakeSettings::default()).unwrap();

    assert_eq!(baked.polygons.len(), 2);
    assert_eq!(baked.portals.len(), 1);
    assert!(
        baked
            .polygons
            .iter()
            .all(|polygon| polygon.portal_indices.len() == 1)
    );
}

#[test]
fn bake_rejects_walkable_space_fully_covered_by_obstacles() {
    let input = NavmeshBuildInput {
        sources: vec![
            source_geometry(
                1,
                NavmeshSourceKind::Walkable,
                NavmeshArea(0),
                NavmeshPrimitive::Quad {
                    size: Vec2::new(4.0, 4.0),
                },
                Transform::default(),
            ),
            source_geometry(
                2,
                NavmeshSourceKind::Obstacle,
                NavmeshArea(0),
                NavmeshPrimitive::Cuboid {
                    size: Vec3::new(3.5, 2.0, 3.5),
                },
                Transform::from_xyz(0.0, 1.0, 0.0),
            ),
        ],
        links: Vec::new(),
    };

    let error = bake_navmesh(
        &input,
        &NavmeshBakeSettings {
            agent_radius: 0.5,
            ..default()
        },
    )
    .unwrap_err();

    assert!(
        error.to_string().contains("produced no walkable polygons"),
        "unexpected error: {error}",
    );
}

#[test]
fn bake_rejects_walkable_space_when_obstacle_crosses_triangle_interiors() {
    let input = NavmeshBuildInput {
        sources: vec![
            source_geometry(
                1,
                NavmeshSourceKind::Walkable,
                NavmeshArea(0),
                NavmeshPrimitive::Quad {
                    size: Vec2::new(4.0, 4.0),
                },
                Transform::default(),
            ),
            source_geometry(
                2,
                NavmeshSourceKind::Obstacle,
                NavmeshArea(0),
                NavmeshPrimitive::Cuboid {
                    size: Vec3::new(0.6, 2.0, 3.6),
                },
                Transform::from_xyz(0.0, 1.0, 0.0),
            ),
        ],
        links: Vec::new(),
    };

    let error = bake_navmesh(
        &input,
        &NavmeshBakeSettings {
            agent_radius: 0.0,
            ..default()
        },
    )
    .unwrap_err();

    assert!(
        error.to_string().contains("produced no walkable polygons"),
        "unexpected error: {error}",
    );
}

#[test]
fn bake_projects_off_mesh_links_onto_polygons() {
    let input = NavmeshBuildInput {
        sources: vec![
            source_geometry(
                1,
                NavmeshSourceKind::Walkable,
                NavmeshArea(0),
                NavmeshPrimitive::Quad {
                    size: Vec2::new(2.0, 2.0),
                },
                Transform::from_xyz(-1.75, 0.0, 0.0),
            ),
            source_geometry(
                2,
                NavmeshSourceKind::Walkable,
                NavmeshArea(0),
                NavmeshPrimitive::Quad {
                    size: Vec2::new(2.0, 2.0),
                },
                Transform::from_xyz(1.75, 0.0, 0.0),
            ),
        ],
        links: vec![NavmeshOffMeshLink {
            start: Vec3::new(-0.75, 0.0, 0.0),
            end: Vec3::new(0.75, 0.0, 0.0),
            bidirectional: true,
            mask: NavmeshAreaMask::all(),
            cost_multiplier: 1.0,
            snap_distance: 1.0,
        }],
    };

    let baked = bake_navmesh(&input, &NavmeshBakeSettings::default()).unwrap();
    let path = baked.query_path(
        NavmeshPathId(7),
        Vec3::new(-1.75, 0.0, 0.0),
        Vec3::new(1.75, 0.0, 0.0),
        &crate::config::NavmeshQuerySettings::default(),
        &crate::config::NavmeshQueryFilter::default(),
    );

    assert_eq!(baked.links.len(), 1);
    assert_eq!(path.status, crate::path::NavmeshPathStatus::Success);
    assert!(path.path.is_some());
    assert!(
        path.path
            .unwrap()
            .points
            .iter()
            .any(|point| matches!(point.transition, NavmeshPathTransition::OffMeshLink(_)))
    );
}
