# `saddle-ai-navmesh` Architecture

## Runtime Model

The crate is split into five layers:

1. Geometry ingestion
2. Bake pipeline
3. Query pipeline
4. Follower pipeline
5. Debug and diagnostics

Stateful orchestration lives in ECS. Geometry, corridor, and search logic live in plain Rust.

## Surface Ownership

Each navmesh instance is represented by one entity:

- `NavmeshSurface`
- `NavmeshBakeSettings`
- `NavmeshSurfaceStatus`
- `NavmeshSurfaceData` once a bake succeeds

Source entities point at a surface entity through `NavmeshSource.surface`.

This keeps multiple independent navmeshes cheap to reason about:

- one dungeon floor can bake separately from another
- different scenes can keep their own settings
- a crate-local lab can expose multiple surfaces without global singleton state

## Bake Pipeline

```
source entities / pure build input
    -> collect triangles and link inputs
    -> project to nav basis
    -> classify walkable triangles by slope
    -> build obstacle footprints
    -> cull blocked triangles
    -> extract adjacency portals
    -> attach off-mesh links
    -> publish NavmeshSurfaceData
```

### Source collection

The ECS collector supports:

- `Mesh3d` sources
- `NavmeshPrimitiveSource`
- explicit pure `NavmeshBuildInput` via `bake_navmesh`

Every collected source is normalized into crate-owned triangle soup plus metadata:

- source id
- source kind
- area id
- traversal mask
- transformed world vertices

### Walkability filtering

Triangles are rejected when:

- their normal is degenerate
- their slope exceeds `max_slope_degrees`
- their centroid falls inside an obstacle footprint or within the configured agent radius of it

### Portal extraction

The bake keeps triangles instead of merging them into larger polygons. That keeps v0.1 simpler while still giving clean corridor extraction:

- shared edges become portals
- portals are stored explicitly for queries and debug drawing
- adjacency is filtered by `max_step_height`

### Runtime rebuild policy

Dirty regions are accumulated per surface, but the actual rebuild policy is:

- debounce source changes for `rebuild_debounce_seconds`
- rebuild the full surface asynchronously
- publish the new surface only when the task completes

This keeps the runtime predictable and correct while leaving space for future tiled backends.

## Query Pipeline

```
world-space start/goal
    -> nearest-point projection
    -> polygon graph search
    -> corridor reconstruction
    -> optional funnel smoothing
    -> NavmeshPathQueryResult
```

### Projection

Projection is triangle-based:

- every triangle is considered for nearest point
- filters reject disallowed area masks
- the result includes the winning polygon index and projected point

If the query settings require on-mesh endpoints, projection failure becomes a terminal status. Otherwise the query can auto-project.

### Graph search

The search graph includes:

- triangle-to-triangle portals
- off-mesh links

Traversal cost is:

- Euclidean step cost
- multiplied by area-cost overrides from `NavmeshQueryFilter`
- multiplied by link cost for off-mesh links

If the goal is unreachable and partial paths are allowed, the search returns the best reached node by heuristic distance to the goal.

### Corridor and funnel

The raw result preserves:

- polygon corridor
- portal list

Then `NavmeshPathSmoothing::Funnel` string-pulls the corridor in the nav basis plane and lifts the result back onto the surface using the corridor polygons.

`NavmeshPathSmoothing::None` does not collapse the route to a straight line. It emits a conservative raw corridor polyline through portal midpoints so path-following can still respect blocked space without using funnel smoothing.

## Follower Pipeline

Followers are intentionally separate from path queries.

```
NavmeshAgent + NavmeshFollowTarget
    -> request refresh decision
    -> NavmeshPathRequest
    -> NavmeshPathResult
    -> waypoint advancement
    -> NavmeshSteeringOutput
```

The follower:

- repaths on interval
- repaths when the target changes enough
- marks paths stale when the surface generation changes
- advances waypoints using arrival and overshoot tolerances

The follower does not mutate transforms. It only emits movement intent.

`NavmeshPath.total_length` reports the produced follow polyline length. `NavmeshPath.total_cost` keeps the weighted search estimate so area costs and off-mesh-link multipliers stay visible to diagnostics and higher-level AI.

## Async Flow

```
dirty source / explicit rebuild request
    -> mark surface dirty
    -> wait for debounce deadline
    -> spawn async bake task
    -> poll task in ECS
    -> update surface component + status + diagnostics
```

Bake tasks return a crate-owned output object. ECS applies that output on the main world once the task is ready.

## System Ordering

`NavmeshSystems` are ordered as:

1. `DetectChanges`
2. `Bake`
3. `Query`
4. `Follow`
5. `Debug`

That yields these guarantees:

- requests see the latest completed surface
- followers read query results from the same frame
- debug reflects the final state for the frame

## Debug Derivation

Debug drawing reads live ECS data only:

- `NavmeshSurfaceData` for triangles, portals, and links
- `NavmeshPathResult` for active paths
- `NavmeshPathRequest` for projected start and goal
- `NavmeshSteeringOutput` for follower state

No shadow debug cache is maintained. `NavmeshDebugSettings::surface_depth_bias` is forwarded into Bevy's default gizmo config group so crate users can lift the wireframe in front of scene geometry when inspecting dense levels.

## Invalidations

When a surface becomes dirty or its generation changes:

- existing results are marked pending or stale
- `NavmeshPathInvalidated` is published
- followers request a refresh on their next update

This keeps query consumers aware of topology churn without forcing the crate to own locomotion rollback or reconciliation.
