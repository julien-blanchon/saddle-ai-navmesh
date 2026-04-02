use saddle_ai_navmesh_example_common as common;

use bevy::prelude::*;
use common::*;
use saddle_ai_navmesh::{NavmeshArea, NavmeshBakeSettings, NavmeshQueryFilter};

fn main() {
    let mut app = App::new();
    configure_app(&mut app, "navmesh multi agent classes");
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
    let goal = Vec3::new(4.0, 0.0, 0.0);

    spawn_goal_marker(
        &mut commands,
        &mut meshes,
        &mut materials,
        goal,
        Color::srgb(0.98, 0.86, 0.39),
    );
    spawn_agent(
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
    spawn_agent(
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
}
