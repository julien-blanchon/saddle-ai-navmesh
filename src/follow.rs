use bevy::prelude::*;

use crate::{
    components::{NavmeshAgent, NavmeshCrowdAvoidance},
    path::NavmeshPath,
};

pub(crate) fn advance_waypoint_index(
    agent: &NavmeshAgent,
    path: &NavmeshPath,
    mut waypoint_index: usize,
    position: Vec3,
) -> usize {
    if path.points.is_empty() {
        return 0;
    }

    waypoint_index = waypoint_index.min(path.points.len().saturating_sub(1));

    while waypoint_index + 1 < path.points.len() {
        let next = path.points[waypoint_index].position;
        let next_distance = position.distance(next);
        if next_distance <= agent.waypoint_distance {
            waypoint_index += 1;
            continue;
        }

        if waypoint_index > 0 {
            let previous = path.points[waypoint_index - 1].position;
            let segment = next - previous;
            let segment_length = segment.length();
            if segment_length > f32::EPSILON {
                let progress = (position - previous).dot(segment / segment_length);
                if progress > segment_length + agent.overshoot_distance {
                    waypoint_index += 1;
                    continue;
                }
            }
        }
        break;
    }

    waypoint_index
}

pub(crate) fn remaining_distance(path: &NavmeshPath, waypoint_index: usize, position: Vec3) -> f32 {
    if path.points.is_empty() {
        return 0.0;
    }

    let clamped = waypoint_index.min(path.points.len().saturating_sub(1));
    let mut remaining = position.distance(path.points[clamped].position);
    for window in path.points[clamped..].windows(2) {
        remaining += window[0].position.distance(window[1].position);
    }
    remaining
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct CrowdNeighbor {
    pub position: Vec3,
    pub desired_velocity: Vec3,
    pub body_radius: f32,
}

pub(crate) fn apply_crowd_avoidance(
    position: Vec3,
    desired_velocity: Vec3,
    config: &NavmeshCrowdAvoidance,
    neighbors: impl IntoIterator<Item = CrowdNeighbor>,
) -> (Vec3, usize) {
    if !config.enabled || desired_velocity.length_squared() <= f32::EPSILON {
        return (desired_velocity, 0);
    }

    let heading = desired_velocity.normalize();
    let speed = desired_velocity.length();
    let mut adjustment = Vec3::ZERO;
    let mut conflict_count = 0_usize;
    let max_neighbors = config.max_neighbors.max(1);
    let horizon = config.time_horizon.max(0.05);
    let neighbor_distance = config.neighbor_distance.max(0.0);

    for neighbor in neighbors.into_iter().take(max_neighbors) {
        let offset = neighbor.position - position;
        let distance = offset.length();
        if distance <= f32::EPSILON || distance > neighbor_distance {
            continue;
        }

        let combined_radius = config.body_radius.max(0.0)
            + neighbor.body_radius.max(0.0)
            + config.comfort_distance.max(0.0);
        let relative_velocity = desired_velocity - neighbor.desired_velocity;
        let relative_speed_sq = relative_velocity.length_squared();
        let time_to_closest = if relative_speed_sq > f32::EPSILON {
            (-offset.dot(relative_velocity) / relative_speed_sq).clamp(0.0, horizon)
        } else {
            0.0
        };
        let closest_offset = offset + relative_velocity * time_to_closest;
        let ahead_distance = offset.dot(heading);
        let lateral_offset = offset - heading * ahead_distance;
        let overlap_now = distance < combined_radius;
        let ahead_conflict = ahead_distance > 0.0
            && ahead_distance <= speed * horizon + combined_radius
            && lateral_offset.length() <= combined_radius;
        let will_collide = overlap_now || closest_offset.length() < combined_radius || ahead_conflict;
        if !will_collide {
            continue;
        }

        conflict_count += 1;
        let away = if overlap_now {
            (-offset).normalize_or_zero()
        } else {
            (-closest_offset).normalize_or_zero()
        };
        let side = Vec3::new(-heading.z, 0.0, heading.x).normalize_or_zero();
        let urgency = if overlap_now {
            1.0 + ((combined_radius - distance) / combined_radius.max(0.001)).clamp(0.0, 1.0)
        } else {
            (1.0 - time_to_closest / horizon).clamp(0.0, 1.0)
        };
        let push = away + side * config.side_bias;
        adjustment += push.normalize_or_zero() * speed * urgency * 0.5;
    }

    (
        (desired_velocity + adjustment).clamp_length_max(speed.max(0.0)),
        conflict_count,
    )
}

#[cfg(test)]
#[path = "follow_tests.rs"]
mod tests;
