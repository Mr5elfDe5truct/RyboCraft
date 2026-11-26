//! Which way up to print a part.

use crate::{PrintMesh, normal_area};
use rayon::prelude::*;
use serde::Serialize;
use rybocore_record::Vec3;
use rybocraft_geom::prelude::*;

/// What matters in an orientation; the weights are relative.
#[derive(Clone, Copy, Debug)]
pub struct Weights {
    pub support: f64,
    pub height: f64,
    pub contact: f64,
}

#[derive(Clone, Debug, Serialize)]
pub struct Orientation {
    /// The direction (in the part's frame) that ends up facing the bed.
    pub down: [f64; 3],
    pub overhang_area: f64,
    /// Overhang area times the height it hangs at: how much support it wants.
    pub support_volume: f64,
    pub height: f64,
    pub contact_area: f64,
    /// Lower is better.
    pub score: f64,
}

/// The shortest turn that takes unit `down` to −Z: a unit axis and an angle in radians.
pub fn axis_angle(down: Vec3) -> (Vec3, f64) {
    let to = Vec3::new(0.0, 0.0, -1.0);
    let c = down.dot(to).clamp(-1.0, 1.0);
    match down.cross(to).normalized() {
        Some(axis) if c.abs() < 1.0 - 1e-12 => (axis, c.acos()),
        _ if c > 0.0 => (Vec3::new(1., 0., 0.), 0.0),
        _ => (Vec3::new(1., 0., 0.), std::f64::consts::PI),
    }
}

/// `v` turned about the unit `axis`.
pub fn rotate(v: Vec3, axis: Vec3, angle: f64) -> Vec3 {
    v * angle.cos() + axis.cross(v) * angle.sin() + axis * (axis.dot(v) * (1.0 - angle.cos()))
}

/// Directions worth trying: the axes, the largest flat regions and a spread of others.
fn candidates(tris: &[(Vec3, f64, [Vec3; 3])]) -> Vec<Vec3> {
    let mut out = Vec::new();
    for s in [-1.0, 1.0] {
        out.extend([Vec3::new(0., 0., s), Vec3::new(s, 0., 0.), Vec3::new(0., s, 0.)]);
    }
    let mut groups: std::collections::HashMap<[i32; 3], (Vec3, f64)> = Default::default();
    for (n, a, _) in tris {
        let key = [(n.x * 50.).round() as i32, (n.y * 50.).round() as i32, (n.z * 50.).round() as i32];
        let e = groups.entry(key).or_insert((Vec3::new(0., 0., 0.), 0.));
        e.0 = e.0 + *n * *a;
        e.1 += a;
    }
    let mut flat: Vec<_> = groups.into_iter().collect();
    flat.sort_by(|a, b| b.1.1.total_cmp(&a.1.1).then(a.0.cmp(&b.0)));
    out.extend(flat.into_iter().take(60).filter_map(|(_, (n, _))| n.normalized()));
    for x in -1..=1 {
        for y in -1..=1 {
            for z in -1..=1 {
                out.extend(Vec3::new(f64::from(x), f64::from(y), f64::from(z)).normalized());
            }
        }
    }
    let mut unique: Vec<Vec3> = Vec::new();
    for d in out {
        if !unique.iter().any(|u| u.dot(d) > 1.0 - 1e-6) {
            unique.push(d);
        }
    }
    unique
}

/// The orientations worth printing in, best first.
pub fn orientations(m: &PrintMesh, angle: f64, w: Weights) -> Vec<Orientation> {
    let tris: Vec<(Vec3, f64, [Vec3; 3])> =
        m.tris().map(|t| (normal_area(t), t)).filter(|((_, a), _)| *a > 0.0).map(|((n, a), t)| (n, a, t)).collect();
    if tris.is_empty() {
        return Vec::new();
    }
    let size = m.bounds().diagonal().max(1e-6);
    let total: f64 = tris.iter().map(|t| t.1).sum::<f64>().max(1e-9);
    let limit = angle.clamp(0.0, 90.0).to_radians().cos() + 1e-6;
    let tol = 1e-4 * size + 1e-3;
    let mut out: Vec<Orientation> = candidates(&tris)
        .into_par_iter()
        .map(|down| {
            let up = down * -1.0;
            let (lo, hi) =
                m.positions.iter().map(|p| p.dot(up)).fold((f64::INFINITY, f64::NEG_INFINITY), |(l, h), z| (l.min(z), h.max(z)));
            let (mut over, mut support, mut contact) = (0.0, 0.0, 0.0);
            for (n, a, t) in &tris {
                let facing = n.dot(down);
                if facing <= limit.min(0.9) {
                    continue;
                }
                let top = t.iter().map(|p| p.dot(up)).fold(f64::NEG_INFINITY, f64::max) - lo;
                if top <= tol {
                    contact += if facing > 0.99 { *a } else { 0.0 };
                } else if facing > limit {
                    let mid = t.iter().map(|p| p.dot(up)).sum::<f64>() / 3.0 - lo;
                    over += a;
                    support += a * facing * mid;
                }
            }
            let height = hi - lo;
            // Standing on a point or an edge is not printing.
            let standing = if contact < 1e-3 * total { 1.0 } else { 0.0 };
            let score = w.support * (support / (total * size) * 8.0 + over / total) + w.height * height / size * 0.5
                - w.contact * (contact / total).sqrt()
                + standing;
            Orientation { down: [down.x, down.y, down.z], overhang_area: over, support_volume: support, height, contact_area: contact, score }
        })
        .collect();
    out.sort_by(|a, b| a.score.total_cmp(&b.score));
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cuboid;

    #[test]
    fn a_plate_on_edge_is_laid_flat() {
        let m = cuboid(Vec3::new(0., 0., 0.), Vec3::new(2., 60., 40.));
        let best = &orientations(&m, 45.0, Weights { support: 1.0, height: 1.0, contact: 1.0 })[0];
        assert!(best.down[0].abs() > 0.999 && (best.height - 2.0).abs() < 1e-9, "{best:?}");
    }

    #[test]
    fn turning_puts_the_direction_down() {
        for d in [Vec3::new(1., 2., 3.), Vec3::new(0., 0., 1.), Vec3::new(0., 0., -1.), Vec3::new(-1., 0., 0.)] {
            let d = d.normalized().unwrap();
            let (axis, angle) = axis_angle(d);
            assert!((rotate(d, axis, angle) - Vec3::new(0., 0., -1.)).len() < 1e-9, "{d:?}");
        }
    }
}
