use bevy::prelude::*;

use crate::path::{NavmeshPathId, NavmeshPathStatus};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Reflect, Default)]
pub enum NavmeshDirtyReason {
    #[default]
    Manual,
    SourceChanged,
    SourceCountChanged,
    MeshChanged,
    LinkChanged,
}

#[derive(Message, Debug, Clone, Copy, Reflect)]
pub struct NavmeshRebuildRequested {
    pub surface: Entity,
    pub reason: NavmeshDirtyReason,
}

impl Default for NavmeshRebuildRequested {
    fn default() -> Self {
        Self {
            surface: Entity::PLACEHOLDER,
            reason: NavmeshDirtyReason::Manual,
        }
    }
}

#[derive(Message, Debug, Clone, Copy, Reflect)]
pub struct NavmeshBakeCompleted {
    pub surface: Entity,
    pub generation: u64,
    pub success: bool,
}

impl Default for NavmeshBakeCompleted {
    fn default() -> Self {
        Self {
            surface: Entity::PLACEHOLDER,
            generation: 0,
            success: false,
        }
    }
}

#[derive(Message, Debug, Clone, Copy, Reflect)]
pub struct NavmeshPathReady {
    pub entity: Entity,
    pub surface: Entity,
    pub request_id: NavmeshPathId,
    pub status: NavmeshPathStatus,
}

impl Default for NavmeshPathReady {
    fn default() -> Self {
        Self {
            entity: Entity::PLACEHOLDER,
            surface: Entity::PLACEHOLDER,
            request_id: NavmeshPathId::default(),
            status: NavmeshPathStatus::Pending,
        }
    }
}

#[derive(Message, Debug, Clone, Copy, Reflect)]
pub struct NavmeshPathInvalidated {
    pub entity: Entity,
    pub surface: Entity,
    pub generation: u64,
    pub reason: NavmeshDirtyReason,
}

impl Default for NavmeshPathInvalidated {
    fn default() -> Self {
        Self {
            entity: Entity::PLACEHOLDER,
            surface: Entity::PLACEHOLDER,
            generation: 0,
            reason: NavmeshDirtyReason::Manual,
        }
    }
}
