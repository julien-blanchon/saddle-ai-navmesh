use bevy::app::AppExit;
use bevy::prelude::*;
use saddle_ai_navmesh::{
    NavmeshAgent, NavmeshArea, NavmeshAreaCost, NavmeshBakeSettings, NavmeshDebugSettings,
    NavmeshFollowTarget, NavmeshLinkSource, NavmeshOffMeshLink, NavmeshPlugin, NavmeshPrimitive,
    NavmeshPrimitiveSource, NavmeshQueryFilter, NavmeshSource, NavmeshSourceKind, NavmeshSurface,
};

pub const TILE_SIZE: f32 = 2.0;

#[derive(Resource)]
struct ExampleAutoExit(Timer);

pub fn configure_app(app: &mut App, title: &str) {
    app.add_plugins(DefaultPlugins.set(WindowPlugin {
        primary_window: Some(Window {
            title: title.into(),
            resolution: (1440, 900).into(),
            ..default()
        }),
        ..default()
    }));
    app.add_plugins(NavmeshPlugin::default());
    app.insert_resource(ClearColor(Color::srgb(0.08, 0.10, 0.12)));
    app.insert_resource(NavmeshDebugSettings {
        enabled: true,
        draw_surface: true,
        draw_portals: true,
        draw_links: true,
        draw_paths: true,
        draw_projections: true,
        draw_agents: true,
        ..default()
    });
    app.add_systems(Update, apply_navmesh_steering);

    #[cfg(not(target_arch = "wasm32"))]
    if let Some(seconds) = std::env::var("NAVMESH_EXAMPLE_EXIT_AFTER_SECONDS")
        .ok()
        .and_then(|value| value.parse::<f32>().ok())
    {
        app.insert_resource(ExampleAutoExit(Timer::from_seconds(
            seconds.max(0.0),
            TimerMode::Once,
        )));
        app.add_systems(Update, auto_exit_example);
    }
}

pub fn spawn_scene_camera(commands: &mut Commands) {
    commands.spawn((
        Name::new("Scene Camera"),
        Camera3d::default(),
        Transform::from_xyz(0.0, 18.0, 14.0).looking_at(Vec3::new(0.0, 0.0, 0.0), Vec3::Y),
        GlobalTransform::default(),
    ));
    commands.spawn((
        Name::new("Sun Light"),
        DirectionalLight {
            shadows_enabled: true,
            illuminance: 14_000.0,
            ..default()
        },
        Transform::from_xyz(6.0, 14.0, 8.0).looking_at(Vec3::ZERO, Vec3::Y),
        GlobalTransform::default(),
    ));
}

pub fn spawn_surface(commands: &mut Commands, settings: NavmeshBakeSettings) -> Entity {
    commands
        .spawn((
            Name::new("Navmesh Surface"),
            NavmeshSurface::default(),
            settings,
        ))
        .id()
}

pub fn spawn_nav_tile(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    surface: Entity,
    translation: Vec3,
    area: NavmeshArea,
    color: Color,
) -> Entity {
    commands
        .spawn((
            Name::new("Nav Tile"),
            NavmeshSource::new(surface, NavmeshSourceKind::Walkable).with_area(area),
            NavmeshPrimitiveSource::new(NavmeshPrimitive::Quad {
                size: Vec2::splat(TILE_SIZE),
            }),
            Mesh3d(meshes.add(Plane3d::default().mesh().size(TILE_SIZE, TILE_SIZE))),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: color,
                perceptual_roughness: 1.0,
                ..default()
            })),
            Transform::from_translation(translation),
            GlobalTransform::default(),
        ))
        .id()
}

pub fn spawn_obstacle_box(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    surface: Entity,
    translation: Vec3,
    size: Vec3,
    color: Color,
) -> Entity {
    commands
        .spawn((
            Name::new("Obstacle Box"),
            NavmeshSource::new(surface, NavmeshSourceKind::Obstacle),
            NavmeshPrimitiveSource::new(NavmeshPrimitive::Cuboid { size }),
            Mesh3d(meshes.add(Cuboid::new(size.x, size.y, size.z))),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: color,
                perceptual_roughness: 0.9,
                ..default()
            })),
            Transform::from_translation(translation),
            GlobalTransform::default(),
        ))
        .id()
}

pub fn spawn_agent(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    name: &str,
    surface: Entity,
    start: Vec3,
    target: Vec3,
    color: Color,
    filter: NavmeshQueryFilter,
) -> Entity {
    commands
        .spawn((
            Name::new(name.to_string()),
            NavmeshAgent::new(surface)
                .with_max_speed(3.8)
                .with_filter(filter),
            NavmeshFollowTarget::Point(target),
            Mesh3d(meshes.add(Cuboid::new(0.5, 0.5, 0.5))),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: color,
                metallic: 0.05,
                perceptual_roughness: 0.35,
                ..default()
            })),
            Transform::from_translation(start + Vec3::Y * 0.25),
            GlobalTransform::default(),
        ))
        .id()
}

pub fn spawn_goal_marker(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    translation: Vec3,
    color: Color,
) {
    commands.spawn((
        Name::new("Goal Marker"),
        Mesh3d(meshes.add(Cuboid::new(0.35, 0.35, 0.35))),
        MeshMaterial3d(materials.add(StandardMaterial {
            emissive: color.into(),
            base_color: color,
            ..default()
        })),
        Transform::from_translation(translation + Vec3::Y * 0.18),
        GlobalTransform::default(),
    ));
}

pub fn spawn_link_marker(
    commands: &mut Commands,
    surface: Entity,
    start: Vec3,
    end: Vec3,
) -> Entity {
    commands
        .spawn((
            Name::new("Off-Mesh Link"),
            NavmeshLinkSource::new(
                surface,
                NavmeshOffMeshLink {
                    start,
                    end,
                    bidirectional: true,
                    ..default()
                },
            ),
        ))
        .id()
}

pub fn rough_filter(multiplier: f32) -> NavmeshQueryFilter {
    NavmeshQueryFilter {
        area_costs: vec![NavmeshAreaCost::new(NavmeshArea(1), multiplier)],
        ..default()
    }
}

pub fn apply_navmesh_steering(
    time: Res<Time>,
    mut movers: Query<(&mut Transform, &saddle_ai_navmesh::NavmeshSteeringOutput)>,
) {
    for (mut transform, output) in &mut movers {
        if output.reached_goal || output.desired_velocity.length_squared() <= f32::EPSILON {
            continue;
        }

        transform.translation += output.desired_velocity * time.delta_secs();
        transform.translation.y = 0.25;
    }
}

fn auto_exit_example(
    time: Res<Time>,
    auto_exit: Option<ResMut<ExampleAutoExit>>,
    mut app_exit: MessageWriter<AppExit>,
) {
    let Some(mut auto_exit) = auto_exit else {
        return;
    };

    if auto_exit.0.tick(time.delta()).just_finished() {
        app_exit.write(AppExit::Success);
    }
}
