//! Support columns: breakaway posts with a thin tip under every overhang.

use crate::PrintMesh;
use rybocore_record::Vec3;

#[derive(Clone, Copy, Debug)]
pub struct SupportSettings {
    /// Overhangs flatter than this (degrees from horizontal) are supported.
    pub angle: f64,
    pub spacing: f64,
    pub diameter: f64,
    pub tip: f64,
    /// Left between a post and the part, above and below.
    pub gap: f64,
    /// Extra radius at the bed.
    pub foot: f64,
    /// Only posts that reach the bed.
    pub bed_only: bool,
}

const MAX_POSTS: usize = 4000;
const SIDES: u32 = 8;

/// The posts, one closed shell each, and how many there are. An error when there would be too many.
pub fn columns(m: &PrintMesh, s: &SupportSettings) -> Result<(PrintMesh, usize), String> {
    let b = m.bounds();
    if b.is_empty() {
        return Ok((PrintMesh::default(), 0));
    }
    let bed = b.min.z;
    let bvh = m.bvh();
    let (over, _) = crate::analysis::overhangs(m, s.angle, bed);
    let step = s.spacing.max(0.5);
    let mut posts: std::collections::BTreeMap<(i64, i64, i64), [f64; 4]> = Default::default();
    for i in over {
        let Some(t) = m.tri(i) else { continue };
        let span = |f: fn(&Vec3) -> f64, from: f64| {
            let lo = t.iter().map(f).fold(f64::INFINITY, f64::min);
            let hi = t.iter().map(f).fold(f64::NEG_INFINITY, f64::max);
            ((lo - from) / step - 0.5).ceil() as i64..=((hi - from) / step - 0.5).floor() as i64
        };
        for gx in span(|p| p.x, b.min.x) {
            for gy in span(|p| p.y, b.min.y) {
                let (x, y) = (b.min.x + (gx as f64 + 0.5) * step, b.min.y + (gy as f64 + 0.5) * step);
                let Some(z) = height_at(t, x, y) else { continue };
                let o = Vec3::new(x, y, z - 1e-3);
                let below = m.raycast(&bvh, o, Vec3::new(0., 0., -1.), 1e-4).map(|(d, _)| o.z - d);
                if s.bed_only && below.is_some() {
                    continue;
                }
                let from = below.map_or(bed, |z| z + s.gap);
                let to = z - s.gap;
                if to - from < 0.4 {
                    continue;
                }
                posts.insert((gx, gy, (z / 0.2).round() as i64), [x, y, from, to]);
                if posts.len() > MAX_POSTS {
                    return Err(format!("more than {MAX_POSTS} support posts are needed: raise the spacing"));
                }
            }
        }
    }
    let mut out = PrintMesh::default();
    for [x, y, from, to] in posts.values().copied() {
        let on_bed = (from - bed).abs() < 1e-9;
        let h = to - from;
        let (r, tip) = (s.diameter * 0.5, s.tip * 0.5);
        let taper = (h * 0.4).min(s.diameter * 1.5);
        let mut rings = Vec::new();
        if on_bed && h > 1.5 && s.foot > 0.0 {
            rings.extend([(from, r + s.foot), (from + 0.4, r + s.foot), (from + 0.9, r)]);
        } else {
            // Resting on the part: thinner there too, so it breaks off.
            rings.push((from, if on_bed { r } else { tip.max(r * 0.5) }));
            if !on_bed && h > 1.5 {
                rings.push((from + taper * 0.5, r));
            }
        }
        if rings.last().is_some_and(|l| l.0 < to - taper - 1e-3) {
            rings.push((to - taper, r));
        }
        rings.push((to, tip));
        post(&mut out, x, y, &rings);
    }
    Ok((out, posts.len()))
}

fn height_at(t: [Vec3; 3], x: f64, y: f64) -> Option<f64> {
    let [a, b, c] = t;
    let d = (b.y - c.y) * (a.x - c.x) + (c.x - b.x) * (a.y - c.y);
    if d.abs() < 1e-12 {
        return None;
    }
    let u = ((b.y - c.y) * (x - c.x) + (c.x - b.x) * (y - c.y)) / d;
    let v = ((c.y - a.y) * (x - c.x) + (a.x - c.x) * (y - c.y)) / d;
    let w = 1.0 - u - v;
    (u >= -1e-9 && v >= -1e-9 && w >= -1e-9).then_some(u * a.z + v * b.z + w * c.z)
}

/// A stack of rings (height, radius) closed at both ends.
fn post(out: &mut PrintMesh, x: f64, y: f64, rings: &[(f64, f64)]) {
    let mut p = Vec::new();
    let mut t = Vec::new();
    for (z, r) in rings {
        for k in 0..SIDES {
            let a = (f64::from(k) + 0.5) / f64::from(SIDES) * std::f64::consts::TAU;
            p.push(Vec3::new(x + r * a.cos(), y + r * a.sin(), *z));
        }
    }
    let top = (rings.len() as u32 - 1) * SIDES;
    for j in 0..rings.len() as u32 - 1 {
        for k in 0..SIDES {
            let (a, b) = (j * SIDES + k, j * SIDES + (k + 1) % SIDES);
            t.extend([[a, b, b + SIDES], [a, b + SIDES, a + SIDES]]);
        }
    }
    for k in 1..SIDES - 1 {
        t.extend([[0, k + 1, k], [top, top + k, top + k + 1]]);
    }
    out.append(&p, &t);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cuboid;

    #[test]
    fn a_table_gets_posts_and_a_cube_none() {
        let s = SupportSettings { angle: 45., spacing: 4., diameter: 1.6, tip: 0.6, gap: 0.2, foot: 1.0, bed_only: false };
        let mut m = cuboid(Vec3::new(0., 0., 0.), Vec3::new(10., 10., 20.));
        assert_eq!(columns(&m, &s).unwrap().1, 0);
        let top = cuboid(Vec3::new(-20., -20., 20.), Vec3::new(30., 30., 23.));
        m.append(&top.positions, &top.triangles);
        let (posts, n) = columns(&m, &s).unwrap();
        assert!(n > 80, "{n}");
        let t = crate::analysis::topology(&posts);
        assert!(t.watertight() && t.shells == n && t.volume > 0.0, "{t:?}");
        let b = posts.bounds();
        assert!(b.min.z.abs() < 1e-9 && (b.max.z - 19.8).abs() < 1e-9);
    }
}
