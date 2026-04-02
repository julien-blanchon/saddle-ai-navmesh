use saddle_ai_navmesh_example_common as common;

use bevy::prelude::*;
use common::*;
use saddle_ai_saddle_ai_navmesh::NavmeshBakeSettings;

fn main() {
    let mut app = App::new();
    configure_app(&mut app, "navmesh basic");
    app.add_systems(Startup, setup);
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
    spawn_goal_marker(
        &mut commands,
        &mut meshes,
        &mut materials,
        Vec3::new(4.0, 0.0, 4.0),
        Color::srgb(0.96, 0.84, 0.34),
    );
    spawn_agent(
        &mut commands,
        &mut meshes,
        &mut materials,
        "Basic Agent",
        surface,
        Vec3::new(-4.0, 0.0, -4.0),
        Vec3::new(4.0, 0.0, 4.0),
        Color::srgb(0.40, 0.86, 0.66),
        saddle_ai_navmesh::NavmeshQueryFilter::default(),
    );
}
