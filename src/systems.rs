use std::collections::{HashMap, HashSet};

use bevy::{
    asset::AssetEvent,
    ecs::schedule::ScheduleLabel,
    mesh::Mesh,
    prelude::*,
    tasks::{AsyncComputeTaskPool, futures_lite::future},
};

use crate::{
    bake::{NavmeshSurfaceData, bake_navmesh_with_generation},
    components::{
        NavmeshAgent, NavmeshFollowTarget, NavmeshFollowerState, NavmeshLinkSource,
        NavmeshPathRequest, NavmeshPathResult, NavmeshSource, NavmeshSteeringOutput,
        NavmeshSurface, NavmeshSurfaceStatus,
    },
    config::{NavmeshBakeSettings, NavmeshBakeState},
    follow,
    geometry::{NavmeshBuildInput, NavmeshSourceGeometry, triangle_soup_from_mesh},
    messages::{
        NavmeshBakeCompleted, NavmeshDirtyReason, NavmeshPathInvalidated, NavmeshPathReady,
        NavmeshRebuildRequested,
    },
    path::{NavmeshPathId, NavmeshPathQueryResult, NavmeshPathStatus},
    resources::{NavmeshBakeOutcome, NavmeshDiagnostics, NavmeshRuntime},
};

#[derive(ScheduleLabel, Debug, Clone, PartialEq, Eq, Hash)]
pub struct NeverDeactivateSchedule;

pub(crate) fn setup_navmesh_entities(
    mut commands: Commands,
    surfaces_without_settings: Query<Entity, (With<NavmeshSurface>, Without<NavmeshBakeSettings>)>,
    surfaces_without_status: Query<Entity, (With<NavmeshSurface>, Without<NavmeshSurfaceStatus>)>,
) {
    for entity in &surfaces_without_settings {
        commands
            .entity(entity)
            .insert(NavmeshBakeSettings::default());
    }
    for entity in &surfaces_without_status {
        commands
            .entity(entity)
            .insert(NavmeshSurfaceStatus::default());
    }
}

pub(crate) fn setup_navmesh_agents(
    mut commands: Commands,
    agents_without_state: Query<Entity, (With<NavmeshAgent>, Without<NavmeshFollowerState>)>,
    agents_without_output: Query<Entity, (With<NavmeshAgent>, Without<NavmeshSteeringOutput>)>,
) {
    for entity in &agents_without_state {
        commands
            .entity(entity)
            .insert(NavmeshFollowerState::default());
    }
    for entity in &agents_without_output {
        commands
            .entity(entity)
            .insert(NavmeshSteeringOutput::default());
    }
}

pub(crate) fn cleanup_navmesh_runtime(mut runtime: ResMut<NavmeshRuntime>) {
    runtime.bake_tasks.clear();
    runtime.completed_bakes.clear();
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn detect_navmesh_changes(
    time: Res<Time>,
    meshes: Res<Assets<Mesh>>,
    mut diagnostics: ResMut<NavmeshDiagnostics>,
    mut mesh_events: MessageReader<AssetEvent<Mesh>>,
    mut rebuild_requests: MessageReader<NavmeshRebuildRequested>,
    sources: Query<(
        Entity,
        &NavmeshSource,
        Option<&crate::geometry::NavmeshPrimitiveSource>,
        Option<&Mesh3d>,
        Option<&GlobalTransform>,
    )>,
    changed_sources: Query<
        (
            Entity,
            &NavmeshSource,
            Option<&crate::geometry::NavmeshPrimitiveSource>,
            Option<&Mesh3d>,
            Option<&GlobalTransform>,
        ),
        Or<(
            Added<NavmeshSource>,
            Changed<NavmeshSource>,
            Changed<crate::geometry::NavmeshPrimitiveSource>,
            Changed<Mesh3d>,
            Changed<GlobalTransform>,
        )>,
    >,
    links: Query<&NavmeshLinkSource>,
    changed_links: Query<
        &NavmeshLinkSource,
        Or<(Added<NavmeshLinkSource>, Changed<NavmeshLinkSource>)>,
    >,
    mesh_sources: Query<(
        &NavmeshSource,
        &Mesh3d,
        Option<&crate::geometry::NavmeshPrimitiveSource>,
        Option<&GlobalTransform>,
    )>,
    mut surfaces: Query<(
        Entity,
        &NavmeshSurface,
        &NavmeshBakeSettings,
        &mut NavmeshSurfaceStatus,
    )>,
) {
    let now = time.elapsed_secs_f64();

    let mut changed_mesh_ids = HashSet::new();
    for event in mesh_events.read() {
        let id = match event {
            AssetEvent::Added { id }
            | AssetEvent::Modified { id }
            | AssetEvent::Removed { id }
            | AssetEvent::Unused { id }
            | AssetEvent::LoadedWithDependencies { id } => *id,
        };
        changed_mesh_ids.insert(id);
    }

    let mut source_counts: HashMap<Entity, u32> = HashMap::new();
    for (_, source, _, _, _) in &sources {
        if source.enabled {
            *source_counts.entry(source.surface).or_default() += 1;
        }
    }
    for link in &links {
        if link.enabled {
            *source_counts.entry(link.surface).or_default() += 1;
        }
    }

    for request in rebuild_requests.read() {
        if let Ok((_, _, settings, mut status)) = surfaces.get_mut(request.surface) {
            mark_surface_dirty(&mut status, settings, now, None);
        }
    }

    for (entity, source, primitive, mesh_handle, transform) in &changed_sources {
        if !source.enabled {
            continue;
        }
        if let Ok((_, _, settings, mut status)) = surfaces.get_mut(source.surface) {
            let bounds = source_bounds(entity, source, primitive, mesh_handle, transform, &meshes);
            mark_surface_dirty(&mut status, settings, now, bounds);
        }
    }

    for link in &changed_links {
        if !link.enabled {
            continue;
        }
        if let Ok((_, _, settings, mut status)) = surfaces.get_mut(link.surface) {
            let min = link.link.start.min(link.link.end);
            let max = link.link.start.max(link.link.end);
            mark_surface_dirty(&mut status, settings, now, Some((min, max)));
        }
    }

    if !changed_mesh_ids.is_empty() {
        for (source, mesh_handle, primitive, transform) in &mesh_sources {
            if !source.enabled || !changed_mesh_ids.contains(&mesh_handle.0.id()) {
                continue;
            }
            if let Ok((_, _, settings, mut status)) = surfaces.get_mut(source.surface) {
                let bounds = source_bounds(
                    source.surface,
                    source,
                    primitive,
                    Some(mesh_handle),
                    transform,
                    &meshes,
                );
                mark_surface_dirty(&mut status, settings, now, bounds);
            }
        }
    }

    for (surface_entity, surface, settings, mut status) in &mut surfaces {
        if !surface.enabled {
            continue;
        }
        let count = source_counts
            .get(&surface_entity)
            .copied()
            .unwrap_or_default();
        if status.source_count != count || matches!(status.state, NavmeshBakeState::Empty) {
            mark_surface_dirty(&mut status, settings, now, None);
        }
    }

    diagnostics.queued_bakes = surfaces
        .iter()
        .filter(|(_, surface, _, status)| {
            surface.enabled
                && (matches!(status.state, NavmeshBakeState::Dirty) || status.queued_rebake)
        })
        .count() as u32;
}

pub(crate) fn start_navmesh_bakes(
    time: Res<Time>,
    meshes: Res<Assets<Mesh>>,
    mut diagnostics: ResMut<NavmeshDiagnostics>,
    mut runtime: ResMut<NavmeshRuntime>,
    sources: Query<(
        Entity,
        &NavmeshSource,
        Option<&crate::geometry::NavmeshPrimitiveSource>,
        Option<&Mesh3d>,
        Option<&GlobalTransform>,
    )>,
    links: Query<&NavmeshLinkSource>,
    mut surfaces: Query<(
        Entity,
        &NavmeshSurface,
        &NavmeshBakeSettings,
        &mut NavmeshSurfaceStatus,
    )>,
) {
    let now = time.elapsed_secs_f64();

    for (surface_entity, surface, settings, mut status) in &mut surfaces {
        if !surface.enabled || runtime.bake_tasks.contains_key(&surface_entity) {
            continue;
        }
        if !matches!(status.state, NavmeshBakeState::Dirty) && !status.queued_rebake {
            continue;
        }
        if now < status.next_bake_at_seconds {
            continue;
        }

        let input = collect_build_input(surface_entity, &sources, &links, &meshes);
        let generation = status.generation + 1;
        let source_count = (input.sources.len() + input.links.len()) as u32;

        status.state = NavmeshBakeState::Baking;
        status.queued_rebake = false;
        status.last_error = None;

        if settings.async_baking {
            let bake_settings = settings.clone();
            let task = AsyncComputeTaskPool::get().spawn(async move {
                NavmeshBakeOutcome {
                    surface: surface_entity,
                    generation,
                    source_count,
                    result: bake_navmesh_with_generation(&input, &bake_settings, generation),
                }
            });
            runtime.bake_tasks.insert(surface_entity, task);
        } else {
            runtime.completed_bakes.push(NavmeshBakeOutcome {
                surface: surface_entity,
                generation,
                source_count,
                result: bake_navmesh_with_generation(&input, settings, generation),
            });
        }
    }

    diagnostics.active_bakes = runtime.bake_tasks.len() as u32;
}

pub(crate) fn poll_navmesh_bakes(
    time: Res<Time>,
    mut commands: Commands,
    mut diagnostics: ResMut<NavmeshDiagnostics>,
    mut runtime: ResMut<NavmeshRuntime>,
    mut completed: MessageWriter<NavmeshBakeCompleted>,
    mut surfaces: Query<(
        Entity,
        &NavmeshBakeSettings,
        &mut NavmeshSurfaceStatus,
        Option<&NavmeshSurfaceData>,
    )>,
) {
    let mut finished = Vec::new();
    for (&surface, task) in &mut runtime.bake_tasks {
        if let Some(outcome) = future::block_on(future::poll_once(task)) {
            finished.push((surface, outcome));
        }
    }
    for (surface, _) in &finished {
        runtime.bake_tasks.remove(surface);
    }
    runtime
        .completed_bakes
        .extend(finished.into_iter().map(|(_, outcome)| outcome));

    let outcomes = std::mem::take(&mut runtime.completed_bakes);
    let now = time.elapsed_secs_f64();
    for outcome in outcomes {
        let Ok((surface_entity, settings, mut status, _existing_data)) =
            surfaces.get_mut(outcome.surface)
        else {
            continue;
        };

        match outcome.result {
            Ok(data) => {
                let stats = data.stats.clone();
                let generation = outcome.generation;
                commands.entity(surface_entity).insert(data);
                status.generation = generation;
                status.polygon_count = stats.polygon_count;
                status.portal_count = stats.portal_count;
                status.link_count = stats.link_count;
                status.source_count = outcome.source_count;
                status.last_bake_ms = stats.last_bake_ms;
                status.last_error = None;
                if status.queued_rebake {
                    status.state = NavmeshBakeState::Dirty;
                    status.next_bake_at_seconds = now + settings.rebuild_debounce_seconds as f64;
                } else {
                    status.state = NavmeshBakeState::Ready;
                    clear_dirty_bounds(&mut status);
                }
                diagnostics.last_bake_ms = stats.last_bake_ms;
                diagnostics.last_surface = Some(surface_entity);
                diagnostics.last_failure = None;
                completed.write(NavmeshBakeCompleted {
                    surface: surface_entity,
                    generation,
                    success: true,
                });
            }
            Err(error) => {
                status.source_count = outcome.source_count;
                status.last_error = Some(error.message.clone());
                status.state = if status.queued_rebake {
                    NavmeshBakeState::Dirty
                } else {
                    NavmeshBakeState::Failed
                };
                status.next_bake_at_seconds = now + settings.rebuild_debounce_seconds as f64;
                diagnostics.last_failure = Some(error.message);
                diagnostics.last_surface = Some(surface_entity);
                completed.write(NavmeshBakeCompleted {
                    surface: surface_entity,
                    generation: status.generation,
                    success: false,
                });
            }
        }
    }

    diagnostics.active_bakes = runtime.bake_tasks.len() as u32;
}

pub(crate) fn invalidate_stale_path_results(
    mut invalidated: MessageWriter<NavmeshPathInvalidated>,
    surfaces: Query<&NavmeshSurfaceStatus>,
    mut requests: Query<(Entity, &NavmeshPathRequest, &mut NavmeshPathResult)>,
) {
    for (entity, request, mut path_result) in &mut requests {
        let Ok(status) = surfaces.get(request.surface) else {
            continue;
        };
        let should_invalidate = !matches!(
            status.state,
            NavmeshBakeState::Ready | NavmeshBakeState::Failed
        ) || path_result.result.generation != status.generation;
        if should_invalidate && !matches!(path_result.result.status, NavmeshPathStatus::Pending) {
            path_result.result.status = NavmeshPathStatus::Pending;
            path_result.result.path = None;
            invalidated.write(NavmeshPathInvalidated {
                entity,
                surface: request.surface,
                generation: status.generation,
                reason: NavmeshDirtyReason::SourceChanged,
            });
        }
    }
}

pub(crate) fn process_path_requests(
    mut commands: Commands,
    mut diagnostics: ResMut<NavmeshDiagnostics>,
    mut ready: MessageWriter<NavmeshPathReady>,
    surfaces: Query<(&NavmeshSurfaceStatus, Option<&NavmeshSurfaceData>)>,
    requests: Query<(Entity, &NavmeshPathRequest, Option<&NavmeshPathResult>)>,
) {
    for (entity, request, existing_result) in &requests {
        let Ok((status, surface_data)) = surfaces.get(request.surface) else {
            commands.entity(entity).insert(NavmeshPathResult {
                result: NavmeshPathQueryResult {
                    request_id: request.request_id,
                    status: NavmeshPathStatus::InvalidSurface,
                    ..default()
                },
            });
            continue;
        };

        let needs_update = existing_result.is_none_or(|current| {
            current.result.request_id != request.request_id
                || current.result.generation != status.generation
                || matches!(current.result.status, NavmeshPathStatus::Pending)
        });
        if !needs_update {
            continue;
        }

        let next_result = if let Some(surface_data) = surface_data {
            if matches!(
                status.state,
                NavmeshBakeState::Ready | NavmeshBakeState::Failed
            ) {
                let result = surface_data.query_path(
                    request.request_id,
                    request.start,
                    request.goal,
                    &request.settings,
                    &request.filter,
                );
                diagnostics.last_query_ms = result.duration_ms;
                diagnostics.completed_queries += 1;
                diagnostics.last_surface = Some(request.surface);
                if !matches!(result.status, NavmeshPathStatus::Pending) {
                    ready.write(NavmeshPathReady {
                        entity,
                        surface: request.surface,
                        request_id: request.request_id,
                        status: result.status,
                    });
                }
                result
            } else {
                NavmeshPathQueryResult {
                    request_id: request.request_id,
                    status: NavmeshPathStatus::Pending,
                    generation: status.generation,
                    ..default()
                }
            }
        } else {
            NavmeshPathQueryResult {
                request_id: request.request_id,
                status: NavmeshPathStatus::InvalidSurface,
                generation: status.generation,
                ..default()
            }
        };

        commands.entity(entity).insert(NavmeshPathResult {
            result: next_result,
        });
    }
}

pub(crate) fn drive_follow_requests(
    time: Res<Time>,
    mut commands: Commands,
    mut runtime: ResMut<NavmeshRuntime>,
    surfaces: Query<&NavmeshSurfaceStatus>,
    target_transforms: Query<&GlobalTransform>,
    mut agents: Query<(
        Entity,
        &NavmeshAgent,
        &NavmeshFollowTarget,
        &GlobalTransform,
        Option<&NavmeshPathRequest>,
        Option<&NavmeshPathResult>,
        &mut NavmeshFollowerState,
    )>,
) {
    let now = time.elapsed_secs_f64();
    for (entity, agent, target, transform, path_request, path_result, mut state) in &mut agents {
        let resolved_target = match target {
            NavmeshFollowTarget::Point(point) => *point,
            NavmeshFollowTarget::Entity { entity, offset } => target_transforms
                .get(*entity)
                .map(|target_transform| target_transform.translation() + *offset)
                .unwrap_or(state.resolved_target),
        };

        let surface_generation = surfaces
            .get(agent.surface)
            .map(|status| status.generation)
            .unwrap_or_default();

        let target_changed = !state.has_resolved_target
            || state.resolved_target.distance(resolved_target) > agent.arrival_distance.max(0.1);
        let request_pending =
            path_request.is_some_and(|request| request.request_id == state.current_request_id);
        let path_missing = path_result.is_none() && !request_pending;
        let generation_mismatch = path_result
            .is_some_and(|result| result.result.generation != surface_generation)
            && !request_pending;
        let request_mismatch = path_result.is_some_and(|result| {
            result.result.request_id != state.current_request_id && !request_pending
        });
        let terminal_status = path_result.is_some_and(|result| {
            matches!(
                result.result.status,
                NavmeshPathStatus::Unreachable | NavmeshPathStatus::InvalidSurface
            )
        }) && !request_pending;
        let stale_path = state.stale_path && !request_pending;

        let needs_repath = target_changed
            || path_missing
            || generation_mismatch
            || request_mismatch
            || terminal_status
            || stale_path
            || now >= state.next_repath_at_seconds
            || state.current_request_id == NavmeshPathId::default();

        if needs_repath {
            let request_id = next_path_id(&mut runtime);
            commands.entity(entity).insert(NavmeshPathRequest {
                surface: agent.surface,
                request_id,
                start: transform.translation(),
                goal: resolved_target,
                settings: agent.query_settings.clone(),
                filter: agent.filter.clone(),
            });
            state.current_request_id = request_id;
            state.stale_path = false;
            state.reached_goal = false;
            state.has_resolved_target = true;
            state.resolved_target = resolved_target;
            state.next_repath_at_seconds = now + agent.repath_interval_seconds as f64;
        }
    }
}

pub(crate) fn update_follow_outputs(
    surfaces: Query<&NavmeshSurfaceStatus>,
    mut agents: ParamSet<(
        Query<
            (
                Entity,
                &NavmeshAgent,
                &GlobalTransform,
                Option<&NavmeshPathResult>,
                &mut NavmeshFollowerState,
                &mut NavmeshSteeringOutput,
                Option<&crate::components::NavmeshCrowdAvoidance>,
            ),
        >,
        Query<
            (
                Entity,
                &NavmeshAgent,
                &GlobalTransform,
                Option<&crate::components::NavmeshCrowdAvoidance>,
                Option<&NavmeshSteeringOutput>,
            ),
        >,
    )>,
) {
    let crowd_snapshots = agents
        .p1()
        .iter()
        .map(
            |(entity, agent, transform, crowd, output)| CrowdSnapshot {
                entity,
                surface: agent.surface,
                position: transform.translation(),
                desired_velocity: output.map(|value| value.desired_velocity).unwrap_or(Vec3::ZERO),
                body_radius: crowd.map(|crowd| crowd.body_radius).unwrap_or(0.35),
            },
        )
        .collect::<Vec<_>>();

    for (entity, agent, transform, path_result, mut state, mut output, crowd_avoidance) in
        &mut agents.p0()
    {
        *output = NavmeshSteeringOutput::default();

        let Some(path_result) = path_result else {
            state.stale_path = true;
            continue;
        };
        output.path_status = path_result.result.status;
        let Ok(surface_status) = surfaces.get(agent.surface) else {
            state.stale_path = true;
            continue;
        };

        if path_result.result.request_id != state.current_request_id
            || !matches!(
                path_result.result.status,
                NavmeshPathStatus::Success | NavmeshPathStatus::Partial
            )
        {
            state.stale_path = true;
            continue;
        }

        let Some(path) = &path_result.result.path else {
            state.stale_path = true;
            continue;
        };
        if path.points.is_empty() {
            state.stale_path = true;
            continue;
        }

        if state.active_generation != path_result.result.generation
            || state.active_path_request_id != path_result.result.request_id
        {
            state.active_generation = path_result.result.generation;
            state.active_path_request_id = path_result.result.request_id;
            state.waypoint_index = if path.points.len() > 1 { 1 } else { 0 };
        }

        state.waypoint_index = follow::advance_waypoint_index(
            agent,
            path,
            state.waypoint_index,
            transform.translation(),
        );
        let index = state
            .waypoint_index
            .min(path.points.len().saturating_sub(1));
        let next_target = path.points[index].position;
        let remaining = follow::remaining_distance(path, index, transform.translation());
        output.remaining_distance = remaining;
        output.next_target = Some(next_target);

        if remaining <= agent.arrival_distance {
            state.reached_goal = true;
            state.stale_path = false;
            output.reached_goal = true;
            continue;
        }

        let to_target = next_target - transform.translation();
        let direction = to_target.normalize_or_zero();
        let mut desired_velocity = direction * agent.max_speed;
        if let Some(crowd_avoidance) = crowd_avoidance {
            let (adjusted_velocity, crowd_neighbors) = follow::apply_crowd_avoidance(
                transform.translation(),
                desired_velocity,
                crowd_avoidance,
                crowd_snapshots
                    .iter()
                    .filter(|neighbor| {
                        neighbor.entity != entity && neighbor.surface == agent.surface
                    })
                    .map(|neighbor| follow::CrowdNeighbor {
                        position: neighbor.position,
                        desired_velocity: neighbor.desired_velocity,
                        body_radius: neighbor.body_radius,
                    }),
            );
            desired_velocity = adjusted_velocity;
            output.crowd_neighbors = crowd_neighbors;
        }
        output.desired_direction = desired_velocity.normalize_or_zero();
        output.desired_velocity = desired_velocity;
        output.reached_goal = false;
        state.reached_goal = false;
        state.stale_path = path_result.result.generation != surface_status.generation;
    }
}

fn next_path_id(runtime: &mut NavmeshRuntime) -> NavmeshPathId {
    runtime.next_path_id += 1;
    NavmeshPathId(runtime.next_path_id)
}

fn mark_surface_dirty(
    status: &mut NavmeshSurfaceStatus,
    settings: &NavmeshBakeSettings,
    now: f64,
    bounds: Option<(Vec3, Vec3)>,
) {
    if matches!(status.state, NavmeshBakeState::Baking) {
        status.queued_rebake = true;
    } else {
        status.state = NavmeshBakeState::Dirty;
    }
    status.next_bake_at_seconds = now + settings.rebuild_debounce_seconds as f64;
    if let Some((min, max)) = bounds {
        merge_dirty_bounds(status, min, max);
    }
}

fn merge_dirty_bounds(status: &mut NavmeshSurfaceStatus, min: Vec3, max: Vec3) {
    if status.has_dirty_bounds {
        status.dirty_bounds_min = status.dirty_bounds_min.min(min);
        status.dirty_bounds_max = status.dirty_bounds_max.max(max);
    } else {
        status.has_dirty_bounds = true;
        status.dirty_bounds_min = min;
        status.dirty_bounds_max = max;
    }
}

fn clear_dirty_bounds(status: &mut NavmeshSurfaceStatus) {
    status.has_dirty_bounds = false;
    status.dirty_bounds_min = Vec3::ZERO;
    status.dirty_bounds_max = Vec3::ZERO;
}

#[derive(Clone, Copy, Debug)]
struct CrowdSnapshot {
    entity: Entity,
    surface: Entity,
    position: Vec3,
    desired_velocity: Vec3,
    body_radius: f32,
}

fn collect_build_input(
    surface: Entity,
    sources: &Query<(
        Entity,
        &NavmeshSource,
        Option<&crate::geometry::NavmeshPrimitiveSource>,
        Option<&Mesh3d>,
        Option<&GlobalTransform>,
    )>,
    links: &Query<&NavmeshLinkSource>,
    meshes: &Assets<Mesh>,
) -> NavmeshBuildInput {
    let mut input = NavmeshBuildInput::default();
    for (entity, source, primitive, mesh_handle, transform) in sources.iter() {
        if !source.enabled || source.surface != surface {
            continue;
        }
        if let Some(geometry) =
            collect_source_geometry(entity, source, primitive, mesh_handle, transform, meshes)
        {
            input.sources.push(geometry);
        }
    }
    for link in links.iter() {
        if link.enabled && link.surface == surface {
            input.links.push(link.link.clone());
        }
    }
    input
}

fn collect_source_geometry(
    entity: Entity,
    source: &NavmeshSource,
    primitive: Option<&crate::geometry::NavmeshPrimitiveSource>,
    mesh_handle: Option<&Mesh3d>,
    transform: Option<&GlobalTransform>,
    meshes: &Assets<Mesh>,
) -> Option<NavmeshSourceGeometry> {
    let transform = transform
        .map(GlobalTransform::compute_transform)
        .unwrap_or_default();

    let triangles = if let Some(primitive) = primitive {
        primitive.triangles().transformed(transform)
    } else if let Some(mesh_handle) = mesh_handle {
        let mesh = meshes.get(mesh_handle.0.id())?;
        triangle_soup_from_mesh(mesh)?.transformed(transform)
    } else {
        return None;
    };

    Some(NavmeshSourceGeometry {
        source_id: entity.to_bits(),
        kind: source.kind,
        area: source.area,
        mask: source.mask,
        triangles,
    })
}

fn source_bounds(
    entity: Entity,
    source: &NavmeshSource,
    primitive: Option<&crate::geometry::NavmeshPrimitiveSource>,
    mesh_handle: Option<&Mesh3d>,
    transform: Option<&GlobalTransform>,
    meshes: &Assets<Mesh>,
) -> Option<(Vec3, Vec3)> {
    collect_source_geometry(entity, source, primitive, mesh_handle, transform, meshes)?.aabb()
}

#[cfg(test)]
#[path = "systems_tests.rs"]
mod tests;
