use bevy::{
    asset::{AssetApp, AssetPlugin},
    prelude::*,
};

use super::*;
use crate::{
    NavmeshPlugin, NavmeshPrimitive, NavmeshPrimitiveSource, NavmeshSourceKind,
    components::{
        NavmeshAgent, NavmeshCrowdAvoidance, NavmeshFollowTarget, NavmeshFollowerState,
        NavmeshPathRequest, NavmeshSource, NavmeshSteeringOutput, NavmeshSurface,
        NavmeshSurfaceStatus,
    },
    config::{NavmeshBakeSettings, NavmeshBakeState},
    path::{NavmeshPathId, NavmeshPathStatus},
};

fn test_app() -> App {
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        AssetPlugin::default(),
        NavmeshPlugin::default(),
    ));
    app.init_asset::<Mesh>();
    app
}

fn spawn_surface(app: &mut App) -> (Entity, Entity) {
    let surface = app
        .world_mut()
        .spawn((
            NavmeshSurface::default(),
            NavmeshBakeSettings {
                async_baking: false,
                rebuild_debounce_seconds: 0.0,
                ..default()
            },
            NavmeshSurfaceStatus::default(),
        ))
        .id();
    let source = app
        .world_mut()
        .spawn((
            NavmeshSource::new(surface, NavmeshSourceKind::Walkable),
            NavmeshPrimitiveSource::new(NavmeshPrimitive::Quad {
                size: Vec2::new(4.0, 4.0),
            }),
            GlobalTransform::default(),
        ))
        .id();

    (surface, source)
}

fn surface_status(world: &World, entity: Entity) -> NavmeshSurfaceStatus {
    world
        .entity(entity)
        .get::<NavmeshSurfaceStatus>()
        .unwrap()
        .clone()
}

#[test]
fn surface_bakes_into_runtime_data() {
    let mut app = test_app();
    let (surface, _) = spawn_surface(&mut app);

    app.update();

    let status = surface_status(app.world(), surface);
    let data = app
        .world()
        .entity(surface)
        .get::<NavmeshSurfaceData>()
        .unwrap();

    assert_eq!(status.state, NavmeshBakeState::Ready);
    assert_eq!(status.generation, 1);
    assert_eq!(data.polygons.len(), 2);
    assert_eq!(data.portals.len(), 1);
}

#[test]
fn path_request_component_produces_a_result_component() {
    let mut app = test_app();
    let (surface, _) = spawn_surface(&mut app);
    app.update();

    let entity = app
        .world_mut()
        .spawn(NavmeshPathRequest::new(
            surface,
            NavmeshPathId(11),
            Vec3::new(-1.5, 0.0, -1.5),
            Vec3::new(1.5, 0.0, 1.5),
        ))
        .id();

    app.update();

    let result = app
        .world()
        .entity(entity)
        .get::<crate::components::NavmeshPathResult>();
    let result = result.expect("path request should resolve into a result");
    assert_eq!(result.result.request_id, NavmeshPathId(11));
    assert_eq!(result.result.status, NavmeshPathStatus::Success);
    assert!(result.result.path.is_some());
}

#[test]
fn follower_state_emits_steering_after_request_and_query_frames() {
    let mut app = test_app();
    let (surface, _) = spawn_surface(&mut app);
    app.update();

    let agent = app
        .world_mut()
        .spawn((
            NavmeshAgent::new(surface).with_max_speed(4.0),
            NavmeshFollowTarget::Point(Vec3::new(1.5, 0.0, 1.5)),
            NavmeshFollowerState::default(),
            NavmeshSteeringOutput::default(),
            GlobalTransform::from_translation(Vec3::new(-1.5, 0.0, -1.5)),
        ))
        .id();

    app.update();
    app.update();
    app.update();
    app.update();
    app.update();

    let output = app
        .world()
        .entity(agent)
        .get::<NavmeshSteeringOutput>()
        .unwrap();
    let state = app
        .world()
        .entity(agent)
        .get::<NavmeshFollowerState>()
        .unwrap();
    let result = app
        .world()
        .entity(agent)
        .get::<crate::components::NavmeshPathResult>()
        .unwrap();

    assert_eq!(result.result.status, NavmeshPathStatus::Success);
    assert!(output.next_target.is_some());
    assert!(output.desired_velocity.length() > 0.0);
    assert!(!state.stale_path);
}

#[test]
fn changed_source_marks_surface_dirty_and_rebakes() {
    let mut app = test_app();
    let (surface, source) = spawn_surface(&mut app);
    app.update();
    assert_eq!(surface_status(app.world(), surface).generation, 1);

    app.world_mut()
        .entity_mut(source)
        .insert(GlobalTransform::from_translation(Vec3::new(1.0, 0.0, 0.0)));

    app.update();

    let status = surface_status(app.world(), surface);
    assert_eq!(status.state, NavmeshBakeState::Ready);
    assert_eq!(status.generation, 2);
}

#[test]
fn path_results_refresh_after_surface_generation_changes() {
    let mut app = test_app();
    let (surface, source) = spawn_surface(&mut app);
    app.update();

    let agent = app
        .world_mut()
        .spawn((
            NavmeshAgent::new(surface).with_max_speed(4.0),
            NavmeshFollowTarget::Point(Vec3::new(1.5, 0.0, 1.5)),
            NavmeshFollowerState::default(),
            NavmeshSteeringOutput::default(),
            GlobalTransform::from_translation(Vec3::new(-1.5, 0.0, -1.5)),
        ))
        .id();

    for _ in 0..5 {
        app.update();
    }

    let original_request_id = app
        .world()
        .entity(agent)
        .get::<NavmeshFollowerState>()
        .unwrap()
        .current_request_id;

    app.world_mut()
        .entity_mut(source)
        .insert(GlobalTransform::from_translation(Vec3::new(0.5, 0.0, 0.0)));

    for _ in 0..5 {
        app.update();
    }

    let state = app
        .world()
        .entity(agent)
        .get::<NavmeshFollowerState>()
        .unwrap();
    let result = app
        .world()
        .entity(agent)
        .get::<crate::components::NavmeshPathResult>()
        .unwrap();

    assert_eq!(state.current_request_id, original_request_id);
    assert_eq!(result.result.status, NavmeshPathStatus::Success);
    assert_eq!(result.result.request_id, state.current_request_id);
    assert_eq!(result.result.generation, 2);
    assert_eq!(state.active_generation, 2);
    assert!(!state.stale_path);
}

#[test]
fn crowd_avoidance_deflects_head_on_followers() {
    let mut app = test_app();
    let (surface, _) = spawn_surface(&mut app);
    app.update();

    let left = app
        .world_mut()
        .spawn((
            NavmeshAgent::new(surface).with_max_speed(4.0),
            NavmeshCrowdAvoidance::default(),
            NavmeshFollowTarget::Point(Vec3::new(1.5, 0.0, 0.0)),
            NavmeshFollowerState::default(),
            NavmeshSteeringOutput::default(),
            GlobalTransform::from_translation(Vec3::new(-1.5, 0.0, 0.0)),
        ))
        .id();

    app.world_mut().spawn((
        NavmeshAgent::new(surface).with_max_speed(4.0),
        NavmeshCrowdAvoidance::default(),
        NavmeshFollowTarget::Point(Vec3::new(-1.5, 0.0, 0.0)),
        NavmeshFollowerState::default(),
        NavmeshSteeringOutput::default(),
        GlobalTransform::from_translation(Vec3::new(1.5, 0.0, 0.0)),
    ));

    for _ in 0..5 {
        app.update();
    }

    let output = app
        .world()
        .entity(left)
        .get::<NavmeshSteeringOutput>()
        .expect("crowd-avoidance follower should emit steering");

    assert!(output.crowd_neighbors > 0);
    assert!(output.desired_velocity.z.abs() > 0.05);
}
