use bevy::prelude::*;

use crate::{
    config::{
        NavmeshArea, NavmeshAreaMask, NavmeshBakeState, NavmeshQueryFilter, NavmeshQuerySettings,
    },
    geometry::NavmeshSourceKind,
    path::{NavmeshOffMeshLink, NavmeshPathId, NavmeshPathQueryResult, NavmeshPathStatus},
};

#[derive(Component, Debug, Clone, PartialEq, Reflect)]
#[reflect(Component, Default)]
pub struct NavmeshSurface {
    pub enabled: bool,
    pub auto_rebuild: bool,
}

impl Default for NavmeshSurface {
    fn default() -> Self {
        Self {
            enabled: true,
            auto_rebuild: true,
        }
    }
}

#[derive(Component, Debug, Clone, PartialEq, Reflect)]
#[reflect(Component, Default)]
pub struct NavmeshSurfaceStatus {
    pub state: NavmeshBakeState,
    pub queued_rebake: bool,
    pub generation: u64,
    pub polygon_count: u32,
    pub portal_count: u32,
    pub link_count: u32,
    pub source_count: u32,
    pub has_dirty_bounds: bool,
    pub dirty_bounds_min: Vec3,
    pub dirty_bounds_max: Vec3,
    pub next_bake_at_seconds: f64,
    pub last_bake_ms: f32,
    pub last_error: Option<String>,
}

impl Default for NavmeshSurfaceStatus {
    fn default() -> Self {
        Self {
            state: NavmeshBakeState::Dirty,
            queued_rebake: false,
            generation: 0,
            polygon_count: 0,
            portal_count: 0,
            link_count: 0,
            source_count: 0,
            has_dirty_bounds: false,
            dirty_bounds_min: Vec3::ZERO,
            dirty_bounds_max: Vec3::ZERO,
            next_bake_at_seconds: 0.0,
            last_bake_ms: 0.0,
            last_error: None,
        }
    }
}

#[derive(Component, Debug, Clone, PartialEq, Reflect)]
#[reflect(Component, Default)]
pub struct NavmeshSource {
    pub surface: Entity,
    pub kind: NavmeshSourceKind,
    pub area: NavmeshArea,
    pub mask: NavmeshAreaMask,
    pub enabled: bool,
}

impl NavmeshSource {
    pub fn new(surface: Entity, kind: NavmeshSourceKind) -> Self {
        Self {
            surface,
            kind,
            area: NavmeshArea::default(),
            mask: NavmeshAreaMask::all(),
            enabled: true,
        }
    }

    pub fn with_area(mut self, area: NavmeshArea) -> Self {
        self.area = area;
        self
    }

    pub fn with_mask(mut self, mask: NavmeshAreaMask) -> Self {
        self.mask = mask;
        self
    }
}

impl Default for NavmeshSource {
    fn default() -> Self {
        Self::new(Entity::PLACEHOLDER, NavmeshSourceKind::Walkable)
    }
}

#[derive(Component, Debug, Clone, PartialEq, Reflect)]
#[reflect(Component, Default)]
pub struct NavmeshLinkSource {
    pub surface: Entity,
    pub link: NavmeshOffMeshLink,
    pub enabled: bool,
}

impl NavmeshLinkSource {
    pub fn new(surface: Entity, link: NavmeshOffMeshLink) -> Self {
        Self {
            surface,
            link,
            enabled: true,
        }
    }
}

impl Default for NavmeshLinkSource {
    fn default() -> Self {
        Self::new(Entity::PLACEHOLDER, NavmeshOffMeshLink::default())
    }
}

#[derive(Component, Debug, Clone, PartialEq, Reflect)]
#[reflect(Component, Default)]
pub struct NavmeshPathRequest {
    pub surface: Entity,
    pub request_id: NavmeshPathId,
    pub start: Vec3,
    pub goal: Vec3,
    pub settings: NavmeshQuerySettings,
    pub filter: NavmeshQueryFilter,
}

impl NavmeshPathRequest {
    pub fn new(surface: Entity, request_id: NavmeshPathId, start: Vec3, goal: Vec3) -> Self {
        Self {
            surface,
            request_id,
            start,
            goal,
            settings: NavmeshQuerySettings::default(),
            filter: NavmeshQueryFilter::default(),
        }
    }
}

impl Default for NavmeshPathRequest {
    fn default() -> Self {
        Self::new(
            Entity::PLACEHOLDER,
            NavmeshPathId::default(),
            Vec3::ZERO,
            Vec3::ZERO,
        )
    }
}

#[derive(Component, Debug, Clone, PartialEq, Reflect, Default)]
#[reflect(Component, Default)]
pub struct NavmeshPathResult {
    pub result: NavmeshPathQueryResult,
}

#[derive(Component, Debug, Clone, PartialEq, Reflect)]
#[reflect(Component)]
pub struct NavmeshAgent {
    pub surface: Entity,
    pub max_speed: f32,
    pub arrival_distance: f32,
    pub waypoint_distance: f32,
    pub overshoot_distance: f32,
    pub repath_interval_seconds: f32,
    pub filter: NavmeshQueryFilter,
    pub query_settings: NavmeshQuerySettings,
}

impl NavmeshAgent {
    pub fn new(surface: Entity) -> Self {
        Self {
            surface,
            max_speed: 3.0,
            arrival_distance: 0.25,
            waypoint_distance: 0.2,
            overshoot_distance: 0.1,
            repath_interval_seconds: 0.35,
            filter: NavmeshQueryFilter::default(),
            query_settings: NavmeshQuerySettings::default(),
        }
    }

    pub fn with_max_speed(mut self, max_speed: f32) -> Self {
        self.max_speed = max_speed;
        self
    }

    pub fn with_filter(mut self, filter: NavmeshQueryFilter) -> Self {
        self.filter = filter;
        self
    }

    pub fn with_query_settings(mut self, query_settings: NavmeshQuerySettings) -> Self {
        self.query_settings = query_settings;
        self
    }
}

#[derive(Component, Debug, Clone, PartialEq, Reflect)]
pub enum NavmeshFollowTarget {
    Point(Vec3),
    Entity { entity: Entity, offset: Vec3 },
}

impl Default for NavmeshFollowTarget {
    fn default() -> Self {
        Self::Point(Vec3::ZERO)
    }
}

#[derive(Component, Debug, Clone, PartialEq, Reflect)]
#[reflect(Component, Default)]
pub struct NavmeshFollowerState {
    pub current_request_id: NavmeshPathId,
    pub active_path_request_id: NavmeshPathId,
    pub active_generation: u64,
    pub waypoint_index: usize,
    pub stale_path: bool,
    pub reached_goal: bool,
    pub has_resolved_target: bool,
    pub resolved_target: Vec3,
    pub next_repath_at_seconds: f64,
}

impl Default for NavmeshFollowerState {
    fn default() -> Self {
        Self {
            current_request_id: NavmeshPathId::default(),
            active_path_request_id: NavmeshPathId::default(),
            active_generation: 0,
            waypoint_index: 0,
            stale_path: true,
            reached_goal: false,
            has_resolved_target: false,
            resolved_target: Vec3::ZERO,
            next_repath_at_seconds: 0.0,
        }
    }
}

#[derive(Component, Debug, Clone, PartialEq, Reflect)]
#[reflect(Component, Default)]
pub struct NavmeshSteeringOutput {
    pub desired_direction: Vec3,
    pub desired_velocity: Vec3,
    pub next_target: Option<Vec3>,
    pub remaining_distance: f32,
    pub reached_goal: bool,
    pub path_status: NavmeshPathStatus,
}

impl Default for NavmeshSteeringOutput {
    fn default() -> Self {
        Self {
            desired_direction: Vec3::ZERO,
            desired_velocity: Vec3::ZERO,
            next_target: None,
            remaining_distance: 0.0,
            reached_goal: false,
            path_status: NavmeshPathStatus::Pending,
        }
    }
}
