use saddle_ai_navmesh_example_common as common;

use bevy::prelude::*;
use common::*;
use saddle_ai_navmesh::NavmeshBakeSettings;

#[derive(Resource)]
struct BridgePlan {
    surface: Entity,
    timer: Timer,
    spawned: bool,
}

fn main() {
    let mut app = App::new();
    configure_app(&mut app, "navmesh procedural rebake");
    app.add_systems(Startup, setup);
    app.add_systems(Update, spawn_bridge_when_ready);
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

    spawn_goal_marker(
        &mut commands,
        &mut meshes,
        &mut materials,
        Vec3::new(2.0, 0.0, 0.0),
        Color::srgb(0.96, 0.84, 0.34),
    );
    spawn_agent(
        &mut commands,
        &mut meshes,
        &mut materials,
        "Procedural Agent",
        surface,
        Vec3::new(-2.0, 0.0, 0.0),
        Vec3::new(2.0, 0.0, 0.0),
        Color::srgb(0.50, 0.83, 0.94),
        saddle_ai_navmesh::NavmeshQueryFilter::default(),
    );
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
