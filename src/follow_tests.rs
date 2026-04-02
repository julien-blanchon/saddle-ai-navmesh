use bevy::prelude::*;

use super::*;
use crate::{
    components::NavmeshAgent,
    path::{NavmeshPath, NavmeshPathPoint},
};

fn sample_path() -> NavmeshPath {
    NavmeshPath {
        points: vec![
            NavmeshPathPoint {
                position: Vec3::new(0.0, 0.0, 0.0),
                ..default()
            },
            NavmeshPathPoint {
                position: Vec3::new(1.0, 0.0, 0.0),
                ..default()
            },
            NavmeshPathPoint {
                position: Vec3::new(3.0, 0.0, 0.0),
                ..default()
            },
        ],
        ..default()
    }
}

#[test]
fn waypoint_advance_skips_close_points() {
    let agent = NavmeshAgent {
        arrival_distance: 0.2,
        waypoint_distance: 0.25,
        overshoot_distance: 0.1,
        ..NavmeshAgent::new(Entity::PLACEHOLDER)
    };

    let next_index = advance_waypoint_index(&agent, &sample_path(), 1, Vec3::new(0.9, 0.0, 0.0));

    assert_eq!(next_index, 2);
}

#[test]
fn waypoint_advance_handles_overshoot() {
    let agent = NavmeshAgent {
        arrival_distance: 0.2,
        waypoint_distance: 0.1,
        overshoot_distance: 0.15,
        ..NavmeshAgent::new(Entity::PLACEHOLDER)
    };

    let next_index = advance_waypoint_index(&agent, &sample_path(), 1, Vec3::new(1.3, 0.0, 0.0));

    assert_eq!(next_index, 2);
}

#[test]
fn remaining_distance_accumulates_remaining_segments() {
    let remaining = remaining_distance(&sample_path(), 1, Vec3::new(0.5, 0.0, 0.0));

    assert!((remaining - 2.5).abs() <= 0.0001);
}
