use bevy::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Reflect, Default)]
#[reflect(Default)]
pub struct NavmeshArea(pub u8);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Reflect)]
#[reflect(Default)]
pub struct NavmeshAreaMask(pub u64);

impl NavmeshAreaMask {
    pub const fn all() -> Self {
        Self(u64::MAX)
    }

    pub const fn empty() -> Self {
        Self(0)
    }

    pub const fn from_area(area: NavmeshArea) -> Self {
        if area.0 < u64::BITS as u8 {
            Self(1_u64 << area.0)
        } else {
            Self::empty()
        }
    }

    pub const fn intersects(self, other: Self) -> bool {
        (self.0 & other.0) != 0
    }

    pub const fn contains_area(self, area: NavmeshArea) -> bool {
        self.intersects(Self::from_area(area))
    }

    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
}

impl Default for NavmeshAreaMask {
    fn default() -> Self {
        Self::all()
    }
}

#[derive(Debug, Clone, PartialEq, Reflect)]
#[reflect(Default)]
pub struct NavmeshAreaCost {
    pub area: NavmeshArea,
    pub multiplier: f32,
}

impl NavmeshAreaCost {
    pub const fn new(area: NavmeshArea, multiplier: f32) -> Self {
        Self { area, multiplier }
    }
}

impl Default for NavmeshAreaCost {
    fn default() -> Self {
        Self::new(NavmeshArea::default(), 1.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Reflect, Default)]
pub enum NavmeshProjectionPolicy {
    RequireOnMesh,
    #[default]
    ProjectToNearest,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Reflect, Default)]
pub enum NavmeshPathSmoothing {
    None,
    #[default]
    Funnel,
}

#[derive(Debug, Clone, PartialEq, Reflect)]
#[reflect(Default)]
pub struct NavmeshQuerySettings {
    pub projection_policy: NavmeshProjectionPolicy,
    pub allow_partial: bool,
    pub nearest_reachable_fallback: bool,
    pub smoothing: NavmeshPathSmoothing,
    pub epsilon: f32,
}

impl Default for NavmeshQuerySettings {
    fn default() -> Self {
        Self {
            projection_policy: NavmeshProjectionPolicy::ProjectToNearest,
            allow_partial: true,
            nearest_reachable_fallback: true,
            smoothing: NavmeshPathSmoothing::Funnel,
            epsilon: 0.0001,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Reflect, Default)]
#[reflect(Default)]
pub struct NavmeshQueryFilter {
    pub mask: NavmeshAreaMask,
    pub link_mask: NavmeshAreaMask,
    pub area_costs: Vec<NavmeshAreaCost>,
}

impl NavmeshQueryFilter {
    pub fn cost_for_area(&self, area: NavmeshArea) -> f32 {
        self.area_costs
            .iter()
            .find(|entry| entry.area == area)
            .map(|entry| entry.multiplier.max(1.0))
            .unwrap_or(1.0)
    }

    pub fn allows_area(&self, mask: NavmeshAreaMask) -> bool {
        self.mask.intersects(mask)
    }

    pub fn allows_link(&self, mask: NavmeshAreaMask) -> bool {
        self.link_mask.intersects(mask)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Reflect, Default)]
pub enum NavmeshBakeState {
    #[default]
    Empty,
    Dirty,
    Baking,
    Ready,
    Failed,
}

#[derive(Component, Debug, Clone, PartialEq, Reflect)]
#[reflect(Component, Default)]
pub struct NavmeshBakeSettings {
    pub max_slope_degrees: f32,
    pub max_step_height: f32,
    pub agent_radius: f32,
    pub rebuild_debounce_seconds: f32,
    pub async_baking: bool,
    pub quantization: f32,
    pub up: Vec3,
}

impl Default for NavmeshBakeSettings {
    fn default() -> Self {
        Self {
            max_slope_degrees: 50.0,
            max_step_height: 0.75,
            agent_radius: 0.35,
            rebuild_debounce_seconds: 0.1,
            async_baking: true,
            quantization: 0.001,
            up: Vec3::Y,
        }
    }
}

#[derive(Resource, Debug, Clone, PartialEq, Reflect)]
#[reflect(Resource, Default)]
pub struct NavmeshDebugSettings {
    pub enabled: bool,
    pub draw_surface: bool,
    pub draw_portals: bool,
    pub draw_links: bool,
    pub draw_paths: bool,
    pub draw_projections: bool,
    pub draw_agents: bool,
    pub surface_depth_bias: f32,
}

impl Default for NavmeshDebugSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            draw_surface: true,
            draw_portals: true,
            draw_links: true,
            draw_paths: true,
            draw_projections: true,
            draw_agents: true,
            surface_depth_bias: 0.0,
        }
    }
}

#[cfg(test)]
#[path = "config_tests.rs"]
mod tests;
