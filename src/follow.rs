use bevy::prelude::*;

use crate::{components::NavmeshAgent, path::NavmeshPath};

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

#[cfg(test)]
#[path = "follow_tests.rs"]
mod tests;
