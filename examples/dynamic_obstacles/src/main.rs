use saddle_ai_navmesh_example_common as common;

use bevy::prelude::*;
use common::*;
use saddle_ai_navmesh::{NavmeshBakeSettings, NavmeshCrowdAvoidance, NavmeshFollowTarget};

#[derive(Component)]
struct OscillatingObstacle {
    from: Vec3,
    to: Vec3,
    speed: f32,
}

#[derive(Component)]
struct GoalMarker;

#[derive(Component)]
struct DemoAgent;

fn main() {
    let mut app = App::new();
    app.insert_resource(NavmeshExamplePane {
        goal_x: 6.0,
        goal_z: 0.0,
        crowd_enabled: true,
        ..default()
    });
    configure_app(&mut app, "navmesh dynamic obstacles");
    app.add_systems(Startup, setup);
    app.add_systems(Update, (animate_obstacles, sync_pane));
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
            rebuild_debounce_seconds: 0.05,
            ..default()
        },
    );

    for x in -3..=3 {
        for z in -1..=1 {
            spawn_nav_tile(
                &mut commands,
                &mut meshes,
                &mut materials,
                surface,
                Vec3::new(x as f32 * TILE_SIZE, 0.0, z as f32 * TILE_SIZE),
                saddle_ai_navmesh::NavmeshArea(0),
                Color::srgb(0.17, 0.22, 0.27),
            );
        }
    }

    let mover = spawn_obstacle_box(
        &mut commands,
        &mut meshes,
        &mut materials,
        surface,
        Vec3::new(0.0, 0.7, -TILE_SIZE),
        Vec3::new(1.5, 1.4, 1.5),
        Color::srgb(0.88, 0.46, 0.22),
    );
    commands.entity(mover).insert(OscillatingObstacle {
        from: Vec3::new(0.0, 0.7, -TILE_SIZE),
        to: Vec3::new(0.0, 0.7, TILE_SIZE),
        speed: 0.65,
    });

    let goal = pane_goal(&NavmeshExamplePane {
        goal_x: 6.0,
        goal_z: 0.0,
        ..default()
    });
    let goal_marker = spawn_goal_marker(
        &mut commands,
        &mut meshes,
        &mut materials,
        goal,
        Color::srgb(0.97, 0.89, 0.41),
    );
    let agent = spawn_agent(
        &mut commands,
        &mut meshes,
        &mut materials,
        "Detour Agent",
        surface,
        Vec3::new(-6.0, 0.0, 0.0),
        goal,
        Color::srgb(0.36, 0.83, 0.90),
        saddle_ai_navmesh::NavmeshQueryFilter::default(),
    );
    commands.entity(goal_marker).insert(GoalMarker);
    commands
        .entity(agent)
        .insert((DemoAgent, NavmeshCrowdAvoidance::default()));
}

fn animate_obstacles(
    time: Res<Time>,
    mut obstacles: Query<(&mut Transform, &OscillatingObstacle)>,
) {
    for (mut transform, motion) in &mut obstacles {
        let phase = (time.elapsed_secs() * motion.speed).sin() * 0.5 + 0.5;
        transform.translation = motion.from.lerp(motion.to, phase);
    }
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
