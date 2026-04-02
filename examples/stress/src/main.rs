use std::time::Instant;

use bevy::prelude::*;
use saddle_ai_saddle_ai_navmesh::{
    NavmeshArea, NavmeshAreaCost, NavmeshBakeSettings, NavmeshBuildInput, NavmeshPathId,
    NavmeshPathSmoothing, NavmeshPrimitive, NavmeshQueryFilter, NavmeshQuerySettings,
    NavmeshSourceGeometry, NavmeshSourceKind, bake_navmesh,
};

const GRID_X: i32 = 4;
const GRID_Z: i32 = 4;
const TILE_SIZE: f32 = 1.5;
const QUERY_COUNT: usize = 4;

fn main() {
    let input = build_grid_input();

    let wall_start = Instant::now();
    let surface = bake_navmesh(&input, &NavmeshBakeSettings::default()).expect("stress bake");
    let wall_bake_ms = wall_start.elapsed().as_secs_f64() * 1000.0;

    let query_settings = NavmeshQuerySettings {
        smoothing: NavmeshPathSmoothing::Funnel,
        ..default()
    };
    let filter = NavmeshQueryFilter {
        area_costs: vec![NavmeshAreaCost::new(NavmeshArea(1), 2.5)],
        ..default()
    };

    let query_started = Instant::now();
    let mut success = 0_usize;
    let mut partial = 0_usize;
    let mut total_query_ms = 0.0_f64;
    let mut longest = 0.0_f32;
    for index in 0..QUERY_COUNT {
        let start = sample_point(index, -1);
        let goal = sample_point(index, 1);
        let result = surface.query_path(
            NavmeshPathId(index as u64 + 1),
            start,
            goal,
            &query_settings,
            &filter,
        );
        total_query_ms += f64::from(result.duration_ms);
        if let Some(path) = &result.path {
            longest = longest.max(path.total_length);
        }
        match result.status {
            saddle_ai_navmesh::NavmeshPathStatus::Success => success += 1,
            saddle_ai_navmesh::NavmeshPathStatus::Partial => partial += 1,
            _ => {}
        }
    }
    let wall_query_ms = query_started.elapsed().as_secs_f64() * 1000.0;

    println!("navmesh stress");
    println!("  sources: {}", input.sources.len());
    println!("  polygons: {}", surface.polygons.len());
    println!("  portals: {}", surface.portals.len());
    println!("  links: {}", surface.links.len());
    println!("  bake stats ms: {:.2}", surface.stats.last_bake_ms);
    println!("  bake wall ms: {:.2}", wall_bake_ms);
    println!("  queries: {}", QUERY_COUNT);
    println!("  query success: {}", success);
    println!("  query partial: {}", partial);
    println!("  query reported ms total: {:.2}", total_query_ms);
    println!("  query wall ms total: {:.2}", wall_query_ms);
    println!(
        "  query wall ms avg: {:.4}",
        wall_query_ms / QUERY_COUNT as f64
    );
    println!("  longest path length: {:.2}", longest);
}

fn build_grid_input() -> NavmeshBuildInput {
    let mut sources = Vec::new();
    let mut source_id = 1_u64;

    for x in 0..GRID_X {
        for z in 0..GRID_Z {
            let translation = Vec3::new(
                (x as f32 - GRID_X as f32 * 0.5) * TILE_SIZE,
                0.0,
                (z as f32 - GRID_Z as f32 * 0.5) * TILE_SIZE,
            );

            let area = if z > GRID_Z / 2 {
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

fn sample_point(index: usize, side: i32) -> Vec3 {
    let lane = (index as i32 % GRID_Z) - GRID_Z / 2;
    let x = (side as f32) * (GRID_X as f32 * 0.5 - 1.0) * TILE_SIZE;
    Vec3::new(x, 0.0, lane as f32 * TILE_SIZE * 0.6)
}
