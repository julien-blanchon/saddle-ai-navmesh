use bevy::prelude::*;

use crate::config::NavmeshAreaMask;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Reflect, Default)]
#[reflect(Default)]
pub struct NavmeshPathId(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Reflect, Default)]
#[reflect(Default)]
pub struct NavmeshLinkId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Reflect, Default)]
pub enum NavmeshPathStatus {
    #[default]
    Pending,
    Success,
    Partial,
    Unreachable,
    StartOutside,
    GoalOutside,
    InvalidSurface,
}

#[derive(Debug, Clone, PartialEq, Reflect)]
#[reflect(Default)]
pub struct NavmeshProjectionHit {
    pub polygon: u32,
    pub position: Vec3,
    pub distance: f32,
}

impl Default for NavmeshProjectionHit {
    fn default() -> Self {
        Self {
            polygon: 0,
            position: Vec3::ZERO,
            distance: 0.0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Reflect, Default)]
pub enum NavmeshPathTransition {
    #[default]
    Surface,
    OffMeshLink(NavmeshLinkId),
}

#[derive(Debug, Clone, PartialEq, Reflect)]
#[reflect(Default)]
pub struct NavmeshPathPoint {
    pub position: Vec3,
    pub transition: NavmeshPathTransition,
}

impl Default for NavmeshPathPoint {
    fn default() -> Self {
        Self {
            position: Vec3::ZERO,
            transition: NavmeshPathTransition::Surface,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Reflect)]
#[reflect(Default)]
pub struct NavmeshCorridorPortal {
    pub from_polygon: u32,
    pub to_polygon: u32,
    pub left: Vec3,
    pub right: Vec3,
}

impl Default for NavmeshCorridorPortal {
    fn default() -> Self {
        Self {
            from_polygon: 0,
            to_polygon: 0,
            left: Vec3::ZERO,
            right: Vec3::ZERO,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Reflect)]
#[reflect(Default)]
pub struct NavmeshOffMeshLink {
    pub start: Vec3,
    pub end: Vec3,
    pub bidirectional: bool,
    pub mask: NavmeshAreaMask,
    pub cost_multiplier: f32,
    pub snap_distance: f32,
}

impl Default for NavmeshOffMeshLink {
    fn default() -> Self {
        Self {
            start: Vec3::ZERO,
            end: Vec3::Z,
            bidirectional: true,
            mask: NavmeshAreaMask::all(),
            cost_multiplier: 1.0,
            snap_distance: 1.0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Reflect)]
#[reflect(Default)]
pub struct NavmeshPath {
    pub points: Vec<NavmeshPathPoint>,
    pub corridor: Vec<NavmeshCorridorPortal>,
    pub polygons: Vec<u32>,
    pub total_cost: f32,
    pub total_length: f32,
    pub generation: u64,
}

impl Default for NavmeshPath {
    fn default() -> Self {
        Self {
            points: Vec::new(),
            corridor: Vec::new(),
            polygons: Vec::new(),
            total_cost: 0.0,
            total_length: 0.0,
            generation: 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Reflect)]
#[reflect(Default)]
pub struct NavmeshPathQueryResult {
    pub request_id: NavmeshPathId,
    pub status: NavmeshPathStatus,
    pub projected_start: Option<NavmeshProjectionHit>,
    pub projected_goal: Option<NavmeshProjectionHit>,
    pub path: Option<NavmeshPath>,
    pub visited_nodes: u32,
    pub generation: u64,
    pub duration_ms: f32,
}

impl Default for NavmeshPathQueryResult {
    fn default() -> Self {
        Self {
            request_id: NavmeshPathId::default(),
            status: NavmeshPathStatus::Pending,
            projected_start: None,
            projected_goal: None,
            path: None,
            visited_nodes: 0,
            generation: 0,
            duration_ms: 0.0,
        }
    }
}
