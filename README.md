# Saddle AI Navmesh

Reusable navmesh baking, querying, and path-following infrastructure for Bevy.

The crate owns reusable navigation data, not game AI. It focuses on:

- baking reusable navmesh surfaces from mesh or primitive source geometry
- explicit surface entities so one world can host multiple navmeshes
- point projection, path queries, corridor extraction, and funnel smoothing
- decoupled path following that outputs desired movement instead of moving bodies directly
- async rebake orchestration, dirty-region tracking, and first-class debug drawing

`saddle-ai-navmesh` deliberately keeps locomotion, animation, crowd avoidance, and decision-making outside the crate. Consumers decide how to move an entity once they receive steering output.

For apps where navigation should remain live for the whole app lifetime, prefer `NavmeshPlugin::always_on(Update)`. Use `NavmeshPlugin::new(...)` when activation should follow explicit schedules such as `OnEnter` and `OnExit`.

## Quick Start

```toml
[dependencies]
saddle-ai-navmesh = { git = "https://github.com/julien-blanchon/saddle-ai-navmesh" }
```

```rust,no_run
use bevy::prelude::*;
use saddle_ai_navmesh::{
    NavmeshAgent, NavmeshBakeSettings, NavmeshFollowTarget, NavmeshPathSmoothing,
    NavmeshPlugin, NavmeshPrimitive, NavmeshPrimitiveSource, NavmeshQueryFilter,
    NavmeshQuerySettings, NavmeshSource, NavmeshSourceKind, NavmeshSurface,
};

#[derive(States, Default, Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum DemoState {
    #[default]
    Active,
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .init_state::<DemoState>()
        .add_plugins(NavmeshPlugin::new(
            OnEnter(DemoState::Active),
            OnExit(DemoState::Active),
            Update,
        ))
        .insert_resource(saddle_ai_navmesh::NavmeshDebugSettings {
            enabled: true,
            draw_surface: true,
            draw_paths: true,
            ..default()
        })
        .add_systems(Startup, setup)
        .run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let surface = commands
        .spawn((
            Name::new("Main Navmesh Surface"),
            NavmeshSurface::default(),
            NavmeshBakeSettings::default(),
        ))
        .id();

    commands.spawn((
        Name::new("Walkable Floor"),
        NavmeshSource::new(surface, NavmeshSourceKind::Walkable),
        NavmeshPrimitiveSource::new(NavmeshPrimitive::Quad {
            size: Vec2::new(14.0, 14.0),
        }),
        Mesh3d(meshes.add(Plane3d::default().mesh().size(14.0, 14.0))),
        MeshMaterial3d(materials.add(Color::srgb(0.14, 0.18, 0.22))),
        Transform::default(),
    ));

    commands.spawn((
        Name::new("Column Obstacle"),
        NavmeshSource::new(surface, NavmeshSourceKind::Obstacle),
        NavmeshPrimitiveSource::new(NavmeshPrimitive::Cuboid {
            size: Vec3::new(2.0, 2.0, 2.0),
        }),
        Mesh3d(meshes.add(Cuboid::new(2.0, 2.0, 2.0))),
        MeshMaterial3d(materials.add(Color::srgb(0.5, 0.22, 0.18))),
        Transform::from_xyz(0.0, 1.0, 0.0),
    ));

    commands.spawn((
        Name::new("Navigation Agent"),
        NavmeshAgent::new(surface)
            .with_max_speed(4.0)
            .with_query_settings(NavmeshQuerySettings {
                smoothing: NavmeshPathSmoothing::Funnel,
                ..default()
            })
            .with_filter(NavmeshQueryFilter::default()),
        NavmeshFollowTarget::Point(Vec3::new(5.5, 0.0, 5.0)),
        Transform::from_xyz(-5.5, 0.0, -5.0),
        GlobalTransform::default(),
    ));
}
```

## Public API

- Plugin: `NavmeshPlugin`
- System sets:
  `NavmeshSystems::{DetectChanges, Bake, Query, Follow, Debug}`
- Core surface components:
  `NavmeshSurface`, `NavmeshBakeSettings`, `NavmeshSurfaceStatus`, `NavmeshSurfaceData`
- Source authoring:
  `NavmeshSource`, `NavmeshSourceKind`, `NavmeshPrimitiveSource`, `NavmeshPrimitive`,
  `NavmeshLinkSource`
- Query surface:
  `NavmeshPathRequest`, `NavmeshPathResult`, `NavmeshQueryFilter`,
  `NavmeshQuerySettings`, `NavmeshPathStatus`, `NavmeshPathQueryResult`
- Following:
  `NavmeshAgent`, `NavmeshFollowTarget`, `NavmeshFollowerState`, `NavmeshSteeringOutput`
- Messages:
  `NavmeshRebuildRequested`, `NavmeshBakeCompleted`, `NavmeshPathReady`,
  `NavmeshPathInvalidated`
- Pure helpers:
  `bake_navmesh`, `nearest_point_on_navmesh`, `query_navmesh_path`,
  `NavmeshSurfaceData::{nearest_point, query_path, line_of_sight}`

## Baking Workflow

1. Spawn a surface entity with `NavmeshSurface` and `NavmeshBakeSettings`.
2. Spawn source entities tagged with `NavmeshSource { surface, kind, ... }`.
3. Provide geometry through either:
   - `Mesh3d` plus a mesh asset
   - `NavmeshPrimitiveSource`
   - pure `NavmeshBuildInput` passed directly to `bake_navmesh`
4. Let the plugin collect sources, track dirty bounds, and rebake asynchronously.
5. Read the resulting `NavmeshSurfaceData` and `NavmeshSurfaceStatus` from the surface entity.

v0.1 tracks dirty regions precisely but still rebuilds the full surface for each bake. The dirty bounds are exposed for metrics, invalidation, and future tiled backends.
Direct physics-collider extraction is intentionally left to consumer-side adapters so the shared crate keeps a pure `bevy` dependency surface.

## Query Workflow

- Use pure queries when you already have `&NavmeshSurfaceData`.
- Use `NavmeshPathRequest` when you want ECS orchestration and message publication.
- Start/goal projection is controlled by `NavmeshQuerySettings::projection_policy`.
- Weighted areas and traversal masks are controlled through `NavmeshQueryFilter`.
- `NavmeshQuerySettings::nearest_reachable_fallback` controls whether unreachable goals may return partial paths.
- `NavmeshSurfaceData::line_of_sight` samples the projected straight segment across the allowed surface, so it can validate direct shortcuts across many polygons instead of only same-polygon checks.
- Query results separate status, projected endpoints, raw corridor, and smoothed points.

## Path Following

`NavmeshAgent` plus `NavmeshFollowTarget` produces `NavmeshSteeringOutput`.

The crate does not move the entity. Consumers should read:

- `desired_direction`
- `desired_velocity`
- `next_target`
- `remaining_distance`

and feed those values into their own movement controller, character motor, or animation logic.

## Examples

| Example | Purpose | Run |
| --- | --- | --- |
| `basic` | Minimal bake, projection, and path query | `cargo run -p saddle-ai-navmesh-example-basic` |
| `dynamic_obstacles` | Source motion, dirty tracking, and rebake flow | `cargo run -p saddle-ai-navmesh-example-dynamic-obstacles` |
| `multi_agent_classes` | Area masks and weighted traversal classes | `cargo run -p saddle-ai-navmesh-example-multi-agent-classes` |
| `offmesh_links` | Gap traversal via explicit off-mesh links | `cargo run -p saddle-ai-navmesh-example-offmesh-links` |
| `procedural_rebake` | Geometry added after startup with explicit rebake | `cargo run -p saddle-ai-navmesh-example-procedural-rebake` |
| `stress` | Reproducible bake/query throughput timing without a windowed app | `cargo run -p saddle-ai-navmesh-example-stress` |
| `saddle-ai-navmesh-lab` | Rich crate-local showcase with BRP and E2E hooks | `cargo run -p saddle-ai-navmesh-lab` |

## Crate-Local Lab

`shared/ai/saddle-ai-navmesh/examples/lab` keeps runtime inspection and E2E scenarios inside the shared crate itself.

```bash
cargo run -p saddle-ai-navmesh-lab
```

E2E commands:

```bash
cargo run -p saddle-ai-navmesh-lab --features e2e -- smoke_launch
cargo run -p saddle-ai-navmesh-lab --features e2e -- navmesh_smoke
cargo run -p saddle-ai-navmesh-lab --features e2e -- navmesh_detour
cargo run -p saddle-ai-navmesh-lab --features e2e -- navmesh_rebake
cargo run -p saddle-ai-navmesh-lab --features e2e -- navmesh_agent_follow
cargo run -p saddle-ai-navmesh-lab --features e2e -- navmesh_multi_class
```

## BRP

Useful BRP commands against the lab:

```bash
NAVMESH_LAB_BRP_PORT=15714 \
uv run --active --project .codex/skills/bevy-brp/script brp app launch saddle-ai-navmesh-lab
uv run --active --project .codex/skills/bevy-brp/script brp world query bevy_ecs::name::Name
uv run --active --project .codex/skills/bevy-brp/script brp world query saddle_ai_navmesh::components::NavmeshAgent
uv run --active --project .codex/skills/bevy-brp/script brp world query saddle_ai_navmesh::components::NavmeshSurfaceStatus
uv run --active --project .codex/skills/bevy-brp/script brp resource get saddle_ai_navmesh::resources::NavmeshDiagnostics
uv run --active --project .codex/skills/bevy-brp/script brp extras screenshot /tmp/navmesh_lab.png
uv run --active --project .codex/skills/bevy-brp/script brp extras shutdown
```

If the local renderer is unavailable, launch the lab headlessly for BRP-only inspection:

```bash
NAVMESH_LAB_HEADLESS=1 cargo run -p saddle-ai-navmesh-lab
```

Headless mode preserves ECS/runtime inspection but does not support screenshots or E2E capture.

## Dependency Philosophy

`saddle-ai-navmesh` owns its bake, query, and follow logic. It studies Recast, Godot, Unity, and existing Rust crates, but it does not wrap them at runtime.

The only runtime dependency is `bevy`.

## Limitations and Deferred Features

v0.1 intentionally ships a smaller but production-usable slice:

- dirty regions are tracked, but rebuilds are still full-surface
- obstacle subtraction is triangle-granularity and works best with navmesh-authored or reasonably tessellated walkable meshes
- area masks are backed by a `u64`, so `NavmeshArea` values `0..=63` are addressable by masks; larger ids are ignored by mask helpers instead of panicking
- the follower outputs movement intent only; local avoidance and crowd resolution stay outside the crate
- `NavmeshPathSmoothing::None` follows portal midpoints instead of applying funnel string-pulling
- the line-of-sight helper is a navmesh shortcut check, not a physics visibility query
- no binary navmesh serialization format is shipped yet

## More Docs

- [Architecture](docs/architecture.md)
- [Configuration](docs/configuration.md)
