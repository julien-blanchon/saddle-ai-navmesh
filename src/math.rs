use bevy::prelude::*;
use std::cmp::Ordering;

#[derive(Debug, Clone, Copy)]
pub struct NavmeshBasis {
    pub right: Vec3,
    pub forward: Vec3,
    pub up: Vec3,
}

impl NavmeshBasis {
    pub fn from_up(up: Vec3) -> Self {
        let up = up.normalize_or_zero();
        let seed = if up.abs_diff_eq(Vec3::Y, 0.001) {
            Vec3::X
        } else {
            Vec3::Y
        };
        let right = up.cross(seed).normalize_or_zero();
        let forward = right.cross(up).normalize_or_zero();
        Self { right, forward, up }
    }

    pub fn project(&self, point: Vec3) -> Vec2 {
        Vec2::new(point.dot(self.right), point.dot(self.forward))
    }
}

pub fn triangle_normal(a: Vec3, b: Vec3, c: Vec3) -> Vec3 {
    (b - a).cross(c - a).normalize_or_zero()
}

pub fn nearest_point_on_triangle(point: Vec3, a: Vec3, b: Vec3, c: Vec3) -> Vec3 {
    let ab = b - a;
    let ac = c - a;
    let ap = point - a;

    let d1 = ab.dot(ap);
    let d2 = ac.dot(ap);
    if d1 <= 0.0 && d2 <= 0.0 {
        return a;
    }

    let bp = point - b;
    let d3 = ab.dot(bp);
    let d4 = ac.dot(bp);
    if d3 >= 0.0 && d4 <= d3 {
        return b;
    }

    let vc = d1 * d4 - d3 * d2;
    if vc <= 0.0 && d1 >= 0.0 && d3 <= 0.0 {
        let v = d1 / (d1 - d3);
        return a + v * ab;
    }

    let cp = point - c;
    let d5 = ab.dot(cp);
    let d6 = ac.dot(cp);
    if d6 >= 0.0 && d5 <= d6 {
        return c;
    }

    let vb = d5 * d2 - d1 * d6;
    if vb <= 0.0 && d2 >= 0.0 && d6 <= 0.0 {
        let w = d2 / (d2 - d6);
        return a + w * ac;
    }

    let va = d3 * d6 - d5 * d4;
    if va <= 0.0 && (d4 - d3) >= 0.0 && (d5 - d6) >= 0.0 {
        let bc = c - b;
        let w = (d4 - d3) / ((d4 - d3) + (d5 - d6));
        return b + w * bc;
    }

    let denom = 1.0 / (va + vb + vc);
    let v = vb * denom;
    let w = vc * denom;
    a + ab * v + ac * w
}

pub fn point_in_triangle_2d(point: Vec2, tri: [Vec2; 3], epsilon: f32) -> bool {
    let [a, b, c] = tri;
    let area = tri_area2(a, b, c);
    if area.abs() <= epsilon {
        return false;
    }
    let w0 = tri_area2(point, b, c) / area;
    let w1 = tri_area2(a, point, c) / area;
    let w2 = tri_area2(a, b, point) / area;
    w0 >= -epsilon && w1 >= -epsilon && w2 >= -epsilon
}

pub fn tri_area2(a: Vec2, b: Vec2, c: Vec2) -> f32 {
    (b.x - a.x) * (c.y - a.y) - (b.y - a.y) * (c.x - a.x)
}

pub fn quantize_vec2(point: Vec2, epsilon: f32) -> (i64, i64) {
    (
        (point.x / epsilon).round() as i64,
        (point.y / epsilon).round() as i64,
    )
}

pub fn distance_to_segment_2d(point: Vec2, a: Vec2, b: Vec2) -> f32 {
    let ab = b - a;
    let denom = ab.length_squared();
    if denom <= f32::EPSILON {
        return point.distance(a);
    }
    let t = ((point - a).dot(ab) / denom).clamp(0.0, 1.0);
    point.distance(a + ab * t)
}

pub fn convex_hull(points: &[Vec2]) -> Vec<Vec2> {
    if points.len() <= 3 {
        return points.to_vec();
    }

    let mut sorted = points.to_vec();
    sorted.sort_by(|a, b| match a.x.total_cmp(&b.x) {
        Ordering::Equal => a.y.total_cmp(&b.y),
        other => other,
    });

    let mut lower: Vec<Vec2> = Vec::new();
    for point in &sorted {
        while lower.len() >= 2
            && tri_area2(lower[lower.len() - 2], lower[lower.len() - 1], *point) <= 0.0
        {
            lower.pop();
        }
        lower.push(*point);
    }

    let mut upper: Vec<Vec2> = Vec::new();
    for point in sorted.iter().rev() {
        while upper.len() >= 2
            && tri_area2(upper[upper.len() - 2], upper[upper.len() - 1], *point) <= 0.0
        {
            upper.pop();
        }
        upper.push(*point);
    }

    lower.pop();
    upper.pop();
    lower.extend(upper);
    lower
}

pub fn point_in_polygon(point: Vec2, polygon: &[Vec2]) -> bool {
    if polygon.is_empty() {
        return false;
    }

    let mut inside = false;
    let mut previous = *polygon.last().unwrap();
    for &current in polygon {
        let intersects = ((current.y > point.y) != (previous.y > point.y))
            && (point.x
                < (previous.x - current.x) * (point.y - current.y)
                    / (previous.y - current.y + f32::EPSILON)
                    + current.x);
        if intersects {
            inside = !inside;
        }
        previous = current;
    }
    inside
}

pub fn point_near_polygon(point: Vec2, polygon: &[Vec2], radius: f32) -> bool {
    if point_in_polygon(point, polygon) {
        return true;
    }

    if polygon.len() < 2 {
        return false;
    }

    let mut previous = *polygon.last().unwrap();
    for &current in polygon {
        if distance_to_segment_2d(point, previous, current) <= radius {
            return true;
        }
        previous = current;
    }
    false
}

pub fn segments_intersect_2d(a: Vec2, b: Vec2, c: Vec2, d: Vec2, epsilon: f32) -> bool {
    fn approx_zero(value: f32, epsilon: f32) -> bool {
        value.abs() <= epsilon
    }

    fn point_on_segment(point: Vec2, start: Vec2, end: Vec2, epsilon: f32) -> bool {
        let min = start.min(end) - Vec2::splat(epsilon);
        let max = start.max(end) + Vec2::splat(epsilon);
        point.cmpge(min).all() && point.cmple(max).all()
    }

    let ab_c = tri_area2(a, b, c);
    let ab_d = tri_area2(a, b, d);
    let cd_a = tri_area2(c, d, a);
    let cd_b = tri_area2(c, d, b);

    if approx_zero(ab_c, epsilon) && point_on_segment(c, a, b, epsilon) {
        return true;
    }
    if approx_zero(ab_d, epsilon) && point_on_segment(d, a, b, epsilon) {
        return true;
    }
    if approx_zero(cd_a, epsilon) && point_on_segment(a, c, d, epsilon) {
        return true;
    }
    if approx_zero(cd_b, epsilon) && point_on_segment(b, c, d, epsilon) {
        return true;
    }

    (ab_c > epsilon) != (ab_d > epsilon) && (cd_a > epsilon) != (cd_b > epsilon)
}
