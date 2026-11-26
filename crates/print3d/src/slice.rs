//! Layers: cross-section areas, outlines, islands, and the print estimate built from them.

use crate::PrintMesh;
use rayon::prelude::*;
use serde::Serialize;
use rybocore_record::Vec3;

/// A directed piece of a layer's outline.
pub type Segment = [[f64; 2]; 2];

/// Triangles filed by the layers they cross.
pub struct Slicer<'a> {
    mesh: &'a PrintMesh,
    pub bed: f64,
    pub top: f64,
    pub layer: f64,
    bins: Vec<Vec<u32>>,
}

/// A print with more layers than this is sliced more coarsely.
pub const MAX_LAYERS: usize = 4000;

impl<'a> Slicer<'a> {
    pub fn new(mesh: &'a PrintMesh, layer: f64) -> Slicer<'a> {
        let b = mesh.bounds();
        let (bed, top) = if b.is_empty() { (0.0, 0.0) } else { (b.min.z, b.max.z) };
        let h = (top - bed).max(0.0);
        let layer = layer.max(0.01).max(h / MAX_LAYERS as f64);
        let n = ((h / layer) - 1e-6).ceil().max(0.0) as usize;
        let mut bins = vec![Vec::new(); n];
        for (i, t) in mesh.tris().enumerate() {
            let lo = t.iter().map(|p| p.z).fold(f64::INFINITY, f64::min);
            let hi = t.iter().map(|p| p.z).fold(f64::NEG_INFINITY, f64::max);
            let a = ((lo - bed) / layer - 1.0).floor().max(0.0) as usize;
            let b = ((hi - bed) / layer + 1.0).ceil().max(0.0) as usize;
            for bin in bins.iter_mut().take(b + 1).skip(a) {
                bin.push(i as u32);
            }
        }
        Slicer { mesh, bed, top, layer, bins }
    }

    pub fn layers(&self) -> usize {
        self.bins.len()
    }

    /// The height a layer is cut at: its middle.
    pub fn z(&self, i: usize) -> f64 {
        self.bed + (i as f64 + 0.5) * self.layer
    }

    /// The outline of layer `i`.
    pub fn segments(&self, i: usize) -> Vec<Segment> {
        let z = self.z(i).min(self.top - 1e-7);
        self.bins.get(i).map_or(Vec::new(), |bin| bin.iter().filter_map(|&t| self.mesh.tri(t as usize).and_then(|t| cut(t, z))).collect())
    }
}

/// Where a triangle crosses height `z`.
fn cut(t: [Vec3; 3], z: f64) -> Option<Segment> {
    // A vertex on the plane counts as above it, so every closed mesh gives closed outlines.
    let (mut from, mut to) = (None, None);
    for (a, b) in [(t[0], t[1]), (t[1], t[2]), (t[2], t[0])] {
        let (ua, ub) = (a.z >= z, b.z >= z);
        if ua != ub {
            let s = (z - a.z) / (b.z - a.z);
            let p = [a.x + (b.x - a.x) * s, a.y + (b.y - a.y) * s];
            if ua {
                from = Some(p);
            } else {
                to = Some(p);
            }
        }
    }
    Some([from?, to?])
}

pub fn area(s: &[Segment]) -> f64 {
    s.iter().map(|[a, b]| a[0] * b[1] - b[0] * a[1]).sum::<f64>().abs() * 0.5
}

pub fn perimeter(s: &[Segment]) -> f64 {
    s.iter().map(|[a, b]| (b[0] - a[0]).hypot(b[1] - a[1])).sum()
}

/// A layer as filled cells of a grid.
struct Raster {
    w: usize,
    h: usize,
    cells: Vec<bool>,
}

fn raster(s: &[Segment], lo: [f64; 2], cell: f64, w: usize, h: usize) -> Raster {
    let mut cells = vec![false; w * h];
    for j in 0..h {
        let y = lo[1] + (j as f64 + 0.5) * cell;
        let mut xs: Vec<(f64, i32)> = s
            .iter()
            .filter_map(|[a, b]| {
                let (ua, ub) = (a[1] >= y, b[1] >= y);
                (ua != ub).then(|| (a[0] + (b[0] - a[0]) * (y - a[1]) / (b[1] - a[1]), if ub { 1 } else { -1 }))
            })
            .collect();
        xs.sort_by(|a, b| a.0.total_cmp(&b.0));
        let mut wind = 0;
        for (k, (x, d)) in xs.iter().enumerate() {
            wind += d;
            if wind != 0
                && let Some((x2, _)) = xs.get(k + 1)
            {
                let a = ((x - lo[0]) / cell - 0.5).ceil().max(0.0) as usize;
                let b = (((x2 - lo[0]) / cell - 0.5).ceil().max(0.0) as usize).min(w);
                for c in cells.iter_mut().take(j * w + b).skip(j * w + a) {
                    *c = true;
                }
            }
        }
    }
    Raster { w, h, cells }
}

/// A patch of a layer with nothing under it: it would be printed in mid-air.
#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct Island {
    pub layer: usize,
    pub z: f64,
    pub x: f64,
    pub y: f64,
    pub area: f64,
}

const MAX_ISLANDS: usize = 500;
const GRID: usize = 512;

/// Patches that start in mid-air.
pub fn islands(s: &Slicer, min_area: f64) -> Vec<Island> {
    let b = s.mesh.bounds();
    if b.is_empty() || s.layers() < 2 {
        return Vec::new();
    }
    let size = b.size();
    let cell = (size.x.max(size.y) / (GRID as f64 - 4.0)).max(0.1);
    let (w, h) = (((size.x / cell).ceil() as usize + 2).min(GRID), ((size.y / cell).ceil() as usize + 2).min(GRID));
    let lo = [b.min.x - cell, b.min.y - cell];
    let mut out = Vec::new();
    // In blocks, so a tall print does not hold every layer at once.
    let mut below = raster(&s.segments(0), lo, cell, w, h);
    let mut at = 1;
    while at < s.layers() && out.len() < MAX_ISLANDS {
        let end = (at + 64).min(s.layers());
        let block: Vec<Raster> = (at..end).into_par_iter().map(|i| raster(&s.segments(i), lo, cell, w, h)).collect();
        for (k, r) in block.into_iter().enumerate() {
            for (count, cx, cy) in unsupported(&r, &below) {
                let area = count as f64 * cell * cell;
                if area >= min_area && out.len() < MAX_ISLANDS {
                    let i = at + k;
                    let (x, y) = (lo[0] + (cx + 0.5) * cell, lo[1] + (cy + 0.5) * cell);
                    out.push(Island { layer: i, z: s.bed + i as f64 * s.layer, x, y, area });
                }
            }
            below = r;
        }
        at = end;
    }
    out
}

/// Connected patches of `r` touching nothing in `below`: cell count and mean cell.
fn unsupported(r: &Raster, below: &Raster) -> Vec<(usize, f64, f64)> {
    let mut left = r.cells.clone();
    let mut out = Vec::new();
    let mut stack = Vec::new();
    for start in 0..r.cells.len() {
        if !std::mem::take(&mut left[start]) {
            continue;
        }
        let (mut n, mut sx, mut sy, mut held) = (0usize, 0.0, 0.0, false);
        stack.push(start);
        while let Some(c) = stack.pop() {
            let (x, y) = (c % r.w, c / r.w);
            n += 1;
            sx += x as f64;
            sy += y as f64;
            held |= below.cells.get(c).copied().unwrap_or(false);
            let mut visit = |o: usize| {
                if std::mem::take(&mut left[o]) {
                    stack.push(o);
                }
            };
            if x > 0 {
                visit(c - 1);
            }
            if x + 1 < r.w {
                visit(c + 1);
            }
            if y > 0 {
                visit(c - r.w);
            }
            if y + 1 < r.h {
                visit(c + r.w);
            }
        }
        if !held {
            out.push((n, sx / n as f64, sy / n as f64));
        }
    }
    out
}

/// How a part would be printed.
#[derive(Clone, Copy, Debug)]
pub struct PrintSettings {
    pub line_width: f64,
    pub walls: usize,
    /// Solid layers, top and bottom.
    pub skins: usize,
    /// 0 to 1.
    pub infill: f64,
    /// mm/s.
    pub wall_speed: f64,
    pub infill_speed: f64,
    /// mm³/s the hotend can melt.
    pub max_flow: f64,
    /// g/cm³.
    pub density: f64,
    /// Per kilogram.
    pub price: f64,
    pub filament_diameter: f64,
    /// mm³ of support material to add.
    pub support_volume: f64,
}

#[derive(Clone, Debug, Default, Serialize, PartialEq)]
pub struct Estimate {
    pub layers: usize,
    pub height: f64,
    pub volume_cm3: f64,
    pub support_cm3: f64,
    pub mass_g: f64,
    pub filament_m: f64,
    pub cost: f64,
    pub minutes: f64,
}

/// Material and time, layer by layer.
pub fn estimate(s: &Slicer, p: &PrintSettings) -> Estimate {
    let n = s.layers();
    let layers: Vec<(f64, f64)> = (0..n)
        .into_par_iter()
        .map(|i| {
            let seg = s.segments(i);
            (area(&seg), perimeter(&seg))
        })
        .collect();
    let mut volume = 0.0;
    let mut seconds = 0.0;
    let at = |i: isize| usize::try_from(i).ok().and_then(|i| layers.get(i)).map_or(0.0, |l| l.0);
    let bead = (p.line_width * s.layer).max(1e-6);
    let flow_speed = p.max_flow.max(0.1) / bead;
    let wall_speed = p.wall_speed.min(flow_speed).max(1.0);
    let infill_speed = p.infill_speed.min(flow_speed).max(1.0);
    let skins = p.skins as isize;
    for (i, &(a, per)) in layers.iter().enumerate() {
        let i = i as isize;
        let wall = (per * p.line_width * p.walls as f64).min(a);
        let inside = a - wall;
        // What is covered by fewer than `skins` layers above or below is printed solid.
        let exposed = if skins == 0 { 0.0 } else { (a - at(i - skins)).max(a - at(i + skins)).clamp(0.0, inside) };
        let fill = exposed + (inside - exposed) * p.infill.clamp(0.0, 1.0);
        volume += (wall + fill) * s.layer;
        let t = wall / p.line_width / wall_speed + fill / p.line_width / infill_speed;
        // Travel and acceleration cost about a third; a small layer is slowed to cool.
        seconds += (t * 1.3).max(5.0) + 0.5;
    }
    let support = p.support_volume.max(0.0);
    seconds += support / bead / infill_speed * 1.3;
    let total = volume + support;
    let mass = total / 1000.0 * p.density;
    let section = std::f64::consts::PI * (p.filament_diameter * 0.5).powi(2);
    Estimate {
        layers: n,
        height: s.top - s.bed,
        volume_cm3: total / 1000.0,
        support_cm3: support / 1000.0,
        mass_g: mass,
        filament_m: if section > 0.0 { total / section / 1000.0 } else { 0.0 },
        cost: mass / 1000.0 * p.price,
        minutes: if n == 0 { 0.0 } else { seconds / 60.0 + 4.0 },
    }
}

/// A rough amount of support under overhangs: the column beneath each, at `density`.
pub fn support_volume(m: &PrintMesh, overhangs: &[usize], bed: f64, density: f64) -> f64 {
    let bvh = m.bvh();
    overhangs
        .par_iter()
        .filter_map(|&i| {
            let t = m.tri(i)?;
            let (n, a) = crate::normal_area(t);
            let c = (t[0] + t[1] + t[2]) * (1.0 / 3.0);
            let o = c + Vec3::new(0.0, 0.0, -1e-3);
            let drop = m.raycast(&bvh, o, Vec3::new(0.0, 0.0, -1.0), 1e-4).map_or(c.z - bed, |(d, _)| d);
            Some(a * n.z.abs() * drop.max(0.0))
        })
        .sum::<f64>()
        * density
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cuboid;

    fn v(x: f64, y: f64, z: f64) -> Vec3 {
        Vec3::new(x, y, z)
    }

    #[test]
    fn layers_of_a_box() {
        let m = cuboid(v(0., 0., 0.), v(10., 20., 5.));
        let s = Slicer::new(&m, 0.2);
        assert_eq!(s.layers(), 25);
        for i in [0, 12, 24] {
            let seg = s.segments(i);
            assert!((area(&seg) - 200.0).abs() < 1e-6 && (perimeter(&seg) - 60.0).abs() < 1e-6, "{i}");
        }
    }

    #[test]
    fn a_floating_box_is_an_island() {
        let mut m = cuboid(v(0., 0., 0.), v(10., 10., 10.));
        assert!(islands(&Slicer::new(&m, 0.5), 1.0).is_empty());
        let f = cuboid(v(20., 0., 5.), v(25., 5., 10.));
        m.append(&f.positions, &f.triangles);
        let found = islands(&Slicer::new(&m, 0.5), 1.0);
        assert_eq!(found.len(), 1, "{found:?}");
        assert!((found[0].area - 25.0).abs() < 3.0 && (found[0].x - 22.5).abs() < 0.3 && (found[0].z - 5.0).abs() < 0.6, "{found:?}");
    }

    #[test]
    fn solid_infill_uses_the_whole_volume() {
        let m = cuboid(v(0., 0., 0.), v(20., 20., 20.));
        let mut p = PrintSettings {
            line_width: 0.42,
            walls: 2,
            skins: 4,
            infill: 1.0,
            wall_speed: 100.,
            infill_speed: 150.,
            max_flow: 15.,
            density: 1.24,
            price: 20.,
            filament_diameter: 1.75,
            support_volume: 0.0,
        };
        let full = estimate(&Slicer::new(&m, 0.2), &p);
        assert!((full.volume_cm3 - 8.0).abs() < 1e-6 && (full.mass_g - 9.92).abs() < 1e-6);
        p.infill = 0.15;
        let light = estimate(&Slicer::new(&m, 0.2), &p);
        assert!(light.mass_g < full.mass_g * 0.6 && light.minutes < full.minutes);
    }
}
