use saddle_ai_navmesh_example_common as common;

use bevy::prelude::*;
use common::*;
use saddle_ai_navmesh::{NavmeshBakeSettings, NavmeshCrowdAvoidance, NavmeshFollowTarget};

#[derive(Component)]
struct GoalMarker;

#[derive(Component)]
struct DemoAgent;

fn main() {
    let mut app = App::new();
    app.insert_resource(NavmeshExamplePane {
        goal_x: 4.0,
        goal_z: 4.0,
        ..default()
    });
    configure_app(&mut app, "navmesh basic");
    app.add_systems(Startup, setup);
    app.add_systems(Update, sync_pane);
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
            ..default()
        },
    );

    for x in -2..=2 {
        for z in -2..=2 {
            spawn_nav_tile(
                &mut commands,
                &mut meshes,
                &mut materials,
                surface,
                Vec3::new(x as f32 * TILE_SIZE, 0.0, z as f32 * TILE_SIZE),
                saddle_ai_navmesh::NavmeshArea(0),
                Color::srgb(0.18, 0.27, 0.22),
            );
        }
    }

    spawn_obstacle_box(
        &mut commands,
        &mut meshes,
        &mut materials,
        surface,
        Vec3::new(0.0, 0.8, 0.0),
        Vec3::new(2.2, 1.6, 2.2),
        Color::srgb(0.53, 0.23, 0.19),
    );
    let goal = pane_goal(&NavmeshExamplePane::default());
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
        "Basic Agent",
        surface,
        Vec3::new(-4.0, 0.0, -4.0),
        goal,
        Color::srgb(0.40, 0.86, 0.66),
        saddle_ai_navmesh::NavmeshQueryFilter::default(),
    );
    commands
        .entity(agent)
        .insert((DemoAgent, NavmeshCrowdAvoidance::default()));
    commands.entity(goal_marker).insert(GoalMarker);
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
