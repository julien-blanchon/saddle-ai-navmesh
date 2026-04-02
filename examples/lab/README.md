# Navmesh Lab

Crate-local verification app for the shared `navmesh` runtime.

## Purpose

- exercise baking, detours, weighted areas, follower output, and rebake invalidation in one deterministic scene
- keep BRP-friendly navmesh, agent, and diagnostics resources available for runtime inspection
- provide screenshot-backed E2E gates for smoke, detour, rebake, follow, and traversal-class behavior

## Run

```bash
cargo run -p navmesh_lab
```

## Headless

```bash
NAVMESH_LAB_HEADLESS=1 cargo run -p navmesh_lab
```

Headless mode keeps ECS inspection and BRP workflows available without a renderer. It is intended for live state inspection, not screenshot capture.

## E2E

```bash
cargo run -p navmesh_lab --features e2e -- smoke_launch
cargo run -p navmesh_lab --features e2e -- navmesh_smoke
cargo run -p navmesh_lab --features e2e -- navmesh_detour
cargo run -p navmesh_lab --features e2e -- navmesh_rebake
cargo run -p navmesh_lab --features e2e -- navmesh_agent_follow
cargo run -p navmesh_lab --features e2e -- navmesh_multi_class
```

## BRP

```bash
NAVMESH_LAB_BRP_PORT=15714 \
uv run --project .codex/skills/bevy-brp/script brp app launch navmesh_lab
uv run --project .codex/skills/bevy-brp/script brp world query bevy_ecs::name::Name
uv run --project .codex/skills/bevy-brp/script brp world query navmesh::components::NavmeshAgent
uv run --project .codex/skills/bevy-brp/script brp resource get navmesh::resources::NavmeshDiagnostics
uv run --project .codex/skills/bevy-brp/script brp extras screenshot /tmp/navmesh_lab.png
uv run --project .codex/skills/bevy-brp/script brp extras shutdown
```
