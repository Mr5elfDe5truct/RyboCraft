//! What would go wrong on the printer: topology, overhangs, thin walls, stability.

use crate::{PrintMesh, normal_area};
use rayon::prelude::*;
use serde::Serialize;
use rybocore_record::Vec3;
use rybocraft_geom::prelude::*;
use std::collections::HashMap;

/// How near the bed a triangle must be to count as lying on it.
pub const ON_BED: f64 = 0.02;

#[derive(Clone, Debug, Default, Serialize, PartialEq)]
pub struct Topology {
    pub triangles: usize,
    pub shells: usize,
    /// Edges with one triangle (holes in the surface).
    pub open_edges: usize,
    /// Edges shared by more than two triangles.
    pub non_manifold_edges: usize,
    /// Edges both triangles run along in the same direction (flipped normals).
    pub flipped_edges: usize,
    pub volume: f64,
    pub area: f64,
}

impl Topology {
    pub fn watertight(&self) -> bool {
        self.triangles > 0 && self.open_edges == 0 && self.non_manifold_edges == 0 && self.flipped_edges == 0
    }
}

fn find(root: &mut [u32], mut i: u32) -> u32 {
    while let Some(&p) = root.get(i as usize) {
        if p == i {
            break;
        }
        let up = root.get(p as usize).copied().unwrap_or(p);
        if let Some(r) = root.get_mut(i as usize) {
            *r = up;
        }
        i = up;
    }
    i
}

pub fn topology(m: &PrintMesh) -> Topology {
    let mut edges: HashMap<(u32, u32), (u32, i32)> = HashMap::new();
    let mut root: Vec<u32> = (0..m.positions.len() as u32).collect();
    for t in &m.triangles {
        for (a, b) in [(t[0], t[1]), (t[1], t[2]), (t[2], t[0])] {
            let e = edges.entry((a.min(b), a.max(b))).or_default();
            e.0 += 1;
            e.1 += if a < b { 1 } else { -1 };
            let (ra, rb) = (find(&mut root, a), find(&mut root, b));
            if let Some(r) = root.get_mut(ra as usize) {
                *r = rb;
            }
        }
    }
    let used: std::collections::HashSet<u32> = m.triangles.iter().map(|t| t[0]).collect();
    let shells: std::collections::HashSet<u32> = used.into_iter().map(|v| find(&mut root, v)).collect();
    Topology {
        triangles: m.triangles.len(),
        shells: shells.len(),
        open_edges: edges.values().filter(|e| e.0 == 1).count(),
        non_manifold_edges: edges.values().filter(|e| e.0 > 2).count(),
        flipped_edges: edges.values().filter(|e| e.0 == 2 && e.1 != 0).count(),
        volume: m.volume(),
        area: m.area(),
    }
}

/// The mesh's connected pieces, each on its own.
pub fn shells(m: &PrintMesh) -> Vec<PrintMesh> {
    let mut root: Vec<u32> = (0..m.positions.len() as u32).collect();
    for t in &m.triangles {
        for b in [t[1], t[2]] {
            let (ra, rb) = (find(&mut root, t[0]), find(&mut root, b));
            if let Some(r) = root.get_mut(ra as usize) {
                *r = rb;
            }
        }
    }
    let mut index: HashMap<u32, usize> = HashMap::new();
    let mut map: HashMap<u32, u32> = HashMap::new();
    let mut out: Vec<PrintMesh> = Vec::new();
    for t in &m.triangles {
        let r = find(&mut root, t[0]);
        let k = *index.entry(r).or_insert_with(|| {
            out.push(PrintMesh::default());
            out.len() - 1
        });
        let Some(part) = out.get_mut(k) else { continue };
        let tri = t.map(|i| {
            *map.entry(i).or_insert_with(|| {
                part.positions.extend(m.positions.get(i as usize));
                part.positions.len() as u32 - 1
            })
        });
        part.triangles.push(tri);
    }
    out
}

/// Triangles that need support: facing down at less than `angle` degrees from horizontal, and not
/// lying on the bed at `bed`. The index of each, and their total area.
pub fn overhangs(m: &PrintMesh, angle: f64, bed: f64) -> (Vec<usize>, f64) {
    let limit = angle.clamp(0.0, 90.0).to_radians().cos();
    let mut out = Vec::new();
    let mut area = 0.0;
    for (i, t) in m.tris().enumerate() {
        let (n, a) = normal_area(t);
        if a > 0.0 && -n.z > limit + 1e-6 && t.iter().any(|p| p.z > bed + ON_BED) {
            out.push(i);
            area += a;
        }
    }
    (out, area)
}

/// Area lying on the bed.
pub fn contact_area(m: &PrintMesh, bed: f64) -> f64 {
    m.tris().filter(|t| t.iter().all(|p| p.z <= bed + ON_BED)).map(normal_area).filter(|(n, _)| n.z < -0.9).map(|(_, a)| a).sum()
}

#[derive(Clone, Debug, Default)]
pub struct Thickness {
    /// Thinnest wall found.
    pub min: f64,
    pub at: Option<Vec3>,
    /// Triangles whose wall is thinner than asked.
    pub thin: Vec<usize>,
    pub thin_area: f64,
}

const MAX_THICKNESS_RAYS: usize = 200_000;

/// Wall thickness by a ray straight into the material from points of every triangle.
pub fn thickness(m: &PrintMesh, limit: f64) -> Thickness {
    let bvh = m.bvh();
    let n = m.triangles.len();
    let step = n.div_ceil(MAX_THICKNESS_RAYS).max(1);
    let found: Vec<(usize, f64, Vec3, f64)> = (0..n)
        .into_par_iter()
        .step_by(step)
        .filter_map(|i| {
            let t = m.tri(i)?;
            let (normal, area) = normal_area(t);
            if area < 1e-9 {
                return None;
            }
            // The centroid and three points part-way to the corners, so a big triangle over a thin
            // region is not judged by its middle alone.
            let centre = (t[0] + t[1] + t[2]) * (1.0 / 3.0);
            let samples = [centre, centre.lerp(t[0], 0.6), centre.lerp(t[1], 0.6), centre.lerp(t[2], 0.6)];
            samples
                .into_iter()
                .filter_map(|p| {
                    let o = p - normal * 1e-4;
                    let (d, hit) = m.raycast(&bvh, o, normal * -1.0, 1e-5)?;
                    // A ray that leaves through a face square-on measures a wall; one that grazes
                    // out near an edge measures the edge.
                    let (other, _) = normal_area(m.tri(hit)?);
                    (other.dot(normal) < -0.3).then_some((d + 1e-4, p))
                })
                .min_by(|a, b| a.0.total_cmp(&b.0))
                .map(|(d, p)| (i, d, p, area))
        })
        .collect();
    let mut out = Thickness { min: f64::INFINITY, ..Thickness::default() };
    for (i, d, p, area) in found {
        if d < out.min {
            out.min = d;
            out.at = Some(p);
        }
        if d < limit - 1e-4 {
            out.thin.push(i);
            out.thin_area += area;
        }
    }
    if !out.min.is_finite() {
        out.min = 0.0;
    }
    out
}

/// Volume centroid (the box centre when the mesh has no volume).
pub fn centroid(m: &PrintMesh) -> Vec3 {
    let mut v = 0.0;
    let mut c = Vec3::new(0.0, 0.0, 0.0);
    for [a, b, d] in m.tris() {
        let w = a.dot(b.cross(d)) / 6.0;
        v += w;
        c = c + (a + b + d) * (w * 0.25);
    }
    if v.abs() > 1e-9 { c * (1.0 / v) } else { m.bounds().center() }
}

/// How far (degrees) the part can lean before its centre of mass leaves its footprint. Negative
/// when it is already outside.
pub fn tip_angle(m: &PrintMesh, bed: f64) -> f64 {
    let c = centroid(m);
    let hull = convex_hull(m.positions.iter().filter(|p| p.z <= bed + ON_BED).map(|p| [p.x, p.y]).collect());
    if hull.len() < 3 {
        return 0.0;
    }
    let mut near = f64::INFINITY;
    for (a, b) in hull.iter().zip(hull.iter().cycle().skip(1)) {
        let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
        let l = dx.hypot(dy);
        if l > 1e-9 {
            near = near.min((dx * (c.y - a[1]) - dy * (c.x - a[0])) / l);
        }
    }
    near.atan2((c.z - bed).max(1e-6)).to_degrees()
}

/// Counter-clockwise convex hull.
pub fn convex_hull(mut p: Vec<[f64; 2]>) -> Vec<[f64; 2]> {
    p.sort_by(|a, b| a[0].total_cmp(&b[0]).then(a[1].total_cmp(&b[1])));
    p.dedup();
    if p.len() < 3 {
        return p;
    }
    let cross = |o: [f64; 2], a: [f64; 2], b: [f64; 2]| (a[0] - o[0]) * (b[1] - o[1]) - (a[1] - o[1]) * (b[0] - o[0]);
    let half = |points: &[[f64; 2]]| {
        let mut h: Vec<[f64; 2]> = Vec::new();
        for q in points {
            while let [.., a, b] = h.as_slice() {
                if cross(*a, *b, *q) <= 1e-12 {
                    h.pop();
                } else {
                    break;
                }
            }
            h.push(*q);
        }
        h.pop();
        h
    };
    let mut out = half(&p);
    p.reverse();
    out.extend(half(&p));
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cuboid;

    fn v(x: f64, y: f64, z: f64) -> Vec3 {
        Vec3::new(x, y, z)
    }

    #[test]
    fn a_box_is_watertight_and_a_box_missing_a_triangle_is_not() {
        let mut m = cuboid(v(0., 0., 0.), v(1., 1., 1.));
        let t = topology(&m);
        assert!(t.watertight() && t.shells == 1);
        m.triangles.pop();
        assert_eq!(topology(&m).open_edges, 3);
    }

    #[test]
    fn only_the_raised_underside_overhangs() {
        let mut m = cuboid(v(0., 0., 0.), v(10., 10., 10.));
        assert!(overhangs(&m, 45.0, 0.0).0.is_empty());
        let top = cuboid(v(-5., 0., 10.), v(15., 10., 12.));
        m.append(&top.positions, &top.triangles);
        assert!((overhangs(&m, 45.0, 0.0).1 - 200.0).abs() < 1e-6);
        assert!((contact_area(&m, 0.0) - 100.0).abs() < 1e-6);
    }

    #[test]
    fn a_thin_plate_is_thin() {
        let m = cuboid(v(0., 0., 0.), v(20., 20., 0.4));
        let t = thickness(&m, 0.8);
        assert!((t.min - 0.4).abs() < 1e-3 && t.thin.len() == 4, "{t:?}");
        assert!(thickness(&m, 0.3).thin.is_empty());
    }

    #[test]
    fn a_cube_tips_at_45_degrees() {
        let m = cuboid(v(0., 0., 0.), v(10., 10., 10.));
        assert!((tip_angle(&m, 0.0) - 45.0).abs() < 1e-6);
    }
}
