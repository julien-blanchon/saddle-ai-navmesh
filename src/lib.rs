#![doc = include_str!("../README.md")]

mod bake;
mod components;
mod config;
mod debug;
mod follow;
mod geometry;
mod math;
mod messages;
mod path;
mod query;
mod resources;
mod systems;

pub use crate::bake::{
    NavmeshBakeError, NavmeshBakeStats, NavmeshBakedLink, NavmeshBasisData, NavmeshPolygon,
    NavmeshPortal, NavmeshSurfaceData, bake_navmesh,
};
pub use crate::components::{
    NavmeshAgent, NavmeshCrowdAvoidance, NavmeshFollowTarget, NavmeshFollowerState,
    NavmeshLinkSource, NavmeshPathRequest, NavmeshPathResult, NavmeshSource, NavmeshSteeringOutput,
    NavmeshSurface, NavmeshSurfaceStatus,
};
pub use crate::config::{
    NavmeshArea, NavmeshAreaCost, NavmeshAreaMask, NavmeshBakeSettings, NavmeshBakeState,
    NavmeshDebugSettings, NavmeshPathSmoothing, NavmeshProjectionPolicy, NavmeshQueryFilter,
    NavmeshQuerySettings,
};
pub use crate::geometry::{
    NavmeshBuildInput, NavmeshPrimitive, NavmeshPrimitiveSource, NavmeshSourceGeometry,
    NavmeshSourceKind, NavmeshTriangleSoup, triangle_soup_from_mesh,
};
pub use crate::messages::{
    NavmeshBakeCompleted, NavmeshDirtyReason, NavmeshPathInvalidated, NavmeshPathReady,
    NavmeshRebuildRequested,
};
pub use crate::path::{
    NavmeshCorridorPortal, NavmeshLinkId, NavmeshOffMeshLink, NavmeshPath, NavmeshPathId,
    NavmeshPathPoint, NavmeshPathQueryResult, NavmeshPathStatus, NavmeshPathTransition,
    NavmeshProjectionHit,
};
pub use crate::query::{nearest_point_on_navmesh, query_navmesh_path};
pub use crate::resources::NavmeshDiagnostics;

use bevy::{
    app::PostStartup,
    ecs::{intern::Interned, schedule::ScheduleLabel},
    prelude::*,
};

#[derive(SystemSet, Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub enum NavmeshSystems {
    DetectChanges,
    Bake,
    Query,
    Follow,
    Debug,
}

pub struct NavmeshPlugin {
    pub activate_schedule: Interned<dyn ScheduleLabel>,
    pub deactivate_schedule: Interned<dyn ScheduleLabel>,
    pub update_schedule: Interned<dyn ScheduleLabel>,
}

impl NavmeshPlugin {
    pub fn new(
        activate_schedule: impl ScheduleLabel,
        deactivate_schedule: impl ScheduleLabel,
        update_schedule: impl ScheduleLabel,
    ) -> Self {
        Self {
            activate_schedule: activate_schedule.intern(),
            deactivate_schedule: deactivate_schedule.intern(),
            update_schedule: update_schedule.intern(),
        }
    }

    pub fn always_on(update_schedule: impl ScheduleLabel) -> Self {
        Self::new(
            PostStartup,
            systems::NeverDeactivateSchedule,
            update_schedule,
        )
    }
}

impl Default for NavmeshPlugin {
    fn default() -> Self {
        Self::always_on(Update)
    }
}

impl Plugin for NavmeshPlugin {
    fn build(&self, app: &mut App) {
        if self.deactivate_schedule == systems::NeverDeactivateSchedule.intern() {
            app.init_schedule(systems::NeverDeactivateSchedule);
        }

        app.init_resource::<NavmeshDebugSettings>()
            .init_resource::<NavmeshDiagnostics>()
            .init_resource::<resources::NavmeshRuntime>()
            .add_message::<NavmeshRebuildRequested>()
            .add_message::<NavmeshBakeCompleted>()
            .add_message::<NavmeshPathReady>()
            .add_message::<NavmeshPathInvalidated>()
            .register_type::<NavmeshAgent>()
            .register_type::<NavmeshArea>()
            .register_type::<NavmeshAreaCost>()
            .register_type::<NavmeshAreaMask>()
            .register_type::<NavmeshBakeSettings>()
            .register_type::<NavmeshBasisData>()
            .register_type::<NavmeshBakedLink>()
            .register_type::<NavmeshCorridorPortal>()
            .register_type::<NavmeshCrowdAvoidance>()
            .register_type::<NavmeshDebugSettings>()
            .register_type::<NavmeshDiagnostics>()
            .register_type::<NavmeshFollowTarget>()
            .register_type::<NavmeshFollowerState>()
            .register_type::<NavmeshLinkId>()
            .register_type::<NavmeshLinkSource>()
            .register_type::<NavmeshOffMeshLink>()
            .register_type::<NavmeshPath>()
            .register_type::<NavmeshPathId>()
            .register_type::<NavmeshPathPoint>()
            .register_type::<NavmeshPathQueryResult>()
            .register_type::<NavmeshPathRequest>()
            .register_type::<NavmeshPathResult>()
            .register_type::<NavmeshPathStatus>()
            .register_type::<NavmeshPathTransition>()
            .register_type::<NavmeshPathSmoothing>()
            .register_type::<NavmeshPolygon>()
            .register_type::<NavmeshPortal>()
            .register_type::<NavmeshProjectionHit>()
            .register_type::<NavmeshProjectionPolicy>()
            .register_type::<NavmeshQueryFilter>()
            .register_type::<NavmeshQuerySettings>()
            .register_type::<NavmeshPrimitive>()
            .register_type::<NavmeshPrimitiveSource>()
            .register_type::<NavmeshSource>()
            .register_type::<NavmeshSourceGeometry>()
            .register_type::<NavmeshSourceKind>()
            .register_type::<NavmeshSteeringOutput>()
            .register_type::<NavmeshSurface>()
            .register_type::<NavmeshSurfaceData>()
            .register_type::<NavmeshSurfaceStatus>()
            .configure_sets(
                self.update_schedule,
                (
                    NavmeshSystems::DetectChanges,
                    NavmeshSystems::Bake,
                    NavmeshSystems::Query,
                    NavmeshSystems::Follow,
                    NavmeshSystems::Debug,
                )
                    .chain(),
            )
            .add_systems(self.activate_schedule, systems::setup_navmesh_entities)
            .add_systems(self.deactivate_schedule, systems::cleanup_navmesh_runtime)
            .add_systems(
                self.update_schedule,
                systems::setup_navmesh_entities.in_set(NavmeshSystems::DetectChanges),
            )
            .add_systems(
                self.update_schedule,
                systems::detect_navmesh_changes.in_set(NavmeshSystems::DetectChanges),
            )
            .add_systems(
                self.update_schedule,
                (systems::start_navmesh_bakes, systems::poll_navmesh_bakes)
                    .chain()
                    .in_set(NavmeshSystems::Bake),
            )
            .add_systems(
                self.update_schedule,
                (
                    systems::invalidate_stale_path_results,
                    systems::process_path_requests,
                )
                    .chain()
                    .in_set(NavmeshSystems::Query),
            )
            .add_systems(
                self.update_schedule,
                (
                    systems::setup_navmesh_agents,
                    systems::drive_follow_requests,
                    systems::update_follow_outputs,
                )
                    .chain()
                    .in_set(NavmeshSystems::Follow),
            )
            .add_systems(
                self.update_schedule,
                (
                    debug::apply_debug_settings,
                    debug::draw_navmesh_debug.run_if(debug::can_draw_debug),
                )
                    .chain()
                    .in_set(NavmeshSystems::Debug),
            );
    }
}
