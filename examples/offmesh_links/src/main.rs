use saddle_ai_navmesh_example_common as common;

use bevy::prelude::*;
use common::*;
use saddle_ai_navmesh::NavmeshBakeSettings;

fn main() {
    let mut app = App::new();
    configure_app(&mut app, "navmesh offmesh links");
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

    for x in [-2.0_f32, 2.0] {
        spawn_nav_tile(
            &mut commands,
            &mut meshes,
            &mut materials,
            surface,
            Vec3::new(x, 0.0, 0.0),
            saddle_ai_navmesh::NavmeshArea(0),
            Color::srgb(0.18, 0.24, 0.31),
        );
    }

    commands.spawn((
        Name::new("Gap Plane"),
        Mesh3d(meshes.add(Plane3d::default().mesh().size(2.0, 3.0))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.06, 0.18, 0.28),
            emissive: Color::srgb(0.02, 0.08, 0.12).into(),
            ..default()
        })),
        Transform::from_xyz(0.0, -0.02, 0.0),
        GlobalTransform::default(),
    ));

    spawn_link_marker(
        &mut commands,
        surface,
        Vec3::new(-1.0, 0.0, 0.0),
        Vec3::new(1.0, 0.0, 0.0),
    );
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
        "Link Agent",
        surface,
        Vec3::new(-2.0, 0.0, 0.0),
        Vec3::new(2.0, 0.0, 0.0),
        Color::srgb(0.92, 0.61, 0.29),
        saddle_ai_navmesh::NavmeshQueryFilter::default(),
    );
}
