//! Models from pictures.

use super::*;

/// Brightness from 0 to 1, row 0 at the bottom.
struct Picture {
    w: usize,
    h: usize,
    px: Vec<f32>,
}

impl Picture {
    fn at(&self, x: f64, y: f64) -> f64 {
        let (x, y) = (x.clamp(0.0, (self.w - 1) as f64), y.clamp(0.0, (self.h - 1) as f64));
        let (i, j) = ((x as usize).min(self.w.saturating_sub(2)), (y as usize).min(self.h.saturating_sub(2)));
        let (u, t) = (x - i as f64, y - j as f64);
        let g = |i: usize, j: usize| f64::from(self.px.get(j * self.w + i).copied().unwrap_or(1.0));
        g(i, j) * (1. - u) * (1. - t) + g(i + 1, j) * u * (1. - t) + g(i, j + 1) * (1. - u) * t + g(i + 1, j + 1) * u * t
    }
}

/// Read the picture scaled to at most `most` points each way.
fn read(a: &Args, most: u32) -> Result<Picture> {
    let path = a.text("image").trim();
    if path.is_empty() {
        return Err(a.fail("choose a picture"));
    }
    let bytes = rybocraft_io::vfs::read(path, 64 << 20).map_err(|e| a.fail(format!("{path}: {e}")))?;
    let img = image::load_from_memory(&bytes).map_err(|e| a.fail(format!("{path}: {e}")))?;
    let most = most.clamp(16, 1500);
    let img = if img.width() > most || img.height() > most { img.thumbnail(most, most) } else { img };
    if img.width() < 2 || img.height() < 2 {
        return Err(a.fail("the picture is too small"));
    }
    let rgba = img.to_rgba8();
    let (w, h) = (rgba.width() as usize, rgba.height() as usize);
    let mut px = vec![1.0; w * h];
    for (x, y, p) in rgba.enumerate_pixels() {
        let [r, g, b, al] = p.0.map(|c| f32::from(c) / 255.0);
        // Clear parts read as white paper.
        let lum = (0.2126 * r + 0.7152 * g + 0.0722 * b) * al + (1.0 - al);
        if let Some(slot) = px.get_mut((h - 1 - y as usize) * w + x as usize) {
            *slot = lum;
        }
    }
    Ok(Picture { w, h, px })
}

/// The picture's grid on the model: cells each way, their size, and the picture's height.
fn grid(a: &Args, p: &Picture) -> Result<(usize, usize, f64, f64)> {
    let w = a.num("width");
    let h = w * p.h as f64 / p.w as f64;
    let pitch = a.num("pitch").max(((w * h) / 900_000.0).sqrt());
    let (nx, ny) = ((w / pitch).ceil() as usize + 1, (h / pitch).ceil() as usize + 1);
    if nx < 3 || ny < 3 {
        return Err(a.fail("the picture would be too small"));
    }
    Ok((nx, ny, pitch, h))
}

pub(super) fn lithophane(s: &mut Session, a: &Args) -> Result<Value> {
    let o = a.at();
    let p = read(a, 1500)?;
    let (nx, ny, pitch, h) = grid(a, &p)?;
    let (w, thin, thick, border) = (a.num("width"), a.num("thin"), a.num("thick"), a.num("border"));
    if thick <= thin {
        return Err(a.fail("the thickest part must be thicker than the thinnest"));
    }
    let inv = a.flag("invert");
    let k = (border / pitch).round() as usize;
    let (tx, ty) = (nx + 2 * k, ny + 2 * k);
    let top = |i: usize, j: usize| {
        if i < k || j < k || i >= nx + k || j >= ny + k {
            return thick;
        }
        let lum = p.at((i - k) as f64 / (nx - 1) as f64 * (p.w - 1) as f64, (j - k) as f64 / (ny - 1) as f64 * (p.h - 1) as f64);
        let lum = if inv { 1.0 - lum } else { lum };
        thick + (thin - thick) * lum
    };
    let full = (tx - 1) as f64 * pitch;
    let sweep = a.num("curve").to_radians();
    // Upright, so it prints with its detail along the layers.
    let m = if sweep > 0.01 {
        let r = full / sweep;
        sdf::height_slab(tx, ty, top, |u, z, d| {
            let ang = -std::f64::consts::FRAC_PI_2 + (u / (tx - 1) as f64 - 0.5) * sweep;
            o + v((r + d) * ang.cos(), (r + d) * ang.sin(), z * pitch)
        })
    } else {
        sdf::height_slab(tx, ty, top, |u, z, d| o + v(u * pitch - full * 0.5, -d, z * pitch))
    };
    mesh_made(s, "Lithophane", &outward(m), format!("Lithophane, {w:.0} × {h:.0} mm"), Vec::new())
}

fn outward(mut m: PrintMesh) -> PrintMesh {
    if m.volume() < 0.0 {
        for t in &mut m.triangles {
            t.swap(1, 2);
        }
    }
    m
}

pub(super) fn relief(s: &mut Session, a: &Args) -> Result<Value> {
    let o = a.at();
    let mut p = read(a, 1500)?;
    for _ in 0..a.int("smooth") {
        let old = std::mem::take(&mut p.px);
        let g = |i: usize, j: usize| old.get(j * p.w + i).copied().unwrap_or(1.0);
        p.px = (0..p.w * p.h)
            .map(|n| {
                let (i, j) = (n % p.w, n / p.w);
                let (l, r, d, u) = (i.saturating_sub(1), (i + 1).min(p.w - 1), j.saturating_sub(1), (j + 1).min(p.h - 1));
                (2.0 * g(i, j) + g(l, j) + g(r, j) + g(i, d) + g(i, u)) / 6.0
            })
            .collect();
    }
    let (nx, ny, pitch, h) = grid(a, &p)?;
    let (base, high, inv, w) = (a.num("base"), a.num("height"), a.flag("invert"), a.num("width"));
    let top = |i: usize, j: usize| {
        let lum = p.at(i as f64 / (nx - 1) as f64 * (p.w - 1) as f64, j as f64 / (ny - 1) as f64 * (p.h - 1) as f64);
        base + high * if inv { 1.0 - lum } else { lum }
    };
    let m = sdf::height_slab(nx, ny, top, |u, t, z| o + v(u * pitch - w * 0.5, t * pitch - h * 0.5, z));
    mesh_made(s, "Relief", &outward(m), format!("Relief, {w:.0} × {h:.0} mm"), Vec::new())
}

/// The picture's dark shape as a distance-like field in mm, centred: negative inside. Distances are
/// measured on the grid, so they are true, not just a threshold.
struct Shape {
    n: [usize; 2],
    pitch: f64,
    size: [f64; 2],
    d: Vec<f32>,
}

impl Shape {
    fn new(a: &Args, p: &Picture, width: f64, pitch: f64, invert: bool, fill: bool) -> Result<Shape> {
        let th = a.num("threshold") / 100.0;
        let size = [width, width * p.h as f64 / p.w as f64];
        // A margin all round, so offsets outside the picture are measured too.
        let pad = (30.0 / pitch).ceil() as usize;
        let pitch = pitch.max(((size[0] + 60.0) * (size[1] + 60.0) / 4_000_000.0).sqrt());
        let inner = [(size[0] / pitch).ceil() as usize + 1, (size[1] / pitch).ceil() as usize + 1];
        let n = [inner[0] + 2 * pad, inner[1] + 2 * pad];
        let mut solid = vec![false; n[0] * n[1]];
        for j in 0..inner[1] {
            for i in 0..inner[0] {
                let lum = p.at(i as f64 / (inner[0] - 1) as f64 * (p.w - 1) as f64, j as f64 / (inner[1] - 1) as f64 * (p.h - 1) as f64);
                if let Some(c) = solid.get_mut((j + pad) * n[0] + i + pad) {
                    *c = (lum < th) != invert;
                }
            }
        }
        if fill {
            // Only what the outside can reach stays empty.
            let mut out = vec![false; solid.len()];
            let mut stack = vec![0usize];
            while let Some(c) = stack.pop() {
                if solid.get(c).copied().unwrap_or(true) || std::mem::replace(&mut out[c], true) {
                    continue;
                }
                let (x, y) = (c % n[0], c / n[0]);
                if x > 0 {
                    stack.push(c - 1);
                }
                if x + 1 < n[0] {
                    stack.push(c + 1);
                }
                if y > 0 {
                    stack.push(c - n[0]);
                }
                if y + 1 < n[1] {
                    stack.push(c + n[0]);
                }
            }
            for (c, o) in solid.iter_mut().zip(&out) {
                *c = !*o;
            }
        }
        if !solid.iter().any(|c| *c) {
            return Err(a.fail("no part of the picture is past the threshold: change it, or invert"));
        }
        let outside = squared_distance(&solid, n, true);
        let inside = squared_distance(&solid, n, false);
        let d = outside.iter().zip(&inside).map(|(o, i)| ((o.sqrt() - i.sqrt()) * pitch) as f32).collect();
        Ok(Shape { n, pitch, size, d })
    }
    fn at(&self, x: f64, y: f64) -> f64 {
        let fx = (x / self.pitch + (self.n[0] - 1) as f64 * 0.5).clamp(0.0, (self.n[0] - 1) as f64 - 1e-6);
        let fy = (y / self.pitch + (self.n[1] - 1) as f64 * 0.5).clamp(0.0, (self.n[1] - 1) as f64 - 1e-6);
        let (i, j) = (fx as usize, fy as usize);
        let (u, t) = (fx - i as f64, fy - j as f64);
        let g = |i: usize, j: usize| f64::from(self.d.get(j * self.n[0] + i).copied().unwrap_or(1.0));
        g(i, j) * (1. - u) * (1. - t) + g(i + 1, j) * u * (1. - t) + g(i, j + 1) * (1. - u) * t + g(i + 1, j + 1) * u * t
    }
}

/// Squared distance, in cells, to the nearest cell that is `to`.
fn squared_distance(solid: &[bool], n: [usize; 2], to: bool) -> Vec<f64> {
    let mut d: Vec<f64> = solid.iter().map(|c| if *c == to { 0.0 } else { 1e12 }).collect();
    let mut pass = |start: usize, step: usize, count: usize| {
        let line: Vec<f64> = (0..count).map(|k| d[start + k * step]).collect();
        // The lower envelope of parabolas.
        let mut hull: Vec<(usize, f64)> = Vec::with_capacity(count);
        let cross = |p: usize, q: usize| ((line[q] + (q * q) as f64) - (line[p] + (p * p) as f64)) / (2.0 * (q as f64 - p as f64));
        for q in 0..count {
            let mut at = f64::NEG_INFINITY;
            while let Some(&(p, from)) = hull.last() {
                at = cross(p, q);
                if at > from {
                    break;
                }
                hull.pop();
                at = f64::NEG_INFINITY;
            }
            hull.push((q, at));
        }
        let mut k = 0;
        for q in 0..count {
            while hull.get(k + 1).is_some_and(|h| h.1 < q as f64) {
                k += 1;
            }
            if let Some(&(p, _)) = hull.get(k) {
                d[start + q * step] = (q as f64 - p as f64).powi(2) + line[p];
            }
        }
    };
    for i in 0..n[0] {
        pass(i, n[0], n[1]);
    }
    for j in 0..n[1] {
        pass(j * n[0], 1, n[0]);
    }
    d
}

pub(super) fn logo(s: &mut Session, a: &Args) -> Result<Value> {
    let o = a.at();
    let p = read(a, 800)?;
    let (w, depth) = (a.num("width"), a.num("depth"));
    let style = a.text("style").to_string();
    let mut warnings = Vec::new();
    let shape = Shape::new(a, &p, w, a.num("detail").min(0.5), a.flag("invert"), false)?;
    let half = [shape.size[0] * 0.5, shape.size[1] * 0.5];
    let edge = 4.0;
    let base = if style == "shape" { 0.0 } else { 2.4 };
    let mirror = style == "stamp";
    let ring = 4.0;
    let mut b: rybocraft_geom::Aabb3 = [v(-half[0] - edge, -half[1] - edge, 0.), v(half[0] + edge, half[1] + edge, base + depth)].into_iter().collect();
    if style == "keychain" {
        b.max.y += 2.0 * ring + 3.0;
    }
    let outline = if style == "keychain" { Some(Shape::new(a, &p, w, a.num("detail").min(0.5), a.flag("invert"), true)?) } else { None };
    let h = detail(a, &b, &mut warnings).min(depth * 0.5);
    let m = sdf::polygonize(b, h, |q| {
        let d = shape.at(if mirror { -q.x } else { q.x }, q.y);
        let raised = d.max(q.z - base - depth).max((base - 0.5 - q.z).min(d)).max(-q.z);
        let plate = match style.as_str() {
            "plate" | "stamp" => sdf::cuboid(v(q.x, q.y, 0.), v(-half[0] - edge, -half[1] - edge, -50.), v(half[0] + edge, half[1] + edge, 50.), edge).max(q.z - base).max(-q.z),
            "keychain" => {
                let around = outline.as_ref().map_or(d, |o| o.at(q.x, q.y)) - 2.4;
                let at = v(0.0, half[1] + ring, 0.0);
                let r = v(q.x, q.y, 0.).dist(at);
                let tab = (r - ring - 0.5).min(sdf::capsule(v(q.x, q.y, 0.), at, v(0., half[1] - 6.0, 0.), 2.5));
                around.min(tab).max(2.2 - r).max(q.z - base).max(-q.z)
            }
            _ => f64::INFINITY,
        };
        if style == "shape" { d.max(q.z - depth).max(-q.z) } else { raised.max(0.2 - q.z).min(plate) }
    })
    .map_err(|e| a.fail(e))?;
    let m = shift(m, o);
    mesh_made(s, "Cut-out", &m, format!("{style} {:.0} × {:.0} mm", shape.size[0], shape.size[1]), warnings)
}

fn shift(mut m: PrintMesh, by: Vec3) -> PrintMesh {
    for p in &mut m.positions {
        *p = *p + by;
    }
    m
}

/// Built-in outlines, fitted to `w` across.
fn outline(kind: &str, w: f64) -> Vec<[f64; 2]> {
    let tau = std::f64::consts::TAU;
    let n = 160;
    let pts: Vec<[f64; 2]> = match kind {
        "star" => (0..10)
            .map(|i| {
                let (r, t) = (if i % 2 == 0 { 1.0 } else { 0.5 }, tau * f64::from(i) / 10.0 + tau / 4.0);
                [r * t.cos(), r * t.sin()]
            })
            .collect(),
        "hexagon" => hexagon(1.0, [0., 0.]),
        "circle" => (0..n).map(|i| tau * f64::from(i) / f64::from(n)).map(|t| [t.cos(), t.sin()]).collect(),
        "flower" => (0..n).map(|i| tau * f64::from(i) / f64::from(n)).map(|t| (t, 1.0 + 0.22 * (6.0 * t).cos())).map(|(t, r)| [r * t.cos(), r * t.sin()]).collect(),
        _ => (0..n)
            .map(|i| tau * f64::from(i) / f64::from(n))
            .map(|t| [16.0 * t.sin().powi(3), 13.0 * t.cos() - 5.0 * (2.0 * t).cos() - 2.0 * (3.0 * t).cos() - (4.0 * t).cos()])
            .collect(),
    };
    let (lo, hi) = pts.iter().fold((f64::INFINITY, f64::NEG_INFINITY), |(l, h), p| (l.min(p[0]), h.max(p[0])));
    let k = w / (hi - lo);
    pts.iter().map(|p| [p[0] * k, p[1] * k]).collect()
}

pub(super) fn cookie_cutter(s: &mut Session, a: &Args) -> Result<Value> {
    let o = a.at();
    let (w, h, t, rim) = (a.num("width"), a.num("height"), a.num("wall"), a.num("rim"));
    let mut warnings = Vec::new();
    let pts = outline(a.text("shape"), w);
    let shape = if a.text("shape") == "picture" { Some(Shape::new(a, &read(a, 600)?, w, 0.4, false, true)?) } else { None };
    let at = |x: f64, y: f64| shape.as_ref().map_or_else(|| sdf::polygon([x, y], &pts), |s| s.at(x, y));
    let (hx, hy) = match &shape {
        Some(s) => (s.size[0] * 0.5, s.size[1] * 0.5),
        None => (pts.iter().map(|p| p[0].abs()).fold(0.0, f64::max), pts.iter().map(|p| p[1].abs()).fold(0.0, f64::max)),
    };
    let m = t + rim + 1.0;
    let b: rybocraft_geom::Aabb3 = [v(-hx - m, -hy - m, 0.), v(hx + m, hy + m, h)].into_iter().collect();
    let cell = detail(a, &b, &mut warnings).min(t * 0.6);
    let mesh = sdf::polygonize(b, cell, |p| {
        let d = at(p.x, p.y);
        // Thinner towards the cutting edge.
        let wall = t * (1.0 - 0.45 * (p.z / h).clamp(0.0, 1.0));
        let blade = d.max(-d - 1e9).max(-d).max(d - wall);
        let grip = (-d).max(d - t - rim).max(p.z - 2.4);
        blade.min(grip).max(p.z - h).max(-p.z)
    })
    .map_err(|e| a.fail(e))?;
    mesh_made(s, "Cookie cutter", &shift(mesh, o), format!("{} cutter, {w:.0} mm", a.text("shape")), warnings)
}

pub(super) fn svg(s: &mut Session, a: &Args) -> Result<Value> {
    let path = a.text("image").trim().to_string();
    if path.is_empty() {
        return Err(a.fail("choose an SVG file"));
    }
    let before = names(s)?;
    let flat = Flat::level(a.at());
    sketched(s, flat, a.num("depth"), 0.0, Do::New("SVG"), |s| s.execute("sketch.insert_svg", &json!({"path": path})).map(|_| ()))?;
    let new = new_since(s, &before)?;
    made(format!("{} solid(s) from the drawing", new.len()), &new)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn distances_on_a_grid_are_true() {
        let n = [40, 30];
        let solid: Vec<bool> = (0..1200).map(|c| c == 15 * 40 + 20).collect();
        let d = squared_distance(&solid, n, true);
        for (c, d) in d.iter().enumerate() {
            let (x, y) = ((c % 40) as f64 - 20.0, (c / 40) as f64 - 15.0);
            assert!((d - x * x - y * y).abs() < 1e-6, "{c}");
        }
    }
}
