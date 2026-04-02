use bevy::{ecs::entity::EntityHashMap, prelude::*, tasks::Task};

use crate::bake::NavmeshSurfaceData;

#[derive(Resource, Debug, Clone, PartialEq, Reflect)]
#[reflect(Resource, Default)]
pub struct NavmeshDiagnostics {
    pub active_bakes: u32,
    pub queued_bakes: u32,
    pub completed_queries: u64,
    pub last_bake_ms: f32,
    pub last_query_ms: f32,
    pub last_surface: Option<Entity>,
    pub last_failure: Option<String>,
}

impl Default for NavmeshDiagnostics {
    fn default() -> Self {
        Self {
            active_bakes: 0,
            queued_bakes: 0,
            completed_queries: 0,
            last_bake_ms: 0.0,
            last_query_ms: 0.0,
            last_surface: None,
            last_failure: None,
        }
    }
}

pub(crate) struct NavmeshBakeOutcome {
    pub surface: Entity,
    pub generation: u64,
    pub source_count: u32,
    pub result: Result<NavmeshSurfaceData, crate::bake::NavmeshBakeError>,
}

#[derive(Resource, Default)]
pub(crate) struct NavmeshRuntime {
    pub bake_tasks: EntityHashMap<Task<NavmeshBakeOutcome>>,
    pub completed_bakes: Vec<NavmeshBakeOutcome>,
    pub next_path_id: u64,
}
