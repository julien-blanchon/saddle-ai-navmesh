use std::time::Instant;

use bevy::prelude::*;
use saddle_ai_navmesh::{
    NavmeshArea, NavmeshAreaCost, NavmeshBuildInput, NavmeshPathId, NavmeshPathSmoothing,
    NavmeshPrimitive, NavmeshQueryFilter, NavmeshQuerySettings, NavmeshSourceGeometry,
    NavmeshSourceKind, bake_navmesh,
};
use saddle_pane::prelude::*;

const TILE_SIZE: f32 = 1.6;

#[derive(Component)]
struct PreviewTile;

#[derive(Resource, Debug, Clone, Copy, Pane)]
#[pane(title = "Navmesh Stress", position = "top-right")]
struct StressPane {
    #[pane(slider, min = 4.0, max = 14.0, step = 1.0)]
    grid_x: i32,
    #[pane(slider, min = 4.0, max = 14.0, step = 1.0)]
    grid_z: i32,
    #[pane(slider, min = 1.0, max = 24.0, step = 1.0)]
    query_count: i32,
    #[pane(slider, min = 1.0, max = 8.0, step = 0.1)]
    rough_multiplier: f32,
    #[pane(monitor)]
    polygons: u32,
    #[pane(monitor)]
    portals: u32,
    #[pane(monitor)]
    links: u32,
    #[pane(monitor)]
    bake_ms: f32,
    #[pane(monitor)]
    query_ms: f32,
    #[pane(monitor)]
    longest_path: f32,
}

impl Default for StressPane {
    fn default() -> Self {
        Self {
            grid_x: 8,
            grid_z: 8,
            query_count: 12,
            rough_multiplier: 2.5,
            polygons: 0,
            portals: 0,
            links: 0,
            bake_ms: 0.0,
            query_ms: 0.0,
            longest_path: 0.0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct StressInputs {
    grid_x: i32,
    grid_z: i32,
    query_count: usize,
    rough_multiplier: u32,
}

fn main() {
    App::new()
        .insert_resource(ClearColor(Color::srgb(0.07, 0.09, 0.11)))
        .insert_resource(StressPane::default())
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "navmesh stress".into(),
                resolution: (1380, 900).into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins((
            bevy_flair::FlairPlugin,
            bevy_input_focus::InputDispatchPlugin,
            bevy_ui_widgets::UiWidgetsPlugins,
            bevy_input_focus::tab_navigation::TabNavigationPlugin,
            PanePlugin,
        ))
        .register_pane::<StressPane>()
        .add_systems(Startup, setup_scene)
        .add_systems(Update, refresh_preview_and_metrics)
        .run();
}

fn setup_scene(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.spawn((
        Name::new("Stress Camera"),
        Camera3d::default(),
        Transform::from_xyz(-4.0, 18.0, 16.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
    commands.spawn((
        Name::new("Stress Light"),
        DirectionalLight {
            shadows_enabled: true,
            illuminance: 18_000.0,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -1.0, 0.7, 0.0)),
    ));
    commands.spawn((
        Name::new("Backdrop"),
        Mesh3d(meshes.add(Plane3d::default().mesh().size(40.0, 40.0))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.09, 0.11, 0.14),
            perceptual_roughness: 0.98,
            ..default()
        })),
    ));
    commands.spawn((
        Name::new("Example Label"),
        Node {
            position_type: PositionType::Absolute,
            left: px(18.0),
            top: px(18.0),
            width: px(480.0),
            padding: UiRect::all(px(14.0)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.02, 0.03, 0.05, 0.78)),
        Text::new(
            "stress: rebuild a synthetic surface and rerun path batches whenever the pane changes.\nUse it as a quick perf smoke test while keeping a readable 3D preview of the generated layout.",
        ),
        TextFont {
            font_size: 16.0,
            ..default()
        },
        TextColor(Color::WHITE),
    ));
}

fn refresh_preview_and_metrics(
    mut commands: Commands,
    mut pane: ResMut<StressPane>,
    mut previous: Local<Option<StressInputs>>,
    preview_tiles: Query<Entity, With<PreviewTile>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let inputs = StressInputs {
        grid_x: pane.grid_x.max(4),
        grid_z: pane.grid_z.max(4),
        query_count: pane.query_count.max(1) as usize,
        rough_multiplier: pane.rough_multiplier.to_bits(),
    };

    if previous.as_ref() == Some(&inputs) {
        return;
    }
    *previous = Some(inputs);

    for entity in &preview_tiles {
        commands.entity(entity).despawn();
    }

    let input = build_grid_input(inputs.grid_x, inputs.grid_z);
    let rough_multiplier = f32::from_bits(inputs.rough_multiplier);

    for source in &input.sources {
        let centroid = source
            .triangles
            .aabb()
            .map(|(min, max)| (min + max) * 0.5)
            .unwrap_or(Vec3::ZERO);
        let color = if source.area == NavmeshArea(1) {
            Color::srgb(0.46, 0.26, 0.18)
        } else {
            Color::srgb(0.18, 0.28, 0.24)
        };
        commands.spawn((
            Name::new("Preview Tile"),
            PreviewTile,
            Mesh3d(meshes.add(Cuboid::new(TILE_SIZE, 0.18, TILE_SIZE))),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: color,
                perceptual_roughness: 0.92,
                ..default()
            })),
            Transform::from_translation(centroid + Vec3::new(0.0, -0.09, 0.0)),
        ));
    }

    let bake_started = Instant::now();
    let surface = bake_navmesh(&input, &saddle_ai_navmesh::NavmeshBakeSettings::default())
        .expect("stress bake should succeed");
    let bake_ms = bake_started.elapsed().as_secs_f64() as f32 * 1000.0;

    let query_settings = NavmeshQuerySettings {
        smoothing: NavmeshPathSmoothing::Funnel,
        ..default()
    };
    let filter = NavmeshQueryFilter {
        area_costs: vec![NavmeshAreaCost::new(NavmeshArea(1), rough_multiplier)],
        ..default()
    };

    let query_started = Instant::now();
    let mut longest_path = 0.0_f32;
    for index in 0..inputs.query_count {
        let start = sample_point(index, -1, inputs.grid_x, inputs.grid_z);
        let goal = sample_point(index, 1, inputs.grid_x, inputs.grid_z);
        let result = surface.query_path(
            NavmeshPathId(index as u64 + 1),
            start,
            goal,
            &query_settings,
            &filter,
        );
        if let Some(path) = &result.path {
            longest_path = longest_path.max(path.total_length);
        }
    }
    let query_ms = query_started.elapsed().as_secs_f64() as f32 * 1000.0;

    pane.polygons = surface.polygons.len() as u32;
    pane.portals = surface.portals.len() as u32;
    pane.links = surface.links.len() as u32;
    pane.bake_ms = bake_ms.max(surface.stats.last_bake_ms);
    pane.query_ms = query_ms;
    pane.longest_path = longest_path;
}

fn build_grid_input(grid_x: i32, grid_z: i32) -> NavmeshBuildInput {
    let mut sources = Vec::new();
    let mut source_id = 1_u64;

    for x in 0..grid_x {
        for z in 0..grid_z {
            let translation = Vec3::new(
                (x as f32 - grid_x as f32 * 0.5) * TILE_SIZE,
                0.0,
                (z as f32 - grid_z as f32 * 0.5) * TILE_SIZE,
            );

            let area = if z > grid_z / 2 {
                NavmeshArea(1)
            } else {
                NavmeshArea(0)
            };
            sources.push(NavmeshSourceGeometry {
                source_id,
                kind: NavmeshSourceKind::Walkable,
                area,
                mask: saddle_ai_navmesh::NavmeshAreaMask::all(),
                triangles: NavmeshPrimitive::Quad {
                    size: Vec2::splat(TILE_SIZE),
                }
                .triangles()
                .transformed(Transform::from_translation(translation)),
            });
            source_id += 1;
        }
    }

    NavmeshBuildInput {
        sources,
        links: Vec::new(),
    }
}

fn sample_point(index: usize, side: i32, grid_x: i32, grid_z: i32) -> Vec3 {
    let lane = (index as i32 % grid_z) - grid_z / 2;
    let x = (side as f32) * (grid_x as f32 * 0.5 - 1.0) * TILE_SIZE;
    Vec3::new(x, 0.0, lane as f32 * TILE_SIZE * 0.6)
}
