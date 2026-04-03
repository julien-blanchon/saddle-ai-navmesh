use saddle_ai_navmesh_example_common as common;

use bevy::prelude::*;
use common::*;
use saddle_ai_navmesh::{NavmeshBakeSettings, NavmeshCrowdAvoidance, NavmeshFollowTarget};

#[derive(Resource)]
struct BridgePlan {
    surface: Entity,
    timer: Timer,
    spawned: bool,
}

#[derive(Component)]
struct GoalMarker;

#[derive(Component)]
struct DemoAgent;

fn main() {
    let mut app = App::new();
    app.insert_resource(NavmeshExamplePane {
        goal_x: 2.0,
        goal_z: 0.0,
        rebuild_debounce_seconds: 0.0,
        ..default()
    });
    configure_app(&mut app, "navmesh procedural rebake");
    app.add_systems(Startup, setup);
    app.add_systems(Update, (spawn_bridge_when_ready, sync_pane));
    app.run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    spawn_scene_camera(&mut commands);

    let surface = spawn_surface(
        &mut commands,
        NavmeshBakeSettings {
            async_baking: false,
            rebuild_debounce_seconds: 0.0,
            ..default()
        },
    );

    spawn_nav_tile(
        &mut commands,
        &mut meshes,
        &mut materials,
        surface,
        Vec3::new(-2.0, 0.0, 0.0),
        saddle_ai_navmesh::NavmeshArea(0),
        Color::srgb(0.20, 0.26, 0.20),
    );
    spawn_nav_tile(
        &mut commands,
        &mut meshes,
        &mut materials,
        surface,
        Vec3::new(2.0, 0.0, 0.0),
        saddle_ai_navmesh::NavmeshArea(0),
        Color::srgb(0.20, 0.26, 0.20),
    );

    commands.insert_resource(BridgePlan {
        surface,
        timer: Timer::from_seconds(1.8, TimerMode::Once),
        spawned: false,
    });

    let goal = pane_goal(&NavmeshExamplePane {
        goal_x: 2.0,
        goal_z: 0.0,
        rebuild_debounce_seconds: 0.0,
        ..default()
    });
    let goal_marker = spawn_goal_marker(
        &mut commands,
        &mut meshes,
        &mut materials,
        goal,
        Color::srgb(0.96, 0.84, 0.34),
    );
    let agent = spawn_agent(
        &mut commands,
        &mut meshes,
        &mut materials,
        "Procedural Agent",
        surface,
        Vec3::new(-2.0, 0.0, 0.0),
        goal,
        Color::srgb(0.50, 0.83, 0.94),
        saddle_ai_navmesh::NavmeshQueryFilter::default(),
    );
    commands.entity(goal_marker).insert(GoalMarker);
    commands
        .entity(agent)
        .insert((DemoAgent, NavmeshCrowdAvoidance::default()));
}

fn spawn_bridge_when_ready(
    mut commands: Commands,
    time: Res<Time>,
    mut plan: ResMut<BridgePlan>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    if plan.spawned {
        return;
    }

    plan.timer.tick(time.delta());
    if !plan.timer.just_finished() {
        return;
    }

    plan.spawned = true;
    spawn_nav_tile(
        &mut commands,
        &mut meshes,
        &mut materials,
        plan.surface,
        Vec3::ZERO,
        saddle_ai_navmesh::NavmeshArea(0),
        Color::srgb(0.56, 0.66, 0.22),
    );
}

fn sync_pane(
    pane: Res<NavmeshExamplePane>,
    mut goal_markers: Query<&mut Transform, With<GoalMarker>>,
    mut agents: Query<
        (
            &mut saddle_ai_navmesh::NavmeshAgent,
            &mut NavmeshFollowTarget,
            &mut NavmeshCrowdAvoidance,
        ),
        With<DemoAgent>,
    >,
) {
    if !pane.is_changed() {
        return;
    }

    let goal = pane_goal(&pane);
    for mut transform in &mut goal_markers {
        transform.translation = goal + Vec3::Y * 0.18;
    }
    for (mut agent, mut target, mut crowd) in &mut agents {
        apply_agent_tuning(&mut agent, &pane);
        apply_crowd_tuning(&mut crowd, &pane);
        *target = NavmeshFollowTarget::Point(goal);
    }
}
