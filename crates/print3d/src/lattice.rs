//! Lattices: periodic minimal surfaces clipped to a part, as a closed mesh.

use crate::PrintMesh;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use rybocore_record::Vec3;
use rybocraft_geom::prelude::*;
use rybocraft_geom::{Bvh, Mesh};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Pattern {
    Gyroid,
    Diamond,
    Schwarz,
    Cubic,
}

impl Pattern {
    /// The surface's field at phase `(x, y, z)`, scaled so its gradient is about 1.
    fn field(self, x: f64, y: f64, z: f64) -> f64 {
        let (sx, cx) = x.sin_cos();
        let (sy, cy) = y.sin_cos();
        let (sz, cz) = z.sin_cos();
        match self {
            Pattern::Gyroid => (sx * cy + sy * cz + sz * cx) / 1.4,
            Pattern::Schwarz => (cx + cy + cz) / 1.3,
            Pattern::Diamond => (sx * sy * sz + sx * cy * cz + cx * sy * cz + cx * cy * sz) / 1.2,
            // Struts along the axes: the distance to the nearest of three families of lines.
            Pattern::Cubic => {
                let q = [(x * 0.5).sin().abs() * 2.0, (y * 0.5).sin().abs() * 2.0, (z * 0.5).sin().abs() * 2.0];
                q[0].hypot(q[1]).min(q[1].hypot(q[2])).min(q[2].hypot(q[0]))
            }
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct LatticeSettings {
    pub pattern: Pattern,
    pub cell: f64,
    pub wall: f64,
    /// Solid skin kept around the part; 0 for an open lattice.
    pub skin: f64,
    pub resolution: f64,
}

const MAX_CELLS: usize = 6_000_000;

/// `m` with its inside replaced by the lattice. An error when the grid would be too large.
pub fn lattice(m: &PrintMesh, s: &LatticeSettings) -> Result<PrintMesh, String> {
    let b = m.bounds();
    if b.is_empty() {
        return Err("there is nothing to fill".into());
    }
    let size = b.size();
    let h = s.resolution.max(0.05);
    let n = [size.x, size.y, size.z].map(|l| (l / h).ceil() as usize + 3);
    let total = n[0].saturating_mul(n[1]).saturating_mul(n[2]);
    if total > MAX_CELLS {
        let want = h * (total as f64 / MAX_CELLS as f64).cbrt();
        return Err(format!("the lattice grid would be too large: use a resolution of {:.2} mm or more", want + 0.01));
    }
    let o = b.min - Vec3::new(h, h, h);
    let source = Mesh { positions: m.positions.clone(), triangles: m.triangles.clone(), ..Mesh::default() };
    let inside = source.inside_index();
    let bvh = m.bvh();
    let k = std::f64::consts::TAU / s.cell.max(0.5);
    let reach = (s.skin + 2.0 * h).max(3.0 * h);
    // Negative inside material.
    let values: Vec<f32> = (0..n[2])
        .into_par_iter()
        .flat_map_iter(|kz| {
            let (inside, bvh) = (&inside, &bvh);
            (0..n[1]).flat_map(move |j| {
                (0..n[0]).map(move |i| {
                    let p = o + Vec3::new(i as f64, j as f64, kz as f64) * h;
                    let d = distance(m, bvh, p, reach);
                    let d = if inside.contains(p) { -d } else { d };
                    let f = s.pattern.field(p.x * k, p.y * k, p.z * k);
                    let core = f.abs() / k - s.wall * 0.5;
                    let core = if s.skin > 0.0 { core.min(-(d + s.skin)) } else { core };
                    // Never exactly zero, so no vertex lands on a grid point twice.
                    let v = d.max(core);
                    (if v.abs() < 1e-5 { 1e-5 } else { v }) as f32
                })
            })
        })
        .collect();
    Ok(tetrahedra(&values, n, o, h))
}

/// Distance to the surface, no more than `reach`.
pub(crate) fn distance(m: &PrintMesh, bvh: &Bvh, p: Vec3, reach: f64) -> f64 {
    let best = std::cell::Cell::new(reach);
    bvh.visit(
        |b| {
            let q = Vec3::new(p.x.clamp(b.min.x, b.max.x), p.y.clamp(b.min.y, b.max.y), p.z.clamp(b.min.z, b.max.z));
            q.dist(p) < best.get()
        },
        |i| {
            if let Some(t) = m.tri(i as usize) {
                best.set(best.get().min(closest(t, p).dist(p)));
            }
        },
    );
    best.get()
}

/// The point of a triangle nearest `p`.
fn closest([a, b, c]: [Vec3; 3], p: Vec3) -> Vec3 {
    let (ab, ac, ap) = (b - a, c - a, p - a);
    let (d1, d2) = (ab.dot(ap), ac.dot(ap));
    if d1 <= 0.0 && d2 <= 0.0 {
        return a;
    }
    let bp = p - b;
    let (d3, d4) = (ab.dot(bp), ac.dot(bp));
    if d3 >= 0.0 && d4 <= d3 {
        return b;
    }
    let vc = d1 * d4 - d3 * d2;
    if vc <= 0.0 && d1 >= 0.0 && d3 <= 0.0 {
        return a + ab * (d1 / (d1 - d3));
    }
    let cp = p - c;
    let (d5, d6) = (ab.dot(cp), ac.dot(cp));
    if d6 >= 0.0 && d5 <= d6 {
        return c;
    }
    let vb = d5 * d2 - d1 * d6;
    if vb <= 0.0 && d2 >= 0.0 && d6 <= 0.0 {
        return a + ac * (d2 / (d2 - d6));
    }
    let va = d3 * d6 - d5 * d4;
    if va <= 0.0 && d4 - d3 >= 0.0 && d5 - d6 >= 0.0 {
        return b + (c - b) * ((d4 - d3) / ((d4 - d3) + (d5 - d6)));
    }
    let den = 1.0 / (va + vb + vc);
    a + ab * (vb * den) + ac * (vc * den)
}

/// The zero surface by marching tetrahedra: closed and manifold wherever the grid's border is
/// outside.
pub(crate) fn tetrahedra(v: &[f32], n: [usize; 3], o: Vec3, h: f64) -> PrintMesh {
    // Six tetrahedra around the cube's main diagonal, the same in every cube.
    const TETS: [[usize; 4]; 6] = [[0, 1, 3, 7], [0, 3, 2, 7], [0, 2, 6, 7], [0, 6, 4, 7], [0, 4, 5, 7], [0, 5, 1, 7]];
    let id = |i: usize, j: usize, k: usize| k * n[0] * n[1] + j * n[0] + i;
    let at = |g: usize| f64::from(v.get(g).copied().unwrap_or(1.0));
    let point = |g: usize| o + Vec3::new((g % n[0]) as f64, (g / n[0] % n[1]) as f64, (g / (n[0] * n[1])) as f64) * h;
    let mean = |l: &[usize]| l.iter().map(|g| point(*g)).fold(Vec3::new(0., 0., 0.), |s, p| s + p) * (1.0 / l.len().max(1) as f64);
    let mut out = PrintMesh::default();
    let mut made: std::collections::HashMap<(usize, usize), u32> = Default::default();
    for k in 0..n[2] - 1 {
        for j in 0..n[1] - 1 {
            for i in 0..n[0] - 1 {
                let corner: [usize; 8] = std::array::from_fn(|c| id(i + (c & 1), j + (c >> 1 & 1), k + (c >> 2 & 1)));
                let neg = corner.iter().filter(|g| at(**g) < 0.0).count();
                if neg == 0 || neg == 8 {
                    continue;
                }
                for tet in TETS {
                    let g = tet.map(|c| corner[c]);
                    let (inside, outside): (Vec<usize>, Vec<usize>) = g.iter().partition(|g| at(**g) < 0.0);
                    let mut vertex = |a: usize, b: usize| {
                        let (a, b) = (a.min(b), a.max(b));
                        *made.entry((a, b)).or_insert_with(|| {
                            let t = (at(a) / (at(a) - at(b))).clamp(0.02, 0.98);
                            out.positions.push(point(a).lerp(point(b), t));
                            out.positions.len() as u32 - 1
                        })
                    };
                    let faces: Vec<[u32; 3]> = match (inside.as_slice(), outside.as_slice()) {
                        ([a], [x, y, z]) => vec![[vertex(*a, *x), vertex(*a, *y), vertex(*a, *z)]],
                        ([a, b, c], [x]) => vec![[vertex(*a, *x), vertex(*b, *x), vertex(*c, *x)]],
                        ([a, b], [x, y]) => {
                            let q = [vertex(*a, *x), vertex(*a, *y), vertex(*b, *y), vertex(*b, *x)];
                            vec![[q[0], q[1], q[2]], [q[0], q[2], q[3]]]
                        }
                        _ => continue,
                    };
                    // Outwards: from the inside corners towards the outside ones.
                    let way = mean(&outside) - mean(&inside);
                    for mut f in faces {
                        if let Some(t) = crate::corners(&out.positions, &f)
                            && (t[1] - t[0]).cross(t[2] - t[0]).dot(way) < 0.0
                        {
                            f.swap(1, 2);
                        }
                        out.triangles.push(f);
                    }
                }
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cuboid;

    #[test]
    fn every_pattern_is_watertight_and_lighter_than_the_part() {
        let m = cuboid(Vec3::new(0., 0., 0.), Vec3::new(16., 16., 16.));
        for pattern in [Pattern::Gyroid, Pattern::Diamond, Pattern::Schwarz, Pattern::Cubic] {
            for skin in [0.0, 1.0] {
                let l = lattice(&m, &LatticeSettings { pattern, cell: 8.0, wall: 1.2, skin, resolution: 0.5 }).unwrap();
                let t = crate::analysis::topology(&l);
                assert!(t.watertight(), "{pattern:?} {skin}: {t:?}");
                assert!(t.volume > 80.0 && t.volume < 4096.0 * 0.85, "{pattern:?} {skin}: {t:?}");
            }
        }
    }

    #[test]
    fn a_grid_too_large_is_refused() {
        let m = cuboid(Vec3::new(0., 0., 0.), Vec3::new(500., 500., 500.));
        assert!(lattice(&m, &LatticeSettings { pattern: Pattern::Gyroid, cell: 8.0, wall: 1.2, skin: 0.0, resolution: 0.2 }).is_err());
    }
}
