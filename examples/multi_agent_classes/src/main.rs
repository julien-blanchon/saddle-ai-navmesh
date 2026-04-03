use saddle_ai_navmesh_example_common as common;

use bevy::prelude::*;
use common::*;
use saddle_ai_navmesh::{
    NavmeshAgent, NavmeshArea, NavmeshBakeSettings, NavmeshCrowdAvoidance, NavmeshFollowTarget,
    NavmeshQueryFilter,
};

#[derive(Component)]
struct GoalMarker;

#[derive(Component)]
struct UtilityAgentMarker;

#[derive(Component)]
struct WheeledAgentMarker;

fn main() {
    let mut app = App::new();
    app.insert_resource(NavmeshExamplePane {
        goal_x: 4.0,
        goal_z: 0.0,
        rough_area_multiplier: 10.0,
        ..default()
    });
    configure_app(&mut app, "navmesh multi agent classes");
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
        for row in -1..=1 {
            let area = if row == 0 {
                NavmeshArea(1)
            } else {
                NavmeshArea(0)
            };
            let color = if row == 0 {
                Color::srgb(0.52, 0.25, 0.17)
            } else {
                Color::srgb(0.16, 0.29, 0.23)
            };
            spawn_nav_tile(
                &mut commands,
                &mut meshes,
                &mut materials,
                surface,
                Vec3::new(x as f32 * TILE_SIZE, 0.0, row as f32 * TILE_SIZE),
                area,
                color,
            );
        }
    }

    let start = Vec3::new(-4.0, 0.0, 0.0);

    let goal = pane_goal(&NavmeshExamplePane {
        goal_x: 4.0,
        goal_z: 0.0,
        rough_area_multiplier: 10.0,
        ..default()
    });
    let goal_marker = spawn_goal_marker(
        &mut commands,
        &mut meshes,
        &mut materials,
        goal,
        Color::srgb(0.98, 0.86, 0.39),
    );
    let utility = spawn_agent(
        &mut commands,
        &mut meshes,
        &mut materials,
        "Utility Agent",
        surface,
        start + Vec3::new(0.0, 0.0, -0.2),
        goal,
        Color::srgb(0.32, 0.90, 0.61),
        NavmeshQueryFilter::default(),
    );
    let wheeled = spawn_agent(
        &mut commands,
        &mut meshes,
        &mut materials,
        "Wheeled Agent",
        surface,
        start + Vec3::new(0.0, 0.0, 0.2),
        goal,
        Color::srgb(0.33, 0.63, 0.96),
        rough_filter(10.0),
    );
    commands.entity(goal_marker).insert(GoalMarker);
    commands
        .entity(utility)
        .insert((UtilityAgentMarker, NavmeshCrowdAvoidance::default()));
    commands
        .entity(wheeled)
        .insert((WheeledAgentMarker, NavmeshCrowdAvoidance::default()));
}

fn sync_pane(
    pane: Res<NavmeshExamplePane>,
    mut goal_markers: Query<&mut Transform, With<GoalMarker>>,
    mut utility_agents: Query<
        (
            &mut NavmeshAgent,
            &mut NavmeshFollowTarget,
            &mut NavmeshCrowdAvoidance,
        ),
        (With<UtilityAgentMarker>, Without<WheeledAgentMarker>),
    >,
    mut wheeled_agents: Query<
        (
            &mut NavmeshAgent,
            &mut NavmeshFollowTarget,
            &mut NavmeshCrowdAvoidance,
        ),
        (With<WheeledAgentMarker>, Without<UtilityAgentMarker>),
    >,
) {
    if !pane.is_changed() {
        return;
    }

    let goal = pane_goal(&pane);
    for mut transform in &mut goal_markers {
        transform.translation = goal + Vec3::Y * 0.18;
    }
    for (mut agent, mut target, mut crowd) in &mut utility_agents {
        apply_agent_tuning(&mut agent, &pane);
        apply_crowd_tuning(&mut crowd, &pane);
        agent.filter = NavmeshQueryFilter::default();
        *target = NavmeshFollowTarget::Point(goal);
    }
    for (mut agent, mut target, mut crowd) in &mut wheeled_agents {
        apply_agent_tuning(&mut agent, &pane);
        apply_crowd_tuning(&mut crowd, &pane);
        agent.filter = rough_filter(pane.rough_area_multiplier);
        *target = NavmeshFollowTarget::Point(goal);
    }
}
