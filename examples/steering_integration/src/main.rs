use bevy::prelude::*;
use saddle_ai_navmesh::{
    NavmeshAgent, NavmeshBakeSettings, NavmeshCrowdAvoidance, NavmeshDebugSettings,
    NavmeshFollowTarget, NavmeshPathResult, NavmeshPlugin, NavmeshQueryFilter, NavmeshSystems,
};
use saddle_ai_navmesh_example_common as common;
use saddle_ai_steering as steering;
use saddle_pane::prelude::*;
use steering::{
    Flocking, PathFollowing, ReciprocalAvoidance, SteeringAgent, SteeringAutoApply,
    SteeringObstacle, SteeringPlane, SteeringPlugin, SteeringSystems,
};

#[derive(Component)]
struct CrowdAgent;

#[derive(Component)]
struct GoalMarker;

#[derive(Resource, Debug, Clone, Copy, Pane)]
#[pane(title = "Steering Blend", position = "top-left")]
struct IntegrationPane {
    #[pane(slider, min = 2.0, max = 18.0, step = 0.2)]
    max_acceleration: f32,
    #[pane(slider, min = 0.4, max = 4.5, step = 0.1)]
    path_lookahead: f32,
    #[pane(slider, min = 0.2, max = 3.0, step = 0.05)]
    separation_weight: f32,
    #[pane(slider, min = 0.2, max = 2.0, step = 0.05)]
    alignment_weight: f32,
    #[pane(slider, min = 0.2, max = 2.0, step = 0.05)]
    cohesion_weight: f32,
}

impl Default for IntegrationPane {
    fn default() -> Self {
        Self {
            max_acceleration: 10.0,
            path_lookahead: 2.1,
            separation_weight: 1.6,
            alignment_weight: 0.9,
            cohesion_weight: 0.8,
        }
    }
}

fn main() {
    let mut app = App::new();
    app.insert_resource(ClearColor(Color::srgb(0.07, 0.09, 0.11)));
    app.insert_resource(common::NavmeshExamplePane {
        goal_x: 6.0,
        goal_z: 0.0,
        agent_speed: 4.2,
        arrival_distance: 0.35,
        crowd_enabled: true,
        crowd_neighbor_distance: 3.8,
        crowd_time_horizon: 1.3,
        ..default()
    });
    app.insert_resource(IntegrationPane::default());
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
    app.add_plugins(DefaultPlugins.set(WindowPlugin {
        primary_window: Some(Window {
            title: "navmesh steering integration".into(),
            resolution: (1480, 920).into(),
            ..default()
        }),
        ..default()
    }));
    app.add_plugins((
        bevy_flair::FlairPlugin,
        bevy_input_focus::InputDispatchPlugin,
        bevy_ui_widgets::UiWidgetsPlugins,
        bevy_input_focus::tab_navigation::TabNavigationPlugin,
        PanePlugin,
    ))
    .register_pane::<common::NavmeshExamplePane>()
    .register_pane::<IntegrationPane>();
    app.add_plugins(NavmeshPlugin::default());
    app.add_plugins(SteeringPlugin::default());
    app.add_systems(Startup, setup);
    app.add_systems(
        Update,
        common::apply_navmesh_steering.before(SteeringSystems::Gather),
    );
    app.add_systems(
        Update,
        sync_navmesh_path_to_steering
            .after(NavmeshSystems::Query)
            .before(SteeringSystems::Evaluate),
    );
    app.add_systems(
        Update,
        sync_panes
            .before(NavmeshSystems::Follow)
            .before(SteeringSystems::Evaluate),
    );
    app.run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    common::spawn_scene_camera(&mut commands);

    let surface = common::spawn_surface(
        &mut commands,
        NavmeshBakeSettings {
            async_baking: false,
            rebuild_debounce_seconds: 0.0,
            ..default()
        },
    );

    for x in -4..=4 {
        for z in -2..=2 {
            let area = if z == 0 {
                saddle_ai_navmesh::NavmeshArea(1)
            } else {
                saddle_ai_navmesh::NavmeshArea(0)
            };
            let color = if z == 0 {
                Color::srgb(0.34, 0.20, 0.16)
            } else {
                Color::srgb(0.15, 0.25, 0.21)
            };
            common::spawn_nav_tile(
                &mut commands,
                &mut meshes,
                &mut materials,
                surface,
                Vec3::new(
                    x as f32 * common::TILE_SIZE,
                    0.0,
                    z as f32 * common::TILE_SIZE,
                ),
                area,
                color,
            );
        }
    }

    for (name, translation, size) in [
        (
            "Island A",
            Vec3::new(-1.0, 0.8, 0.0),
            Vec3::new(1.8, 1.6, 1.8),
        ),
        (
            "Island B",
            Vec3::new(2.6, 0.8, 2.0),
            Vec3::new(1.5, 1.6, 1.5),
        ),
        (
            "Island C",
            Vec3::new(2.8, 0.8, -2.0),
            Vec3::new(1.5, 1.6, 1.5),
        ),
    ] {
        let obstacle = common::spawn_obstacle_box(
            &mut commands,
            &mut meshes,
            &mut materials,
            surface,
            translation,
            size,
            Color::srgb(0.62, 0.28, 0.20),
        );
        commands
            .entity(obstacle)
            .insert(Name::new(name))
            .insert(SteeringObstacle::aabb(size * 0.5));
    }

    let goal = Vec3::new(6.0, 0.0, 0.0);
    let goal_marker = common::spawn_goal_marker(
        &mut commands,
        &mut meshes,
        &mut materials,
        goal,
        Color::srgb(0.98, 0.86, 0.36),
    );
    commands.entity(goal_marker).insert(GoalMarker);

    for (index, offset) in [
        Vec3::new(-6.0, 0.0, -2.4),
        Vec3::new(-6.8, 0.0, -1.2),
        Vec3::new(-7.2, 0.0, 0.0),
        Vec3::new(-6.8, 0.0, 1.2),
        Vec3::new(-6.0, 0.0, 2.4),
        Vec3::new(-5.2, 0.0, -0.6),
        Vec3::new(-5.2, 0.0, 0.8),
    ]
    .into_iter()
    .enumerate()
    {
        let color = Color::hsl(170.0 + index as f32 * 9.0, 0.72, 0.58);
        let visual = common::spawn_agent(
            &mut commands,
            &mut meshes,
            &mut materials,
            &format!("Crowd Agent {index}"),
            surface,
            offset,
            goal,
            color,
            NavmeshQueryFilter::default(),
        );
        commands.entity(visual).insert((
            CrowdAgent,
            NavmeshCrowdAvoidance::default(),
            SteeringAgent::new(SteeringPlane::XZ)
                .with_max_speed(4.2)
                .with_max_acceleration(10.0),
            SteeringAutoApply::default(),
            PathFollowing::new(steering::SteeringPath::new([offset + Vec3::Y * 0.25, goal])),
            Flocking::default(),
            ReciprocalAvoidance::default(),
        ));
    }
}

fn sync_navmesh_path_to_steering(
    mut agents: Query<(&NavmeshPathResult, &mut PathFollowing), With<CrowdAgent>>,
) {
    for (result, mut path_following) in &mut agents {
        let Some(path) = &result.result.path else {
            continue;
        };
        let points = path
            .points
            .iter()
            .map(|point| point.position + Vec3::Y * 0.25)
            .collect::<Vec<_>>();
        if points.len() >= 2 {
            path_following.path.points = points;
        }
    }
}

fn sync_panes(
    navmesh_pane: Res<common::NavmeshExamplePane>,
    integration_pane: Res<IntegrationPane>,
    mut goal_markers: Query<&mut Transform, With<GoalMarker>>,
    mut agents: Query<
        (
            &mut NavmeshAgent,
            &mut NavmeshFollowTarget,
            &mut NavmeshCrowdAvoidance,
            &mut SteeringAgent,
            &mut PathFollowing,
            &mut Flocking,
            &mut ReciprocalAvoidance,
        ),
        With<CrowdAgent>,
    >,
) {
    if !navmesh_pane.is_changed() && !integration_pane.is_changed() {
        return;
    }

    let goal = common::pane_goal(&navmesh_pane);
    for mut transform in &mut goal_markers {
        transform.translation = goal + Vec3::Y * 0.18;
    }
    for (
        mut nav_agent,
        mut target,
        mut crowd,
        mut steering_agent,
        mut path,
        mut flocking,
        mut reciprocal,
    ) in &mut agents
    {
        common::apply_agent_tuning(&mut nav_agent, &navmesh_pane);
        common::apply_crowd_tuning(&mut crowd, &navmesh_pane);
        steering_agent.max_speed = navmesh_pane.agent_speed;
        steering_agent.max_acceleration = integration_pane.max_acceleration;
        path.path.lookahead_distance = integration_pane.path_lookahead;
        flocking.separation_weight = integration_pane.separation_weight;
        flocking.alignment_weight = integration_pane.alignment_weight;
        flocking.cohesion_weight = integration_pane.cohesion_weight;
        reciprocal.neighbor_distance = navmesh_pane.crowd_neighbor_distance;
        reciprocal.time_horizon = navmesh_pane.crowd_time_horizon;
        reciprocal.comfort_distance = navmesh_pane.crowd_comfort_distance;
        *target = NavmeshFollowTarget::Point(goal);
    }
}
