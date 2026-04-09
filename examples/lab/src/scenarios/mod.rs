mod support;

use bevy::prelude::*;
use saddle_bevy_e2e::{action::Action, actions::assertions, scenario::Scenario};

use crate::{LabDiagnostics, set_gate_blocked};
use support::{wait_for_detour_cost_increase, wait_for_follow_reached, wait_for_rebake_generation_increase};

#[derive(Resource, Debug, Clone, Copy, Default)]
struct BaselineGeneration(pub u64);

pub fn list_scenarios() -> Vec<&'static str> {
    vec![
        "smoke_launch",
        "navmesh_smoke",
        "navmesh_detour",
        "navmesh_rebake",
        "navmesh_agent_follow",
        "navmesh_multi_class",
        "navmesh_crowd_follow",
    ]
}

pub fn scenario_by_name(name: &str) -> Option<Scenario> {
    match name {
        "smoke_launch" => Some(smoke_launch()),
        "navmesh_smoke" => Some(navmesh_smoke()),
        "navmesh_detour" => Some(navmesh_detour()),
        "navmesh_rebake" => Some(navmesh_rebake()),
        "navmesh_agent_follow" => Some(navmesh_agent_follow()),
        "navmesh_multi_class" => Some(navmesh_multi_class()),
        "navmesh_crowd_follow" => Some(navmesh_crowd_follow()),
        _ => None,
    }
}

fn set_gate(blocked: bool) -> Action {
    Action::Custom(Box::new(move |world| set_gate_blocked(world, blocked)))
}

fn wait_until_surface_ready() -> Action {
    Action::WaitUntil {
        label: "surface ready".into(),
        condition: Box::new(|world| world.resource::<LabDiagnostics>().surface_ready),
        max_frames: 180,
    }
}

fn wait_until_smoke_path() -> Action {
    Action::WaitUntil {
        label: "smoke path ready".into(),
        condition: Box::new(|world| world.resource::<LabDiagnostics>().smoke_path_cost > 0.0),
        max_frames: 180,
    }
}

fn build_smoke(name: &'static str) -> Scenario {
    Scenario::builder(name)
        .description("Launch the crate-local navmesh lab, wait for the primary path to resolve, and capture the default overlay.")
        .then(wait_until_surface_ready())
        .then(wait_until_smoke_path())
        .then(assertions::custom("primary path resolved", |world| {
            world.resource::<LabDiagnostics>().smoke_path_cost > 0.0
        }))
        .then(Action::Screenshot("smoke".into()))
        .then(Action::WaitFrames(1))
        .then(assertions::log_summary(name))
        .build()
}

fn smoke_launch() -> Scenario {
    build_smoke("smoke_launch")
}

fn navmesh_smoke() -> Scenario {
    build_smoke("navmesh_smoke")
}

fn navmesh_detour() -> Scenario {
    Scenario::builder("navmesh_detour")
        .description("Capture the baseline route, block the central gate, and verify the smoke route takes a more expensive detour.")
        .then(wait_until_surface_ready())
        .then(wait_until_smoke_path())
        .then(Action::Screenshot("detour_before".into()))
        .then(Action::WaitFrames(1))
        .then(set_gate(true))
        .then(Action::WaitUntil {
            label: "path invalidated".into(),
            condition: Box::new(|world| world.resource::<LabDiagnostics>().invalidations > 0),
            max_frames: 180,
        })
        .then(wait_for_detour_cost_increase(180))
        .then(assertions::custom("detour increased route cost", |world| {
            let diagnostics = world.resource::<LabDiagnostics>();
            diagnostics.smoke_detour_cost > diagnostics.smoke_baseline_cost
        }))
        .then(Action::Screenshot("detour_after".into()))
        .then(Action::WaitFrames(1))
        .then(assertions::log_summary("navmesh_detour"))
        .build()
}

fn navmesh_rebake() -> Scenario {
    Scenario::builder("navmesh_rebake")
        .description("Toggle the gate obstacle and verify the surface generation increments while the smoke path remains valid.")
        .then(wait_until_surface_ready())
        .then(wait_until_smoke_path())
        .then(Action::Screenshot("rebake_before".into()))
        .then(Action::WaitFrames(1))
        .then(Action::Custom(Box::new(|world| {
            let generation = world.resource::<LabDiagnostics>().rebake_generation;
            world.insert_resource(BaselineGeneration(generation));
        })))
        .then(set_gate(true))
        .then(wait_for_rebake_generation_increase(180))
        .then(assertions::custom("rebake generation advanced", |world| {
            let baseline = world.resource::<BaselineGeneration>().0;
            let diagnostics = world.resource::<LabDiagnostics>();
            diagnostics.rebake_generation > baseline && diagnostics.smoke_path_cost > 0.0
        }))
        .then(Action::Screenshot("rebake_after".into()))
        .then(Action::WaitFrames(1))
        .then(assertions::log_summary("navmesh_rebake"))
        .build()
}

fn navmesh_agent_follow() -> Scenario {
    Scenario::builder("navmesh_agent_follow")
        .description("Wait for the smoke agent to follow the resolved route and end near its goal.")
        .then(wait_until_surface_ready())
        .then(wait_until_smoke_path())
        .then(Action::Screenshot("follow_start".into()))
        .then(wait_for_follow_reached(420))
        .then(assertions::custom("agent follow reached goal", |world| {
            let diagnostics = world.resource::<LabDiagnostics>();
            diagnostics.follow_reached || diagnostics.follow_distance <= 0.5
        }))
        .then(Action::Screenshot("follow_end".into()))
        .then(Action::WaitFrames(1))
        .then(assertions::log_summary("navmesh_agent_follow"))
        .build()
}

fn navmesh_multi_class() -> Scenario {
    Scenario::builder("navmesh_multi_class")
        .description("Compare the default utility agent with the rough-terrain-averse wheeled agent on the same navmesh.")
        .then(wait_until_surface_ready())
        .then(Action::WaitUntil {
            label: "comparison paths ready".into(),
            condition: Box::new(|world| {
                let diagnostics = world.resource::<LabDiagnostics>();
                diagnostics.utility_cost > 0.0 && diagnostics.wheeled_cost > 0.0
            }),
            max_frames: 180,
        })
        .then(assertions::custom("wheeled route differs from utility route", |world| {
            let diagnostics = world.resource::<LabDiagnostics>();
            diagnostics.wheeled_cost > diagnostics.utility_cost
        }))
        .then(Action::Screenshot("multi_class".into()))
        .then(Action::WaitFrames(1))
        .then(assertions::log_summary("navmesh_multi_class"))
        .build()
}

fn navmesh_crowd_follow() -> Scenario {
    Scenario::builder("navmesh_crowd_follow")
        .description(
            "Let the three lab agents route together and verify crowd avoidance reports nearby followers while the lead route remains valid.",
        )
        .then(wait_until_surface_ready())
        .then(wait_until_smoke_path())
        .then(Action::WaitFrames(120))
        .then(assertions::custom("crowd avoidance observed neighbors", |world| {
            let diagnostics = world.resource::<LabDiagnostics>();
            diagnostics.peak_crowd_neighbors > 0 && diagnostics.smoke_path_cost > 0.0
        }))
        .then(Action::Screenshot("crowd_follow".into()))
        .then(Action::WaitFrames(1))
        .then(assertions::log_summary("navmesh_crowd_follow"))
        .build()
}
