# `saddle-ai-navmesh` Configuration

## `NavmeshBakeSettings`

| Field | Type | Default | Meaningful range | Effect |
| --- | --- | --- | --- | --- |
| `max_slope_degrees` | `f32` | `50.0` | `0.0..=89.0` | Rejects triangles whose normal is too steep against the nav up vector. Lower values make the navmesh more conservative. |
| `max_step_height` | `f32` | `0.75` | `>= 0.0` | Rejects adjacency between triangles whose shared edge midpoint differs too much in height. Larger values allow harsher stair or ledge transitions. |
| `agent_radius` | `f32` | `0.35` | `>= 0.0` | Inflates obstacle footprints during triangle culling. Larger values pull the usable mesh farther away from blockers. |
| `rebuild_debounce_seconds` | `f32` | `0.1` | `>= 0.0` | Delays bake starts after change detection. Higher values collapse rapid edits into fewer bakes at the cost of responsiveness. |
| `async_baking` | `bool` | `true` | `true/false` | When enabled, baking runs on `AsyncComputeTaskPool`. Disable for deterministic single-threaded tests or very small navmeshes. |
| `quantization` | `f32` | `0.001` | `> 0.0` | Epsilon used for edge matching and geometric comparisons. Too high merges unrelated edges; too low can miss portals due to float drift. |
| `up` | `Vec3` | `Vec3::Y` | normalized non-zero vector | Defines the bake/query projection basis. Keep it aligned with your world's gravity or authored floor orientation. |

## `NavmeshQuerySettings`

| Field | Type | Default | Meaningful range | Effect |
| --- | --- | --- | --- | --- |
| `projection_policy` | `NavmeshProjectionPolicy` | `ProjectToNearest` | enum | Controls whether off-mesh endpoints are auto-projected or rejected. |
| `allow_partial` | `bool` | `true` | `true/false` | If `true`, queries can return the closest reached corridor when the goal is unreachable. |
| `nearest_reachable_fallback` | `bool` | `true` | `true/false` | If `true`, unreachable goals may return a partial path to the reached polygon with the best heuristic distance. If `false`, unreachable goals stay `Unreachable` even when `allow_partial` is enabled. |
| `smoothing` | `NavmeshPathSmoothing` | `Funnel` | enum | `Funnel` string-pulls the corridor. `None` emits a conservative polyline through portal midpoints. |
| `epsilon` | `f32` | `0.0001` | `> 0.0` | Numerical tolerance used by point-in-triangle tests and portal comparisons. |

## `NavmeshQueryFilter`

| Field | Type | Default | Effect |
| --- | --- | --- | --- |
| `mask` | `NavmeshAreaMask` | `all()` | Only polygons whose mask intersects this value may be traversed. |
| `link_mask` | `NavmeshAreaMask` | `all()` | Controls which off-mesh links are allowed. |
| `area_costs` | `Vec<NavmeshAreaCost>` | empty | Multiplies traversal cost for matching areas. Missing areas use cost `1.0`. |

### Area Costs

- `NavmeshAreaMask` uses a `u64`, so mask-addressable `NavmeshArea` ids are `0..=63`.
- Out-of-range area ids degrade to an empty mask instead of panicking.
- Costs below `1.0` are clamped to `1.0`.
- Costs behave as hints, not hard guarantees.
- Higher costs make the search prefer alternate corridors when they are still competitive.

## `NavmeshAgent`

| Field | Type | Default | Meaningful range | Effect |
| --- | --- | --- | --- | --- |
| `max_speed` | `f32` | `3.0` | `>= 0.0` | Used only for `desired_velocity` output. |
| `arrival_distance` | `f32` | `0.25` | `>= 0.0` | Goal is considered reached once remaining distance is below this threshold. |
| `waypoint_distance` | `f32` | `0.2` | `>= 0.0` | Distance threshold for advancing to the next waypoint. |
| `overshoot_distance` | `f32` | `0.1` | `>= 0.0` | Extra segment progress tolerated before the follower advances a waypoint after overshooting it. |
| `repath_interval_seconds` | `f32` | `0.35` | `>= 0.0` | Minimum time between automatic repath requests. |
| `filter` | `NavmeshQueryFilter` | default | Traversal class for this agent. |
| `query_settings` | `NavmeshQuerySettings` | default | Query behavior for this agent. |

## `NavmeshCrowdAvoidance`

| Field | Type | Default | Meaningful range | Effect |
| --- | --- | --- | --- | --- |
| `enabled` | `bool` | `true` | `true/false` | Master toggle for the local crowd-avoidance pass. |
| `body_radius` | `f32` | `0.35` | `>= 0.0` | Physical footprint used for overlap prediction. |
| `neighbor_distance` | `f32` | `3.0` | `>= 0.0` | Maximum distance used to gather nearby follower agents. |
| `time_horizon` | `f32` | `1.0` | `> 0.0` recommended | Prediction window for relative collision checks. |
| `comfort_distance` | `f32` | `0.1` | `>= 0.0` | Extra spacing added beyond the summed body radii. |
| `side_bias` | `f32` | `1.0` | `>= 0.0` | Lateral sidestep strength relative to braking. |
| `max_neighbors` | `usize` | `8` | `>= 1` | Upper bound on crowd neighbors sampled per agent. |

## `NavmeshDebugSettings`

| Field | Type | Default | Effect |
| --- | --- | --- | --- |
| `enabled` | `bool` | `true` | Master toggle. |
| `draw_surface` | `bool` | `true` | Draws triangle wireframes. |
| `draw_portals` | `bool` | `true` | Draws shared-edge portals between adjacent polygons. |
| `draw_links` | `bool` | `true` | Draws off-mesh links. |
| `draw_paths` | `bool` | `true` | Draws active path results. |
| `draw_projections` | `bool` | `true` | Draws projected start and goal points for requests. |
| `draw_agents` | `bool` | `true` | Draws steering output arrows and waypoint targets. |
| `surface_depth_bias` | `f32` | `0.0` | Applied to Bevy's default gizmo config group depth bias so navmesh lines can be nudged in front of or behind scene geometry. |

## Runtime Strategy Notes

- Dirty bounds are accumulated for metrics and invalidation, but v0.1 still rebakes the full surface.
- Off-mesh links are snapped to the nearest polygon within their `snap_distance`.
- Obstacle subtraction is triangle-granularity. If blockers should carve crisp holes, prefer navmesh-authored floor geometry or smaller floor triangles.
- `NavmeshCrowdAvoidance` is a local follower-side adjustment. It does not change the baked surface or path result; it only bends the published steering intent.
