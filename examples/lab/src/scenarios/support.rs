use saddle_bevy_e2e::action::Action;

use crate::LabDiagnostics;

pub(super) fn wait_for_detour_cost_increase(max_frames: u32) -> Action {
    Action::WaitUntil {
        label: "detour route cost increased".into(),
        condition: Box::new(|world| {
            let diagnostics = world.resource::<LabDiagnostics>();
            diagnostics.smoke_detour_cost > diagnostics.smoke_baseline_cost
        }),
        max_frames,
    }
}

pub(super) fn wait_for_rebake_generation_increase(max_frames: u32) -> Action {
    Action::WaitUntil {
        label: "surface generation increased".into(),
        condition: Box::new(|world| {
            let diagnostics = world.resource::<LabDiagnostics>();
            diagnostics.rebake_generation > world.resource::<super::BaselineGeneration>().0
        }),
        max_frames,
    }
}

pub(super) fn wait_for_follow_reached(max_frames: u32) -> Action {
    Action::WaitUntil {
        label: "agent reached goal".into(),
        condition: Box::new(|world| world.resource::<LabDiagnostics>().follow_reached),
        max_frames,
    }
}
