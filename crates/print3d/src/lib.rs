//! RyboCraft's 3D printing toolkit. Everything here works on plain triangle meshes in
//! millimetres with Z up, so it has no opinion about where a model came from; the engine's
//! `print.*` commands put it to work on bodies.

pub mod analysis;
pub mod lattice;
pub mod orient;
pub mod pack;
pub mod sdf;
pub mod slice;
pub mod supports;
pub mod tables;

use rybocore_record::Vec3;
use rybocraft_geom::prelude::*;
use rybocraft_geom::{Aabb3, Bvh};

/// Triangles of a printed part, welded. Degenerate triangles are dropped.
#[derive(Clone, Debug, Default)]
pub struct PrintMesh {
    pub positions: Vec<Vec3>,
    pub triangles: Vec<[u32; 3]>,
}

/// Vertices closer than this are one vertex.
const WELD: f64 = 1e-4;

impl PrintMesh {
    /// Weld `positions` and drop triangles that are degenerate or refer to missing vertices.
    pub fn new(positions: &[Vec3], triangles: &[[u32; 3]]) -> PrintMesh {
        let mut m = PrintMesh::default();
        m.append(positions, triangles);
        m
    }

    /// Add more triangles, welded among themselves only, so separate parts stay separate.
    pub fn append(&mut self, positions: &[Vec3], triangles: &[[u32; 3]]) {
        let mut seen = std::collections::HashMap::new();
        let mut map = Vec::with_capacity(positions.len());
        for p in positions {
            let key = [(p.x / WELD).round() as i64, (p.y / WELD).round() as i64, (p.z / WELD).round() as i64];
            let next = self.positions.len() as u32;
            let i = *seen.entry(key).or_insert_with(|| {
                self.positions.push(*p);
                next
            });
            map.push(i);
        }
        for t in triangles {
            let Some([a, b, c]) = corners(&map, t) else { continue };
            if a != b && b != c && c != a {
                self.triangles.push([a, b, c]);
            }
        }
    }

    pub fn tri(&self, i: usize) -> Option<[Vec3; 3]> {
        corners(&self.positions, self.triangles.get(i)?)
    }

    /// The triangles, as corner points.
    pub fn tris(&self) -> impl Iterator<Item = [Vec3; 3]> + '_ {
        self.triangles.iter().filter_map(|t| corners(&self.positions, t))
    }

    pub fn bounds(&self) -> Aabb3 {
        self.positions.iter().copied().collect()
    }

    pub fn bvh(&self) -> Bvh {
        let boxes: Vec<Aabb3> =
            self.triangles.iter().map(|t| corners(&self.positions, t).map_or(Aabb3::EMPTY, |c| c.into_iter().collect())).collect();
        Bvh::new(&boxes)
    }

    /// The nearest triangle the ray from `o` along unit `d` crosses, farther than `from`.
    pub fn raycast(&self, bvh: &Bvh, o: Vec3, d: Vec3, from: f64) -> Option<(f64, usize)> {
        bvh.nearest(o, d, |i| self.tri(i as usize).and_then(|t| ray_triangle(o, d, t)).filter(|t| *t > from)).map(|(t, i)| (t, i as usize))
    }

    pub fn volume(&self) -> f64 {
        self.tris().map(|[a, b, c]| a.dot(b.cross(c)) / 6.0).sum()
    }

    pub fn area(&self) -> f64 {
        self.tris().map(|t| normal_area(t).1).sum()
    }
}

pub(crate) fn corners<T: Copy>(list: &[T], t: &[u32; 3]) -> Option<[T; 3]> {
    Some([*list.get(t[0] as usize)?, *list.get(t[1] as usize)?, *list.get(t[2] as usize)?])
}

/// Unit normal (zero for a sliver) and area.
pub fn normal_area([a, b, c]: [Vec3; 3]) -> (Vec3, f64) {
    let n = (b - a).cross(c - a);
    let l = n.len();
    if l > 1e-14 { (n * (1.0 / l), l * 0.5) } else { (Vec3::new(0.0, 0.0, 0.0), 0.0) }
}

fn ray_triangle(o: Vec3, d: Vec3, [a, b, c]: [Vec3; 3]) -> Option<f64> {
    let (e1, e2) = (b - a, c - a);
    let p = d.cross(e2);
    let det = e1.dot(p);
    if det.abs() < 1e-14 {
        return None;
    }
    let s = o - a;
    let u = s.dot(p) / det;
    let q = s.cross(e1);
    let v = d.dot(q) / det;
    if u < -1e-9 || v < -1e-9 || u + v > 1.0 + 1e-9 {
        return None;
    }
    let t = e2.dot(q) / det;
    (t.is_finite() && t >= 0.0).then_some(t)
}

/// An axis-aligned box, for tests.
#[cfg(test)]
pub(crate) fn cuboid(lo: Vec3, hi: Vec3) -> PrintMesh {
    let p: Vec<Vec3> = (0..8)
        .map(|i| Vec3::new(if i & 1 == 0 { lo.x } else { hi.x }, if i & 2 == 0 { lo.y } else { hi.y }, if i & 4 == 0 { lo.z } else { hi.z }))
        .collect();
    let q = [[0, 2, 3, 1], [4, 5, 7, 6], [0, 1, 5, 4], [2, 6, 7, 3], [0, 4, 6, 2], [1, 3, 7, 5]];
    let t: Vec<[u32; 3]> = q.iter().flat_map(|f| [[f[0], f[1], f[2]], [f[0], f[2], f[3]]]).collect();
    PrintMesh::new(&p, &t)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_box_measures_itself() {
        let m = cuboid(Vec3::new(0.0, 0.0, 0.0), Vec3::new(2.0, 3.0, 4.0));
        assert!((m.volume() - 24.0).abs() < 1e-9 && (m.area() - 52.0).abs() < 1e-9);
        let (t, _) = m.raycast(&m.bvh(), Vec3::new(1.0, 1.0, 10.0), Vec3::new(0.0, 0.0, -1.0), 0.0).unwrap();
        assert!((t - 6.0).abs() < 1e-9);
    }
}
