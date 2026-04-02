#[cfg(feature = "e2e")]
mod e2e;
#[cfg(feature = "e2e")]
mod scenarios;

#[cfg(all(feature = "dev", not(target_arch = "wasm32")))]
use bevy::remote::{RemotePlugin, http::RemoteHttpPlugin};
use bevy::{app::ScheduleRunnerPlugin, prelude::*};
#[cfg(all(feature = "dev", not(target_arch = "wasm32")))]
use bevy_brp_extras::BrpExtrasPlugin;
use saddle_ai_saddle_ai_navmesh::{
    NavmeshAgent, NavmeshArea, NavmeshAreaCost, NavmeshBakeSettings, NavmeshDebugSettings,
    NavmeshFollowTarget, NavmeshPathInvalidated, NavmeshPathResult, NavmeshPlugin,
    NavmeshPrimitive, NavmeshPrimitiveSource, NavmeshQueryFilter, NavmeshSource, NavmeshSourceKind,
    NavmeshSteeringOutput, NavmeshSurface, NavmeshSurfaceStatus,
};

const TILE_SIZE: f32 = 2.0;
const DEFAULT_LAB_BRP_PORT: u16 = 15_714;
const SMOKE_QUERY_START: Vec3 = Vec3::new(-6.0, 0.0, 0.0);
const SMOKE_QUERY_GOAL: Vec3 = Vec3::new(6.0, 0.0, 0.0);
const GATE_SIZE: Vec3 = Vec3::new(3.8, 1.6, 1.7);

#[derive(Component)]
struct SmokeAgent;

#[derive(Component)]
struct UtilityAgent;

#[derive(Component)]
struct WheeledAgent;

#[derive(Component)]
struct GateObstacle;

#[derive(Component)]
struct GateVisual;

#[derive(Resource, Debug, Clone, Copy, Default)]
pub struct LabControl {
    pub gate_blocked: bool,
}

#[derive(Resource, Debug, Clone, Default)]
pub struct LabDiagnostics {
    pub surface_ready: bool,
    pub smoke_path_cost: f32,
    pub smoke_baseline_cost: f32,
    pub smoke_detour_cost: f32,
    pub utility_cost: f32,
    pub wheeled_cost: f32,
    pub rebake_generation: u64,
    pub invalidations: u64,
    pub follow_distance: f32,
    pub follow_reached: bool,
}

fn main() {
    let mut app = App::new();
    let headless = lab_headless();

    app.insert_resource(ClearColor(Color::srgb(0.08, 0.10, 0.12)));
    app.insert_resource(LabControl::default());
    app.insert_resource(LabDiagnostics::default());

    if headless {
        app.add_plugins(MinimalPlugins.set(ScheduleRunnerPlugin::run_loop(
            std::time::Duration::from_secs_f64(1.0 / 60.0),
        )));
        app.init_resource::<Assets<Mesh>>();
        app.add_message::<bevy::asset::AssetEvent<Mesh>>();
        #[cfg(all(feature = "dev", not(target_arch = "wasm32")))]
        app.add_plugins((
            RemotePlugin::default(),
            RemoteHttpPlugin::default().with_port(lab_brp_port()),
        ));
    } else {
        app.add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "navmesh crate-local lab".into(),
                resolution: (1460, 920).into(),
                ..default()
            }),
            ..default()
        }));
        #[cfg(all(feature = "dev", not(target_arch = "wasm32")))]
        app.add_plugins((
            RemotePlugin::default(),
            BrpExtrasPlugin::with_http_plugin(
                RemoteHttpPlugin::default().with_port(lab_brp_port()),
            ),
        ));
        #[cfg(feature = "e2e")]
        app.add_plugins(e2e::NavmeshLabE2EPlugin);
    }

    app.add_plugins(NavmeshPlugin::default());
    app.insert_resource(NavmeshDebugSettings {
        enabled: !headless,
        draw_surface: true,
        draw_portals: true,
        draw_links: true,
        draw_paths: true,
        draw_projections: true,
        draw_agents: true,
        ..default()
    });
    app.add_systems(Startup, setup);
    app.add_systems(Update, apply_steering);
    app.add_systems(
        Update,
        sync_gate_state.before(saddle_ai_navmesh::NavmeshSystems::DetectChanges),
    );
    app.add_systems(
        Update,
        update_diagnostics.after(saddle_ai_navmesh::NavmeshSystems::Follow),
    );
    app.add_systems(
        Update,
        track_invalidations.after(saddle_ai_navmesh::NavmeshSystems::Query),
    );
    app.run();
}

#[cfg(all(feature = "dev", not(target_arch = "wasm32")))]
fn lab_brp_port() -> u16 {
    std::env::var("NAVMESH_LAB_BRP_PORT")
        .ok()
        .and_then(|value| value.parse::<u16>().ok())
        .unwrap_or(DEFAULT_LAB_BRP_PORT)
}

#[cfg(any(not(feature = "dev"), target_arch = "wasm32"))]
fn lab_brp_port() -> u16 {
    DEFAULT_LAB_BRP_PORT
}

fn lab_headless() -> bool {
    std::env::var("NAVMESH_LAB_HEADLESS")
        .is_ok_and(|value| value == "1" || value.eq_ignore_ascii_case("true"))
}

fn setup(
    mut commands: Commands,
    meshes: Option<ResMut<Assets<Mesh>>>,
    materials: Option<ResMut<Assets<StandardMaterial>>>,
) {
    let mut meshes = meshes;
    let mut materials = materials;

    if let (Some(meshes), Some(materials)) = (&mut meshes, &mut materials) {
        spawn_camera_and_light(&mut commands);
        spawn_goal_marker(
            &mut commands,
            meshes,
            materials,
            Vec3::new(6.0, 0.0, 0.0),
            Color::srgb(0.97, 0.87, 0.37),
        );
    }

    let surface = commands
        .spawn((
            Name::new("Lab Surface"),
            NavmeshSurface::default(),
            NavmeshBakeSettings {
                agent_radius: 0.0,
                async_baking: false,
                rebuild_debounce_seconds: 0.0,
                ..default()
            },
        ))
        .id();

    for x in -3..=3 {
        for row in -1..=1 {
            let area = if row == 0 {
                NavmeshArea(1)
            } else {
                NavmeshArea(0)
            };
            let color = if row == 0 {
                Color::srgb(0.46, 0.23, 0.16)
            } else {
                Color::srgb(0.16, 0.28, 0.23)
            };
            if let (Some(meshes), Some(materials)) = (&mut meshes, &mut materials) {
                spawn_nav_tile(
                    &mut commands,
                    meshes,
                    materials,
                    surface,
                    Vec3::new(x as f32 * TILE_SIZE, 0.0, row as f32 * TILE_SIZE),
                    area,
                    color,
                );
            } else {
                spawn_nav_tile_headless(
                    &mut commands,
                    surface,
                    Vec3::new(x as f32 * TILE_SIZE, 0.0, row as f32 * TILE_SIZE),
                    area,
                );
            }
        }
    }

    let gate_translation = Vec3::ZERO;
    commands.spawn((
        Name::new("Gate Obstacle"),
        GateObstacle,
        NavmeshSource::new(surface, NavmeshSourceKind::Obstacle),
        NavmeshPrimitiveSource::new(NavmeshPrimitive::Cuboid { size: GATE_SIZE }),
        Transform::from_translation(gate_translation + Vec3::new(0.0, 0.8, 0.0)),
        GlobalTransform::default(),
    ));
    if let (Some(meshes), Some(materials)) = (&mut meshes, &mut materials) {
        commands.spawn((
            Name::new("Gate Visual"),
            GateVisual,
            Mesh3d(meshes.add(Cuboid::new(GATE_SIZE.x, GATE_SIZE.y, GATE_SIZE.z))),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: Color::srgba(0.95, 0.32, 0.22, 0.08),
                emissive: Color::srgb(0.12, 0.02, 0.01).into(),
                ..default()
            })),
            Transform::from_translation(gate_translation + Vec3::new(0.0, 0.8, 0.0)),
            GlobalTransform::default(),
        ));
    }

    let wheeled_filter = NavmeshQueryFilter {
        area_costs: vec![NavmeshAreaCost::new(NavmeshArea(1), 10.0)],
        ..default()
    };

    if let (Some(meshes), Some(materials)) = (&mut meshes, &mut materials) {
        spawn_agent(
            &mut commands,
            meshes,
            materials,
            "Smoke Agent",
            SmokeAgent,
            surface,
            Vec3::new(-6.0, 0.0, 0.0),
            Vec3::new(6.0, 0.0, 0.0),
            Color::srgb(0.36, 0.86, 0.66),
            NavmeshQueryFilter::default(),
        );
        spawn_agent(
            &mut commands,
            meshes,
            materials,
            "Utility Agent",
            UtilityAgent,
            surface,
            Vec3::new(-6.0, 0.0, -0.4),
            Vec3::new(6.0, 0.0, -0.4),
            Color::srgb(0.33, 0.92, 0.61),
            NavmeshQueryFilter::default(),
        );
        spawn_agent(
            &mut commands,
            meshes,
            materials,
            "Wheeled Agent",
            WheeledAgent,
            surface,
            Vec3::new(-6.0, 0.0, 0.4),
            Vec3::new(6.0, 0.0, 0.4),
            Color::srgb(0.33, 0.64, 0.96),
            wheeled_filter,
        );
    } else {
        spawn_agent_headless(
            &mut commands,
            "Smoke Agent",
            SmokeAgent,
            surface,
            Vec3::new(-6.0, 0.0, 0.0),
            Vec3::new(6.0, 0.0, 0.0),
            NavmeshQueryFilter::default(),
        );
        spawn_agent_headless(
            &mut commands,
            "Utility Agent",
            UtilityAgent,
            surface,
            Vec3::new(-6.0, 0.0, -0.4),
            Vec3::new(6.0, 0.0, -0.4),
            NavmeshQueryFilter::default(),
        );
        spawn_agent_headless(
            &mut commands,
            "Wheeled Agent",
            WheeledAgent,
            surface,
            Vec3::new(-6.0, 0.0, 0.4),
            Vec3::new(6.0, 0.0, 0.4),
            wheeled_filter,
        );
    }
}

fn spawn_camera_and_light(commands: &mut Commands) {
    commands.spawn((
        Name::new("Lab Camera"),
        Camera3d::default(),
        Transform::from_xyz(0.0, 19.0, 15.0).looking_at(Vec3::ZERO, Vec3::Y),
        GlobalTransform::default(),
    ));
    commands.spawn((
        Name::new("Lab Light"),
        DirectionalLight {
            shadows_enabled: true,
            illuminance: 15_000.0,
            ..default()
        },
        Transform::from_xyz(8.0, 14.0, 8.0).looking_at(Vec3::ZERO, Vec3::Y),
        GlobalTransform::default(),
    ));
}

fn spawn_nav_tile(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    surface: Entity,
    translation: Vec3,
    area: NavmeshArea,
    color: Color,
) {
    commands.spawn((
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
    ));
}

fn spawn_nav_tile_headless(
    commands: &mut Commands,
    surface: Entity,
    translation: Vec3,
    area: NavmeshArea,
) {
    commands.spawn((
        Name::new("Nav Tile"),
        NavmeshSource::new(surface, NavmeshSourceKind::Walkable).with_area(area),
        NavmeshPrimitiveSource::new(NavmeshPrimitive::Quad {
            size: Vec2::splat(TILE_SIZE),
        }),
        Transform::from_translation(translation),
        GlobalTransform::default(),
    ));
}

fn spawn_goal_marker(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    translation: Vec3,
    color: Color,
) {
    commands.spawn((
        Name::new("Goal Marker"),
        Mesh3d(meshes.add(Cuboid::new(0.36, 0.36, 0.36))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: color,
            emissive: color.into(),
            ..default()
        })),
        Transform::from_translation(translation + Vec3::Y * 0.18),
        GlobalTransform::default(),
    ));
}

fn spawn_agent<T: Component>(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    name: &str,
    marker: T,
    surface: Entity,
    start: Vec3,
    target: Vec3,
    color: Color,
    filter: NavmeshQueryFilter,
) {
    let mut agent = NavmeshAgent::new(surface)
        .with_max_speed(3.5)
        .with_filter(filter);
    agent.arrival_distance = 0.35;
    agent.waypoint_distance = 0.35;
    agent.overshoot_distance = 0.2;

    commands.spawn((
        Name::new(name.to_string()),
        marker,
        agent,
        NavmeshFollowTarget::Point(target),
        Mesh3d(meshes.add(Cuboid::new(0.45, 0.45, 0.45))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: color,
            metallic: 0.04,
            perceptual_roughness: 0.35,
            ..default()
        })),
        Transform::from_translation(start + Vec3::Y * 0.25),
        GlobalTransform::default(),
    ));
}

fn spawn_agent_headless<T: Component>(
    commands: &mut Commands,
    name: &str,
    marker: T,
    surface: Entity,
    start: Vec3,
    target: Vec3,
    filter: NavmeshQueryFilter,
) {
    let mut agent = NavmeshAgent::new(surface)
        .with_max_speed(3.5)
        .with_filter(filter);
    agent.arrival_distance = 0.35;
    agent.waypoint_distance = 0.35;
    agent.overshoot_distance = 0.2;

    commands.spawn((
        Name::new(name.to_string()),
        marker,
        agent,
        NavmeshFollowTarget::Point(target),
        Transform::from_translation(start + Vec3::Y * 0.25),
        GlobalTransform::default(),
    ));
}

fn sync_gate_state(
    control: Res<LabControl>,
    mut gate_sources: Query<&mut NavmeshSource, With<GateObstacle>>,
    mut gate_visuals: Query<&mut Visibility, With<GateVisual>>,
) {
    if !control.is_changed() {
        return;
    }

    for mut source in &mut gate_sources {
        source.enabled = control.gate_blocked;
    }
    for mut visibility in &mut gate_visuals {
        *visibility = if control.gate_blocked {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
}

fn apply_steering(time: Res<Time>, mut movers: Query<(&mut Transform, &NavmeshSteeringOutput)>) {
    for (mut transform, output) in &mut movers {
        if output.reached_goal || output.desired_velocity.length_squared() <= f32::EPSILON {
            continue;
        }
        let planar_velocity = Vec3::new(output.desired_velocity.x, 0.0, output.desired_velocity.z);
        transform.translation += planar_velocity * time.delta_secs();
        transform.translation.y = 0.25;
    }
}

fn update_diagnostics(
    control: Res<LabControl>,
    mut diagnostics: ResMut<LabDiagnostics>,
    surfaces: Query<(&NavmeshSurfaceStatus, Option<&saddle_ai_navmesh::NavmeshSurfaceData>)>,
    smoke: Query<&NavmeshSteeringOutput, With<SmokeAgent>>,
    utility: Query<&NavmeshPathResult, With<UtilityAgent>>,
    wheeled: Query<&NavmeshPathResult, With<WheeledAgent>>,
) {
    if let Ok((status, surface_data)) = surfaces.single() {
        diagnostics.surface_ready = matches!(status.state, saddle_ai_navmesh::NavmeshBakeState::Ready);
        diagnostics.rebake_generation = status.generation;

        if let Some(surface_data) = surface_data {
            let fixed_query = surface_data.query_path(
                saddle_ai_navmesh::NavmeshPathId(0),
                SMOKE_QUERY_START,
                SMOKE_QUERY_GOAL,
                &saddle_ai_navmesh::NavmeshQuerySettings::default(),
                &NavmeshQueryFilter::default(),
            );

            if let Some(path) = fixed_query.path {
                diagnostics.smoke_path_cost = path.total_cost;
                if control.gate_blocked {
                    diagnostics.smoke_detour_cost = path.total_cost;
                } else {
                    diagnostics.smoke_baseline_cost = path.total_cost;
                }
            }
        }
    }

    if let Ok(steering) = smoke.single() {
        diagnostics.follow_distance = steering.remaining_distance;
        diagnostics.follow_reached = steering.reached_goal;
    }

    if let Ok(path_result) = utility.single() {
        if let Some(path) = &path_result.result.path {
            diagnostics.utility_cost = path.total_cost;
        }
    }
    if let Ok(path_result) = wheeled.single() {
        if let Some(path) = &path_result.result.path {
            diagnostics.wheeled_cost = path.total_cost;
        }
    }
}

fn track_invalidations(
    mut diagnostics: ResMut<LabDiagnostics>,
    mut invalidated: MessageReader<NavmeshPathInvalidated>,
) {
    diagnostics.invalidations += invalidated.read().count() as u64;
}

pub fn set_gate_blocked(world: &mut World, blocked: bool) {
    world.resource_mut::<LabControl>().gate_blocked = blocked;
}
