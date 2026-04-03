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
        goal_x: 2.0,
        goal_z: 0.0,
        crowd_enabled: false,
        ..default()
    });
    configure_app(&mut app, "navmesh offmesh links");
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
    let goal = pane_goal(&NavmeshExamplePane {
        goal_x: 2.0,
        goal_z: 0.0,
        crowd_enabled: false,
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
        "Link Agent",
        surface,
        Vec3::new(-2.0, 0.0, 0.0),
        goal,
        Color::srgb(0.92, 0.61, 0.29),
        saddle_ai_navmesh::NavmeshQueryFilter::default(),
    );
    commands.entity(goal_marker).insert(GoalMarker);
    commands
        .entity(agent)
        .insert((DemoAgent, NavmeshCrowdAvoidance::default()));
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
