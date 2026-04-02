use saddle_ai_navmesh_example_common as common;

use bevy::prelude::*;
use common::*;
use saddle_ai_navmesh::NavmeshBakeSettings;

#[derive(Component)]
struct OscillatingObstacle {
    from: Vec3,
    to: Vec3,
    speed: f32,
}

fn main() {
    let mut app = App::new();
    configure_app(&mut app, "navmesh dynamic obstacles");
    app.add_systems(Startup, setup);
    app.add_systems(Update, animate_obstacles);
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

    spawn_goal_marker(
        &mut commands,
        &mut meshes,
        &mut materials,
        Vec3::new(6.0, 0.0, 0.0),
        Color::srgb(0.97, 0.89, 0.41),
    );
    spawn_agent(
        &mut commands,
        &mut meshes,
        &mut materials,
        "Detour Agent",
        surface,
        Vec3::new(-6.0, 0.0, 0.0),
        Vec3::new(6.0, 0.0, 0.0),
        Color::srgb(0.36, 0.83, 0.90),
        saddle_ai_navmesh::NavmeshQueryFilter::default(),
    );
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
