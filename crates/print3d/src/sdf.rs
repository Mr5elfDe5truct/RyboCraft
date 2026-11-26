//! Shapes as signed distances (negative inside), meshed watertight. This is what lets imported
//! STL models be cut, textured, jointed and moulded, where exact solids cannot be had.

use crate::PrintMesh;
use rayon::prelude::*;
use rybocore_record::Vec3;
use rybocraft_geom::prelude::*;
use rybocraft_geom::{Aabb3, Bvh, InsideIndex, Mesh};

const MAX_CELLS: usize = 16_000_000;

/// Mesh `f` over the box `b` (a margin is added), with cells of `h`.
pub fn polygonize(b: Aabb3, h: f64, f: impl Fn(Vec3) -> f64 + Sync) -> Result<PrintMesh, String> {
    if b.is_empty() {
        return Err("there is nothing to make".into());
    }
    let h = h.max(0.02);
    let size = b.size();
    let n = [size.x, size.y, size.z].map(|l| (l / h).ceil() as usize + 4);
    let total = n[0].saturating_mul(n[1]).saturating_mul(n[2]);
    if total > MAX_CELLS {
        let want = h * (total as f64 / MAX_CELLS as f64).cbrt() + 0.01;
        return Err(format!("too fine for this size: use a detail of {want:.2} mm or more"));
    }
    let o = b.min - Vec3::new(h, h, h) * 1.371;
    let edge = |i: usize, n: usize| i == 0 || i + 1 == n;
    let values: Vec<f32> = (0..n[2])
        .into_par_iter()
        .flat_map_iter(|k| {
            let f = &f;
            (0..n[1]).flat_map(move |j| {
                (0..n[0]).map(move |i| {
                    // The border is always outside, so the result is always closed.
                    if edge(i, n[0]) || edge(j, n[1]) || edge(k, n[2]) {
                        return h as f32;
                    }
                    let v = f(o + Vec3::new(i as f64, j as f64, k as f64) * h);
                    (if v.abs() < 1e-5 { 1e-5 } else { v }) as f32
                })
            })
        })
        .collect();
    let m = crate::lattice::tetrahedra(&values, n, o, h);
    if m.triangles.is_empty() { Err("nothing is left of the shape".into()) } else { Ok(m) }
}

/// What a [`Field`] looks things up in.
pub struct Source {
    pub mesh: PrintMesh,
    solid: Mesh,
    bvh: Bvh,
}

impl Source {
    pub fn new(mesh: PrintMesh) -> Source {
        let solid = Mesh { positions: mesh.positions.clone(), triangles: mesh.triangles.clone(), ..Mesh::default() };
        let bvh = mesh.bvh();
        Source { mesh, solid, bvh }
    }
    pub fn field(&self) -> Field<'_> {
        Field { s: self, inside: self.solid.inside_index(), bounds: self.mesh.bounds() }
    }
}

/// The signed distance to a closed mesh.
pub struct Field<'a> {
    s: &'a Source,
    inside: InsideIndex<'a>,
    pub bounds: Aabb3,
}

impl Field<'_> {
    /// Exact within `reach` of the surface; at most that beyond.
    pub fn at(&self, p: Vec3, reach: f64) -> f64 {
        let b = &self.bounds;
        let q = Vec3::new(p.x.clamp(b.min.x, b.max.x), p.y.clamp(b.min.y, b.max.y), p.z.clamp(b.min.z, b.max.z));
        if q.dist(p) > reach {
            return reach;
        }
        let d = crate::lattice::distance(&self.s.mesh, &self.s.bvh, p, reach);
        if self.inside.contains(p) { -d } else { d }
    }
}

// ---- shapes ------------------------------------------------------------------------------------

pub fn sphere(p: Vec3, c: Vec3, r: f64) -> f64 {
    p.dist(c) - r
}

/// A round-ended rod.
pub fn capsule(p: Vec3, a: Vec3, b: Vec3, r: f64) -> f64 {
    let d = b - a;
    let t = ((p - a).dot(d) / d.len2().max(1e-12)).clamp(0.0, 1.0);
    p.dist(a + d * t) - r
}

/// A cylinder from `base` along unit `axis`.
pub fn cylinder(p: Vec3, base: Vec3, axis: Vec3, r: f64, h: f64) -> f64 {
    let v = p - base;
    let z = v.dot(axis);
    let radial = (v - axis * z).len() - r;
    let along = (z - h).max(-z);
    if radial > 0.0 && along > 0.0 { radial.hypot(along) } else { radial.max(along) }
}

/// A box with rounded edges.
pub fn cuboid(p: Vec3, lo: Vec3, hi: Vec3, r: f64) -> f64 {
    let c = (lo + hi) * 0.5;
    let half = (hi - lo) * 0.5;
    let q = [(p.x - c.x).abs() - half.x + r, (p.y - c.y).abs() - half.y + r, (p.z - c.z).abs() - half.z + r];
    let out = (q[0].max(0.0).powi(2) + q[1].max(0.0).powi(2) + q[2].max(0.0).powi(2)).sqrt();
    out + q[0].max(q[1]).max(q[2]).min(0.0) - r
}

pub fn torus(p: Vec3, c: Vec3, axis: Vec3, major: f64, minor: f64) -> f64 {
    let v = p - c;
    let z = v.dot(axis);
    ((v - axis * z).len() - major).hypot(z) - minor
}

/// The signed distance to a polygon in the plane.
pub fn polygon(p: [f64; 2], pts: &[[f64; 2]]) -> f64 {
    let mut d = f64::INFINITY;
    let mut inside = false;
    for (a, b) in pts.iter().zip(pts.iter().cycle().skip(1)) {
        let (e, w) = ([b[0] - a[0], b[1] - a[1]], [p[0] - a[0], p[1] - a[1]]);
        let t = ((w[0] * e[0] + w[1] * e[1]) / (e[0] * e[0] + e[1] * e[1]).max(1e-12)).clamp(0.0, 1.0);
        d = d.min((w[0] - e[0] * t).hypot(w[1] - e[1] * t));
        if (a[1] > p[1]) != (b[1] > p[1]) && p[0] < a[0] + (b[0] - a[0]) * (p[1] - a[1]) / (b[1] - a[1]) {
            inside = !inside;
        }
    }
    if inside { -d } else { d }
}

/// The surface patterns models can be textured with, from -1 to 1 at phase `p`.
pub fn pattern(kind: &str, p: Vec3) -> f64 {
    let (x, y, z) = (p.x, p.y, p.z);
    match kind {
        "waves" => (z + (x + y).sin() * 0.5).sin(),
        "knurl" => ((x + y + z).sin() * (x + y - z).sin()).clamp(-1.0, 1.0),
        "bumps" => (x.sin() * y.sin() * z.sin() * 2.0).clamp(-1.0, 1.0),
        "gyroid" => (x.sin() * y.cos() + y.sin() * z.cos() + z.sin() * x.cos()) / 1.5,
        "layers" => (z.sin() * 3.0).clamp(-1.0, 1.0),
        "scales" => {
            let row = (z / std::f64::consts::PI).floor();
            let u = x + y + row * std::f64::consts::FRAC_PI_2;
            let v = z / std::f64::consts::PI - row;
            (1.0 - v - u.sin().abs() * 0.6) * 1.2
        }
        _ => {
            // Rock: a few octaves of value noise.
            let (mut sum, mut amp, mut q) = (0.0, 0.55, p * 0.5);
            for _ in 0..3 {
                sum += amp * noise(q);
                amp *= 0.5;
                q = q * 2.03;
            }
            sum
        }
    }
}

fn noise(p: Vec3) -> f64 {
    let hash = |x: i64, y: i64, z: i64| {
        let mut h = (x.wrapping_mul(374_761_393) ^ y.wrapping_mul(668_265_263) ^ z.wrapping_mul(2_147_483_647)) as u64;
        h = (h ^ (h >> 13)).wrapping_mul(1_274_126_177_361_721);
        ((h >> 20) & 0xffff) as f64 / 32767.5 - 1.0
    };
    let (i, f) = ([p.x.floor(), p.y.floor(), p.z.floor()], |t: f64| t * t * (3.0 - 2.0 * t));
    let (u, v, w) = (f(p.x - i[0]), f(p.y - i[1]), f(p.z - i[2]));
    let mut out = 0.0;
    for k in 0..8i64 {
        let (a, b, c) = (k & 1, k >> 1 & 1, k >> 2 & 1);
        let weight = (if a == 1 { u } else { 1.0 - u }) * (if b == 1 { v } else { 1.0 - v }) * (if c == 1 { w } else { 1.0 - w });
        out += weight * hash(i[0] as i64 + a, i[1] as i64 + b, i[2] as i64 + c);
    }
    out
}

// ---- height fields and rings ---------------------------------------------------------------------

/// A closed slab whose top follows `top(i, j)`, its points placed by `place(u, v, height)`; `u` and
/// `v` run over the grid in cells.
pub fn height_slab(nx: usize, ny: usize, top: impl Fn(usize, usize) -> f64, place: impl Fn(f64, f64, f64) -> Vec3) -> PrintMesh {
    let mut m = PrintMesh::default();
    let id = |i: usize, j: usize, up: bool| (j * nx + i) as u32 * 2 + u32::from(up);
    for j in 0..ny {
        for i in 0..nx {
            m.positions.push(place(i as f64, j as f64, 0.0));
            m.positions.push(place(i as f64, j as f64, top(i, j)));
        }
    }
    let mut quad = |a: u32, b: u32, c: u32, d: u32| m.triangles.extend([[a, b, c], [a, c, d]]);
    for j in 0..ny - 1 {
        for i in 0..nx - 1 {
            quad(id(i, j, true), id(i + 1, j, true), id(i + 1, j + 1, true), id(i, j + 1, true));
            quad(id(i, j, false), id(i, j + 1, false), id(i + 1, j + 1, false), id(i + 1, j, false));
        }
    }
    for i in 0..nx - 1 {
        quad(id(i, 0, false), id(i + 1, 0, false), id(i + 1, 0, true), id(i, 0, true));
        quad(id(i + 1, ny - 1, false), id(i, ny - 1, false), id(i, ny - 1, true), id(i + 1, ny - 1, true));
    }
    for j in 0..ny - 1 {
        quad(id(0, j + 1, false), id(0, j, false), id(0, j, true), id(0, j + 1, true));
        quad(id(nx - 1, j, false), id(nx - 1, j + 1, false), id(nx - 1, j + 1, true), id(nx - 1, j, true));
    }
    m
}

/// A closed profile (counter-clockwise in radius, height) turned about Z; `radius(r, z, angle)`
/// shapes it as it goes. The profile must stay off the axis.
pub fn turned(profile: &[[f64; 2]], around: usize, radius: impl Fn(usize, f64, f64) -> f64) -> PrintMesh {
    let n = profile.len();
    let mut m = PrintMesh::default();
    for k in 0..around {
        let a = std::f64::consts::TAU * k as f64 / around as f64;
        for (i, [_, z]) in profile.iter().enumerate() {
            let r = radius(i, a, *z);
            m.positions.push(Vec3::new(r * a.cos(), r * a.sin(), *z));
        }
    }
    let id = |k: usize, i: usize| ((k % around) * n + i % n) as u32;
    for k in 0..around {
        for i in 0..n {
            let (a, b, c, d) = (id(k, i), id(k + 1, i), id(k + 1, i + 1), id(k, i + 1));
            m.triangles.extend([[a, b, c], [a, c, d]]);
        }
    }
    if m.volume() < 0.0 {
        for t in &mut m.triangles {
            t.swap(1, 2);
        }
    }
    m
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analysis::topology;

    fn v(x: f64, y: f64, z: f64) -> Vec3 {
        Vec3::new(x, y, z)
    }

    #[test]
    fn shapes_mesh_watertight_at_their_volume() {
        let b: Aabb3 = [v(-6., -6., -6.), v(6., 6., 6.)].into_iter().collect();
        let t = topology(&polygonize(b, 0.4, |p| sphere(p, v(0., 0., 0.), 5.0)).unwrap());
        assert!(t.watertight() && (t.volume - 523.6).abs() < 15.0, "{t:?}");
        let t = topology(&polygonize(b, 0.4, |p| cuboid(p, v(-5., -4., -3.), v(5., 4., 3.), 0.5)).unwrap());
        assert!(t.watertight() && (t.volume - 480.0).abs() < 25.0, "{t:?}");
        // Filling the whole box still closes.
        assert!(topology(&polygonize(b, 0.5, |_| -1.0).unwrap()).watertight());
    }

    #[test]
    fn a_mesh_field_is_signed() {
        let s = Source::new(crate::cuboid(v(0., 0., 0.), v(10., 10., 10.)));
        let f = s.field();
        assert!((f.at(v(5., 5., 12.), 5.0) - 2.0).abs() < 1e-9 && (f.at(v(5., 5., 9.), 5.0) + 1.0).abs() < 1e-9);
    }

    #[test]
    fn slabs_and_turned_parts_close() {
        let m = height_slab(20, 30, |i, j| 1.0 + ((i * j) % 5) as f64 * 0.2, v);
        assert!(topology(&m).watertight() && m.volume() > 0.0);
        let m = turned(&[[5., 0.], [10., 0.], [10., 20.], [8., 20.], [8., 2.], [5., 2.]], 48, |i, a, _| [5., 10., 10., 8., 8., 5.][i] + (a * 6.0).sin() * 0.3);
        let t = topology(&m);
        assert!(t.watertight() && t.volume > 0.0, "{t:?}");
    }

    #[test]
    fn polygons_are_signed() {
        let sq = [[0., 0.], [4., 0.], [4., 4.], [0., 4.]];
        assert!((polygon([2., 2.], &sq) + 2.0).abs() < 1e-9 && (polygon([6., 2.], &sq) - 2.0).abs() < 1e-9);
    }
}
