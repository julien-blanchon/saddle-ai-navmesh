use bevy::{
    color::palettes::css::{AQUA, GOLD, HOT_PINK, LIME, ORANGE, RED, WHITE, YELLOW},
    gizmos::config::{DefaultGizmoConfigGroup, GizmoConfigStore},
    prelude::*,
};

use crate::{
    components::{NavmeshAgent, NavmeshPathResult, NavmeshSteeringOutput},
    config::NavmeshDebugSettings,
};

pub(crate) fn can_draw_debug(
    settings: Option<Res<NavmeshDebugSettings>>,
    gizmo_config: Option<Res<GizmoConfigStore>>,
) -> bool {
    settings.is_some_and(|settings| settings.enabled) && gizmo_config.is_some()
}

pub(crate) fn apply_debug_settings(
    settings: Res<NavmeshDebugSettings>,
    gizmo_config: Option<ResMut<GizmoConfigStore>>,
) {
    if !settings.is_changed() {
        return;
    }

    let Some(mut gizmo_config) = gizmo_config else {
        return;
    };
    let (config, _) = gizmo_config.config_mut::<DefaultGizmoConfigGroup>();
    config.depth_bias = settings.surface_depth_bias;
}

pub(crate) fn draw_navmesh_debug(
    settings: Res<NavmeshDebugSettings>,
    mut gizmos: Gizmos,
    surfaces: Query<(
        &crate::components::NavmeshSurfaceStatus,
        &crate::bake::NavmeshSurfaceData,
    )>,
    path_results: Query<&NavmeshPathResult>,
    agents: Query<(&GlobalTransform, &NavmeshSteeringOutput), With<NavmeshAgent>>,
) {
    if !settings.enabled {
        return;
    }

    for (_status, surface) in &surfaces {
        if settings.draw_surface {
            for polygon in &surface.polygons {
                if let Some(vertices) = surface.polygon_vertices(polygon.id) {
                    gizmos.line(vertices[0], vertices[1], AQUA);
                    gizmos.line(vertices[1], vertices[2], AQUA);
                    gizmos.line(vertices[2], vertices[0], AQUA);
                }
            }
        }

        if settings.draw_portals {
            for portal in &surface.portals {
                gizmos.line(portal.edge[0], portal.edge[1], GOLD);
            }
        }

        if settings.draw_links {
            for link in &surface.links {
                gizmos.arrow(link.start, link.end, ORANGE);
                gizmos.cross(link.start, 0.15, ORANGE);
                gizmos.cross(link.end, 0.15, ORANGE);
            }
        }
    }

    if settings.draw_paths || settings.draw_projections {
        for path_result in &path_results {
            if settings.draw_projections {
                if let Some(start) = &path_result.result.projected_start {
                    gizmos.cross(start.position, 0.12, HOT_PINK);
                }
                if let Some(goal) = &path_result.result.projected_goal {
                    gizmos.cross(goal.position, 0.12, YELLOW);
                }
            }

            if settings.draw_paths {
                if let Some(path) = &path_result.result.path {
                    for window in path.points.windows(2) {
                        let color = match window[1].transition {
                            crate::path::NavmeshPathTransition::Surface => LIME,
                            crate::path::NavmeshPathTransition::OffMeshLink(_) => ORANGE,
                        };
                        gizmos.line(window[0].position, window[1].position, color);
                    }
                }
            }
        }
    }

    if settings.draw_agents {
        for (transform, output) in &agents {
            let origin = transform.translation();
            if let Some(next_target) = output.next_target {
                gizmos.line(origin, next_target, WHITE);
                gizmos.cross(next_target, 0.1, WHITE);
            } else {
                gizmos.cross(origin, 0.08, RED);
            }

            if output.desired_direction.length_squared() > f32::EPSILON {
                gizmos.arrow(
                    origin,
                    origin + output.desired_direction.normalize() * 0.8,
                    WHITE,
                );
            }
        }
    }
}
