use std::{cmp::Ordering, collections::BinaryHeap, time::Instant};

use bevy::prelude::*;

use crate::{
    bake::{NavmeshPortal, NavmeshSurfaceData},
    config::{
        NavmeshPathSmoothing, NavmeshProjectionPolicy, NavmeshQueryFilter, NavmeshQuerySettings,
    },
    math::{NavmeshBasis, nearest_point_on_triangle, tri_area2},
    path::{
        NavmeshCorridorPortal, NavmeshPath, NavmeshPathId, NavmeshPathPoint,
        NavmeshPathQueryResult, NavmeshPathStatus, NavmeshPathTransition, NavmeshProjectionHit,
    },
};

#[derive(Clone, Copy)]
enum PathEdge {
    Portal(u32),
    Link { link_index: usize, forward: bool },
}

#[derive(Clone, Copy)]
struct PreviousStep {
    previous_polygon: u32,
    via: PathEdge,
}

#[derive(Clone, Copy)]
struct FrontierNode {
    polygon: u32,
    estimated_total: f32,
}

impl PartialEq for FrontierNode {
    fn eq(&self, other: &Self) -> bool {
        self.polygon == other.polygon && self.estimated_total == other.estimated_total
    }
}

impl Eq for FrontierNode {}

impl PartialOrd for FrontierNode {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for FrontierNode {
    fn cmp(&self, other: &Self) -> Ordering {
        other
            .estimated_total
            .total_cmp(&self.estimated_total)
            .then_with(|| self.polygon.cmp(&other.polygon))
    }
}

pub fn nearest_point_on_navmesh(
    surface: &NavmeshSurfaceData,
    point: Vec3,
    filter: &NavmeshQueryFilter,
) -> Option<NavmeshProjectionHit> {
    let mut best_hit: Option<NavmeshProjectionHit> = None;

    for polygon in &surface.polygons {
        if !filter.allows_area(polygon.mask) {
            continue;
        }
        let vertices = surface.polygon_vertices(polygon.id)?;
        let nearest = nearest_point_on_triangle(point, vertices[0], vertices[1], vertices[2]);
        let distance = point.distance(nearest);
        match &best_hit {
            Some(hit) if hit.distance <= distance => {}
            _ => {
                best_hit = Some(NavmeshProjectionHit {
                    polygon: polygon.id,
                    position: nearest,
                    distance,
                });
            }
        }
    }

    best_hit
}

pub fn line_of_sight(
    surface: &NavmeshSurfaceData,
    start: Vec3,
    goal: Vec3,
    filter: &NavmeshQueryFilter,
) -> bool {
    let Some(start_hit) = nearest_point_on_navmesh(surface, start, filter) else {
        return false;
    };
    let Some(goal_hit) = nearest_point_on_navmesh(surface, goal, filter) else {
        return false;
    };

    let direct_length = start_hit.position.distance(goal_hit.position);
    if direct_length <= 0.0001 {
        return true;
    }

    // Sample the straight segment densely enough to catch small gaps without turning the helper
    // into a full path query. This keeps the API useful for shortcut checks and debug probes.
    let sample_count = ((direct_length / 0.25).ceil() as usize).clamp(2, 256);
    let tolerance = 0.02_f32.max(direct_length * 0.001);

    (0..=sample_count).all(|index| {
        let t = index as f32 / sample_count as f32;
        let sample = start_hit.position.lerp(goal_hit.position, t);
        nearest_point_on_navmesh(surface, sample, filter)
            .is_some_and(|hit| hit.distance <= tolerance)
    })
}

pub fn query_navmesh_path(
    surface: &NavmeshSurfaceData,
    request_id: NavmeshPathId,
    start: Vec3,
    goal: Vec3,
    settings: &NavmeshQuerySettings,
    filter: &NavmeshQueryFilter,
) -> NavmeshPathQueryResult {
    let started = Instant::now();
    let mut result = NavmeshPathQueryResult {
        request_id,
        generation: surface.generation,
        ..default()
    };

    let Some(start_hit) = nearest_point_on_navmesh(surface, start, filter) else {
        result.status = NavmeshPathStatus::StartOutside;
        result.duration_ms = started.elapsed().as_secs_f32() * 1000.0;
        return result;
    };
    if matches!(
        settings.projection_policy,
        NavmeshProjectionPolicy::RequireOnMesh
    ) && start_hit.distance > settings.epsilon
    {
        result.status = NavmeshPathStatus::StartOutside;
        result.duration_ms = started.elapsed().as_secs_f32() * 1000.0;
        return result;
    }

    let Some(goal_hit) = nearest_point_on_navmesh(surface, goal, filter) else {
        result.projected_start = Some(start_hit);
        result.status = NavmeshPathStatus::GoalOutside;
        result.duration_ms = started.elapsed().as_secs_f32() * 1000.0;
        return result;
    };
    if matches!(
        settings.projection_policy,
        NavmeshProjectionPolicy::RequireOnMesh
    ) && goal_hit.distance > settings.epsilon
    {
        result.projected_start = Some(start_hit);
        result.status = NavmeshPathStatus::GoalOutside;
        result.duration_ms = started.elapsed().as_secs_f32() * 1000.0;
        return result;
    }

    result.projected_start = Some(start_hit.clone());
    result.projected_goal = Some(goal_hit.clone());

    if start_hit.polygon == goal_hit.polygon {
        result.status = NavmeshPathStatus::Success;
        result.path = Some(NavmeshPath {
            points: vec![
                NavmeshPathPoint {
                    position: start_hit.position,
                    transition: NavmeshPathTransition::Surface,
                },
                NavmeshPathPoint {
                    position: goal_hit.position,
                    transition: NavmeshPathTransition::Surface,
                },
            ],
            polygons: vec![start_hit.polygon],
            total_cost: start_hit.position.distance(goal_hit.position),
            total_length: start_hit.position.distance(goal_hit.position),
            generation: surface.generation,
            ..default()
        });
        result.duration_ms = started.elapsed().as_secs_f32() * 1000.0;
        return result;
    }

    let polygon_count = surface.polygons.len();
    let mut frontier = BinaryHeap::new();
    let mut cost_so_far = vec![f32::INFINITY; polygon_count];
    let mut previous = vec![None; polygon_count];
    let mut visited_nodes = 0_u32;

    cost_so_far[start_hit.polygon as usize] = 0.0;
    frontier.push(FrontierNode {
        polygon: start_hit.polygon,
        estimated_total: heuristic(surface, start_hit.polygon, goal_hit.position),
    });

    let mut best_partial = start_hit.polygon;
    let mut best_partial_score = heuristic(surface, start_hit.polygon, goal_hit.position);
    let mut reached_goal = false;

    while let Some(current) = frontier.pop() {
        visited_nodes += 1;
        if current.polygon == goal_hit.polygon {
            reached_goal = true;
            break;
        }

        let current_cost = cost_so_far[current.polygon as usize];
        let partial_score = heuristic(surface, current.polygon, goal_hit.position);
        if partial_score < best_partial_score {
            best_partial_score = partial_score;
            best_partial = current.polygon;
        }

        let current_polygon = &surface.polygons[current.polygon as usize];
        for &portal_index in &current_polygon.portal_indices {
            let portal = &surface.portals[portal_index as usize];
            let next_polygon = if portal.polygons[0] == current.polygon {
                portal.polygons[1]
            } else {
                portal.polygons[0]
            };
            let next = &surface.polygons[next_polygon as usize];
            if !filter.allows_area(next.mask) {
                continue;
            }

            let step_cost =
                current_polygon.centroid.distance(next.centroid) * filter.cost_for_area(next.area);
            let new_cost = current_cost + step_cost.max(settings.epsilon);
            if new_cost < cost_so_far[next_polygon as usize] {
                cost_so_far[next_polygon as usize] = new_cost;
                previous[next_polygon as usize] = Some(PreviousStep {
                    previous_polygon: current.polygon,
                    via: PathEdge::Portal(portal_index),
                });
                frontier.push(FrontierNode {
                    polygon: next_polygon,
                    estimated_total: new_cost + heuristic(surface, next_polygon, goal_hit.position),
                });
            }
        }

        for (link_index, link) in surface.links.iter().enumerate() {
            let (matches, next_polygon, entry, exit) = if link.from_polygon == current.polygon {
                (true, link.to_polygon, link.start, link.end)
            } else if link.bidirectional && link.to_polygon == current.polygon {
                (true, link.from_polygon, link.end, link.start)
            } else {
                (false, 0, Vec3::ZERO, Vec3::ZERO)
            };
            if !matches || !filter.allows_link(link.mask) {
                continue;
            }
            let next = &surface.polygons[next_polygon as usize];
            if !filter.allows_area(next.mask) {
                continue;
            }

            let step_cost = current_polygon.centroid.distance(entry)
                + entry.distance(exit) * link.cost_multiplier.max(1.0)
                + exit.distance(next.centroid);
            let new_cost = current_cost + step_cost * filter.cost_for_area(next.area);
            if new_cost < cost_so_far[next_polygon as usize] {
                cost_so_far[next_polygon as usize] = new_cost;
                previous[next_polygon as usize] = Some(PreviousStep {
                    previous_polygon: current.polygon,
                    via: PathEdge::Link {
                        link_index,
                        forward: link.from_polygon == current.polygon,
                    },
                });
                frontier.push(FrontierNode {
                    polygon: next_polygon,
                    estimated_total: new_cost + heuristic(surface, next_polygon, goal_hit.position),
                });
            }
        }
    }

    result.visited_nodes = visited_nodes;
    let target_polygon = if reached_goal {
        goal_hit.polygon
    } else if settings.allow_partial && settings.nearest_reachable_fallback {
        best_partial
    } else {
        result.status = NavmeshPathStatus::Unreachable;
        result.duration_ms = started.elapsed().as_secs_f32() * 1000.0;
        return result;
    };
    let query_cost = cost_so_far[target_polygon as usize];

    let path = reconstruct_path(
        surface,
        &previous,
        start_hit.clone(),
        goal_hit.clone(),
        target_polygon,
        settings.smoothing,
        query_cost,
    );

    result.status = if reached_goal {
        NavmeshPathStatus::Success
    } else {
        NavmeshPathStatus::Partial
    };
    result.path = Some(path);
    result.duration_ms = started.elapsed().as_secs_f32() * 1000.0;
    result
}

fn heuristic(surface: &NavmeshSurfaceData, polygon: u32, goal: Vec3) -> f32 {
    surface.polygons[polygon as usize].centroid.distance(goal)
}

fn reconstruct_path(
    surface: &NavmeshSurfaceData,
    previous: &[Option<PreviousStep>],
    start_hit: NavmeshProjectionHit,
    goal_hit: NavmeshProjectionHit,
    target_polygon: u32,
    smoothing: NavmeshPathSmoothing,
    query_cost: f32,
) -> NavmeshPath {
    let mut polygons = vec![target_polygon];
    let mut steps = Vec::new();
    let mut current = target_polygon;
    while let Some(step) = previous[current as usize] {
        polygons.push(step.previous_polygon);
        steps.push(step.via);
        current = step.previous_polygon;
    }
    polygons.reverse();
    steps.reverse();

    let basis = surface.basis.basis();
    let mut points = Vec::new();
    let mut corridor = Vec::new();
    let mut current_start = start_hit.position;
    let mut active_portals = Vec::new();

    points.push(NavmeshPathPoint {
        position: start_hit.position,
        transition: NavmeshPathTransition::Surface,
    });

    for (step_index, step) in steps.iter().enumerate() {
        match step {
            PathEdge::Portal(portal_index) => {
                let from_polygon = polygons[step_index];
                let portal = directed_portal(
                    surface,
                    &surface.portals[*portal_index as usize],
                    from_polygon,
                    basis,
                );
                corridor.push(portal.clone());
                active_portals.push(portal);
            }
            PathEdge::Link {
                link_index,
                forward,
            } => {
                let link = &surface.links[*link_index];
                let entry = if *forward { link.start } else { link.end };
                let exit = if *forward { link.end } else { link.start };
                append_smoothed_points(
                    &mut points,
                    current_start,
                    entry,
                    &active_portals,
                    basis,
                    smoothing,
                );
                points.push(NavmeshPathPoint {
                    position: exit,
                    transition: NavmeshPathTransition::OffMeshLink(link.id),
                });
                current_start = exit;
                active_portals.clear();
            }
        }
    }

    let final_goal = if target_polygon == goal_hit.polygon {
        goal_hit.position
    } else {
        surface.polygons[target_polygon as usize].centroid
    };
    append_smoothed_points(
        &mut points,
        current_start,
        final_goal,
        &active_portals,
        basis,
        smoothing,
    );

    let total_length = points
        .windows(2)
        .map(|window| window[0].position.distance(window[1].position))
        .sum::<f32>();

    NavmeshPath {
        points,
        corridor,
        polygons,
        total_cost: total_length.max(query_cost),
        total_length,
        generation: surface.generation,
    }
}

fn directed_portal(
    surface: &NavmeshSurfaceData,
    portal: &NavmeshPortal,
    from_polygon: u32,
    basis: NavmeshBasis,
) -> NavmeshCorridorPortal {
    let left_right = {
        let a = basis.project(portal.edge[0]);
        let b = basis.project(portal.edge[1]);
        let centroid = basis.project(surface.polygons[from_polygon as usize].centroid);
        if tri_area2(a, b, centroid) < 0.0 {
            [portal.edge[0], portal.edge[1]]
        } else {
            [portal.edge[1], portal.edge[0]]
        }
    };

    let to_polygon = if portal.polygons[0] == from_polygon {
        portal.polygons[1]
    } else {
        portal.polygons[0]
    };

    NavmeshCorridorPortal {
        from_polygon,
        to_polygon,
        left: left_right[0],
        right: left_right[1],
    }
}

fn append_smoothed_points(
    points: &mut Vec<NavmeshPathPoint>,
    start: Vec3,
    goal: Vec3,
    portals: &[NavmeshCorridorPortal],
    basis: NavmeshBasis,
    smoothing: NavmeshPathSmoothing,
) {
    let smoothed = if portals.is_empty() {
        vec![start, goal]
    } else if matches!(smoothing, NavmeshPathSmoothing::None) {
        raw_corridor_points(start, goal, portals)
    } else {
        string_pull(start, goal, portals, basis)
    };

    for (index, point) in smoothed.into_iter().enumerate() {
        if index == 0
            && points
                .last()
                .is_some_and(|last| last.position.distance(point) <= 0.0001)
        {
            continue;
        }
        points.push(NavmeshPathPoint {
            position: point,
            transition: NavmeshPathTransition::Surface,
        });
    }
}

fn raw_corridor_points(start: Vec3, goal: Vec3, portals: &[NavmeshCorridorPortal]) -> Vec<Vec3> {
    let mut points = Vec::with_capacity(portals.len() + 2);
    points.push(start);

    for portal in portals {
        let midpoint = (portal.left + portal.right) * 0.5;
        if points
            .last()
            .is_none_or(|last| last.distance(midpoint) > 0.0001)
        {
            points.push(midpoint);
        }
    }

    if points
        .last()
        .is_none_or(|last| last.distance(goal) > 0.0001)
    {
        points.push(goal);
    }

    points
}

fn string_pull(
    start: Vec3,
    goal: Vec3,
    portals: &[NavmeshCorridorPortal],
    basis: NavmeshBasis,
) -> Vec<Vec3> {
    if portals.is_empty() {
        return vec![start, goal];
    }

    let start_2d = basis.project(start);
    let goal_2d = basis.project(goal);
    let mut points = vec![start];

    let mut left_index = 0_usize;
    let mut right_index = 0_usize;
    let mut portal_apex = start_2d;
    let mut portal_left = basis.project(portals[0].left);
    let mut portal_right = basis.project(portals[0].right);

    let mut index = 1_usize;
    let mut iterations = 0_usize;
    let max_iterations = portals.len().saturating_mul(8).max(16);
    while index <= portals.len() {
        iterations += 1;
        if iterations > max_iterations {
            return raw_corridor_points(start, goal, portals);
        }

        let (left, right) = if index == portals.len() {
            (goal_2d, goal_2d)
        } else {
            (
                basis.project(portals[index].left),
                basis.project(portals[index].right),
            )
        };

        if tri_area2(portal_apex, portal_right, right) <= 0.0 {
            if portal_apex == portal_right || tri_area2(portal_apex, portal_left, right) > 0.0 {
                portal_right = right;
                right_index = index;
            } else {
                points.push(portals[left_index].left);
                portal_apex = basis.project(points.last().copied().unwrap());
                let apex_index = left_index;
                right_index = left_index;
                if apex_index < portals.len() {
                    portal_left = basis.project(portals[apex_index].left);
                    portal_right = basis.project(portals[apex_index].right);
                } else {
                    portal_left = goal_2d;
                    portal_right = goal_2d;
                }
                index = apex_index + 1;
                continue;
            }
        }

        if tri_area2(portal_apex, portal_left, left) >= 0.0 {
            if portal_apex == portal_left || tri_area2(portal_apex, portal_right, left) < 0.0 {
                portal_left = left;
                left_index = index;
            } else {
                points.push(portals[right_index].right);
                portal_apex = basis.project(points.last().copied().unwrap());
                let apex_index = right_index;
                left_index = apex_index;
                right_index = apex_index;
                if apex_index < portals.len() {
                    portal_left = basis.project(portals[apex_index].left);
                    portal_right = basis.project(portals[apex_index].right);
                } else {
                    portal_left = goal_2d;
                    portal_right = goal_2d;
                }
                index = apex_index + 1;
                continue;
            }
        }
        index += 1;
    }

    if points
        .last()
        .is_none_or(|last| last.distance(goal) > 0.0001)
    {
        points.push(goal);
    }
    points
}

#[cfg(test)]
#[path = "query_tests.rs"]
mod tests;
