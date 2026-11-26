//! Ready-made printable parts, built as ordinary editable features.

use serde_json::{Value, json};
use rybocore_record::Vec3;

use super::*;

const UP: Vec3 = Vec3::new(0.0, 0.0, 1.0);

fn v(x: f64, y: f64, z: f64) -> Vec3 {
    Vec3::new(x, y, z)
}

fn made(summary: String, bodies: &[String]) -> Result<Value> {
    Ok(json!({"summary": summary, "created_bodies": bodies}))
}

pub(super) fn hinge(s: &mut Session, a: &Args) -> Result<Value> {
    let o = a.at();
    let (l, leaf, t, d, c) = (a.num("length"), a.num("leaf"), a.num("thickness"), a.num("barrel"), a.num("clearance"));
    let n = a.int("knuckles") | 1;
    let r = d * 0.5;
    if t > r {
        return Err(a.fail("the leaves must be no thicker than half the barrel"));
    }
    let seg = (l - c * (n as f64 - 1.0)) / n as f64;
    // Cones: wide enough to hold, with at least 0.8 mm of barrel around them.
    let (big, small) = (r - 0.8 - c, (r - 0.8 - c) * 0.45);
    let reach = (big - small).max(0.8);
    if small < 0.5 || seg < reach * 2.0 + 1.5 {
        return Err(a.fail("too small to hold together: use a larger barrel, a longer hinge or fewer knuckles"));
    }
    let x = |i: usize| o.x - l * 0.5 + i as f64 * (seg + c);
    let axis_z = o.z + r;
    let along = v(1.0, 0.0, 0.0);
    let mut bodies = Vec::new();
    for side in 0..2usize {
        let sign = if side == 0 { -1.0 } else { 1.0 };
        // The plate stops clear of the other leaf's knuckles.
        let near = r + c;
        let y0 = if side == 0 { o.y - near - leaf } else { o.y + near };
        let name = if side == 0 { "Hinge leaf A" } else { "Hinge leaf B" };
        let body = cuboid(s, v(o.x - l * 0.5, y0, o.z), [l, leaf, t], Do::New(name))?;
        for i in (side..n).step_by(2) {
            let arm_y = if sign < 0.0 { o.y - near - 0.5 } else { o.y };
            cuboid(s, v(x(i), arm_y, o.z), [seg, near + 0.5, t], Do::Join(&body))?;
            cylinder(s, v(x(i), o.y, axis_z), along, d, seg, Do::Join(&body))?;
        }
        bodies.push(body);
    }
    // Leaf A's knuckles carry the cones, leaf B's the matching sockets.
    for i in (1..n).step_by(2) {
        for (face, dir) in [(x(i), 1.0), (x(i) + seg, -1.0)] {
            cone(s, v(face, o.y, axis_z), along * dir, 2.0 * (big + c), 2.0 * (small + c), reach, Do::Cut(&bodies[1]))?;
            let from = face - dir * c;
            cone(s, v(from - dir * 0.2, o.y, axis_z), along * dir, 2.0 * big, 2.0 * big, 0.2, Do::Join(&bodies[0])).or_else(|_| {
                // Equal circles: a cylinder where a cone must taper.
                cylinder(s, v(from - dir * 0.2, o.y, axis_z), along * dir, 2.0 * big, 0.2, Do::Join(&bodies[0]))
            })?;
            cone(s, v(from, o.y, axis_z), along * dir, 2.0 * big, 2.0 * small, reach, Do::Join(&bodies[0]))?;
        }
    }
    made(format!("Print-in-place hinge, {n} knuckles, {c} mm clearance: print it open and flat, then work it loose"), &bodies)
}

pub(super) fn enclosure(s: &mut Session, a: &Args) -> Result<Value> {
    let o = a.at();
    let (l, w, h, t, r) = (a.num("length"), a.num("width"), a.num("height"), a.num("wall"), a.num("radius"));
    let f = a.fastener()?;
    let (ol, ow) = (l + 2.0 * t, w + 2.0 * t);
    let inner = (r - t).max(0.6);
    let post = f.insert[0] + 2.0 * 1.8;
    let posts = a.flag("posts");
    if posts && (post * 2.0 + 4.0 > l.min(w) || h < f.insert[1] + 1.0) {
        return Err(a.fail("too small inside for the screw posts: make it larger or turn them off"));
    }
    let base = rounded(s, Flat::level(o), [ol, ow], r, t + h, 0.0, Do::New("Enclosure"))?;
    rounded(s, Flat::level(o + UP * t), [l, w], inner, h + 0.1, 0.0, Do::Cut(&base))?;
    let k = post * 0.5 - 0.3;
    let corners: Vec<[f64; 2]> =
        if posts { [[-1., -1.], [1., -1.], [1., 1.], [-1., 1.]].iter().map(|c| [c[0] * (l * 0.5 - k), c[1] * (w * 0.5 - k)]).collect() } else { Vec::new() };
    for c in &corners {
        // Rooted in the floor and the walls, its top level with the rim.
        cylinder(s, o + v(c[0], c[1], t * 0.5), UP, post, t * 0.5 + h, Do::Join(&base))?;
    }
    for c in &corners {
        let top = o + v(c[0], c[1], t + h);
        cylinder(s, top + UP * 0.05, UP * -1.0, f.insert[0], f.insert[1].min(h) + 0.05, Do::Cut(&base))?;
    }
    // The lid prints upside down beside the base, its lip up.
    let lid_at = o + v(ol + 10.0, 0.0, 0.0);
    let lid = rounded(s, Flat::level(lid_at), [ol, ow], r, t, 0.0, Do::New("Enclosure lid"))?;
    let c = a.num("clearance");
    let lip = [l - 2.0 * c, w - 2.0 * c];
    let lip_h = 2.5f64.min(h * 0.3);
    rounded(s, Flat::level(lid_at + UP * (t - 0.1)), lip, (inner - c).max(0.3), lip_h + 0.1, 0.0, Do::Join(&lid))?;
    rounded(s, Flat::level(lid_at + UP * t), [lip[0] - 3.0, lip[1] - 3.0], (inner - c - 1.5).max(0.3), lip_h + 0.2, 0.0, Do::Cut(&lid))?;
    for q in &corners {
        let at = lid_at + v(q[0], q[1], 0.0);
        // The lip gives way to the posts; the head sinks into the outside.
        cylinder(s, at + UP * t, UP, post + 2.0 * c + 0.4, lip_h + 0.2, Do::Cut(&lid))?;
        cylinder(s, at - UP * 0.05, UP, f.clearance, t + 0.2, Do::Cut(&lid))?;
        let sink = (f.head[1]).min(t - 1.0);
        if sink > 0.3 {
            cylinder(s, at - UP * 0.05, UP, f.head[0] + 0.5, sink + 0.05, Do::Cut(&lid))?;
        }
    }
    made(format!("Enclosure {l} × {w} × {h} mm inside, with a lid"), &[base, lid])
}

/// Pitch of the storage grid.
const GRID: f64 = 42.0;

pub(super) fn grid_bin(s: &mut Session, a: &Args) -> Result<Value> {
    let o = a.at();
    let (nx, ny, nz, t) = (a.int("units_x"), a.int("units_y"), a.num("units_z"), a.num("wall"));
    let (fx, fy) = (nx as f64 * GRID, ny as f64 * GRID);
    let size = [fx - 0.5, fy - 0.5];
    let height = nz * 7.0;
    let foot = 4.75;
    // The walls and floor first, so every foot joins one body.
    let bin = rounded(s, Flat::level(o + UP * foot), size, 3.75, height - foot, 0.0, Do::New("Grid bin"))?;
    let floor = 7.0;
    rounded(s, Flat::level(o + UP * floor), [size[0] - 2.0 * t, size[1] - 2.0 * t], (3.75 - t).max(0.5), height - floor + 0.1, 0.0, Do::Cut(&bin))?;
    let mut magnets = Vec::new();
    for i in 0..nx {
        for j in 0..ny {
            let c = o + v((i as f64 + 0.5) * GRID - fx * 0.5, (j as f64 + 0.5) * GRID - fy * 0.5, 0.0);
            // A chamfer, a straight band and a chamfer: 0.8, 1.8 and 2.15 mm.
            rounded(s, Flat::level(c + UP * 2.6), [37.2, 37.2], 1.6, 2.15, 45.0, Do::Join(&bin))?;
            rounded(s, Flat::level(c + UP * 0.8), [37.2, 37.2], 1.6, 1.8, 0.0, Do::Join(&bin))?;
            rounded(s, Flat::level(c), [35.6, 35.6], 0.8, 0.8, 45.0, Do::Join(&bin))?;
            magnets.extend([[-13., -13.], [13., -13.], [13., 13.], [-13., 13.]].map(|[x, y]: [f64; 2]| c + v(x, y, -0.05)));
        }
    }
    if a.flag("magnets") {
        for m in magnets {
            cylinder(s, m, UP, 6.5, 2.45, Do::Cut(&bin))?;
        }
    }
    let n = a.int("dividers");
    for i in 1..=n {
        let x = o.x - size[0] * 0.5 + size[0] * i as f64 / (n as f64 + 1.0) - t * 0.5;
        cuboid(s, v(x, o.y - size[1] * 0.5 + t * 0.5, o.z + floor - 0.1), [t, size[1] - t, height - floor + 0.1], Do::Join(&bin))?;
    }
    made(format!("{nx} × {ny} grid bin, {height} mm high"), &[bin])
}

pub(super) fn grid_base(s: &mut Session, a: &Args) -> Result<Value> {
    let o = a.at();
    let (nx, ny) = (a.int("units_x"), a.int("units_y"));
    let (fx, fy) = (nx as f64 * GRID, ny as f64 * GRID);
    let base = rounded(s, Flat::level(o), [fx, fy], 4.0, 4.65, 0.0, Do::New("Grid base"))?;
    for i in 0..nx {
        for j in 0..ny {
            let c = o + v((i as f64 + 0.5) * GRID - fx * 0.5, (j as f64 + 0.5) * GRID - fy * 0.5, 0.0);
            // The foot's shape, a quarter millimetre larger all round, open below.
            rounded(s, Flat::level(c + UP * 2.5), [37.7, 37.7], 1.85, 2.15, 45.0, Do::Cut(&base))?;
            rounded(s, Flat::level(c + UP * 0.7), [37.7, 37.7], 1.85, 1.8, 0.0, Do::Cut(&base))?;
            rounded(s, Flat::level(c), [36.3, 36.3], 1.15, 0.7, 45.0, Do::Cut(&base))?;
        }
    }
    made(format!("{nx} × {ny} grid base plate"), &[base])
}

/// One involute spur gear's outline, counter-clockwise.
pub(super) fn gear_outline(teeth: usize, module: f64, pressure: f64, backlash: f64) -> Vec<[f64; 2]> {
    let z = teeth as f64;
    let (pitch, pa) = (module * z * 0.5, pressure.to_radians());
    let (base, tip) = (pitch * pa.cos(), pitch + module);
    let root = (pitch - 1.25 * module).max(module * 0.5);
    let inv = |r: f64| {
        let t = ((r / base).powi(2) - 1.0).max(0.0).sqrt();
        t - t.atan()
    };
    // Half a tooth's angle at the pitch circle, less the backlash.
    let half = std::f64::consts::PI / (2.0 * z) - backlash / (4.0 * pitch);
    let at_base = half + inv(pitch);
    let start = base.max(root);
    const N: usize = 6;
    let flank: Vec<(f64, f64)> = (0..=N)
        .map(|i| {
            let r = start + (tip - start) * i as f64 / N as f64;
            (r, (at_base - inv(r)).max(0.002))
        })
        .collect();
    let mut out = Vec::new();
    let mut push = |r: f64, a: f64| out.push([r * a.cos(), r * a.sin()]);
    for k in 0..teeth {
        let c = std::f64::consts::TAU * k as f64 / z;
        if root < base - 1e-6 {
            push(root, c - at_base);
        }
        for (r, a) in &flank {
            push(*r, c - a);
        }
        for (r, a) in flank.iter().rev() {
            push(*r, c + a);
        }
        if root < base - 1e-6 {
            push(root, c + at_base);
        }
    }
    out
}

pub(super) fn gear(s: &mut Session, a: &Args) -> Result<Value> {
    let o = a.at();
    let (z, m, t, bore) = (a.int("teeth"), a.num("module"), a.num("thickness"), a.num("bore"));
    let outline = gear_outline(z, m, a.num("pressure"), a.num("backlash"));
    if bore > m * (z as f64 - 2.5) - 1.6 {
        return Err(a.fail("the bore leaves no rim"));
    }
    let gear = prism(s, Flat::level(o), &outline, t, Do::New("Gear"))?;
    if bore > 0.0 {
        cylinder(s, o - UP * 0.05, UP, bore, t + 0.1, Do::Cut(&gear))?;
    }
    let pitch = m * z as f64;
    let r = made(format!("{z}-tooth gear, module {m}, {pitch} mm pitch diameter"), &[gear])?;
    Ok(with(r, json!({"pitch_diameter_mm": pitch, "outside_diameter_mm": pitch + 2.0 * m})))
}

fn with(mut v: Value, more: Value) -> Value {
    if let (Some(o), Value::Object(m)) = (v.as_object_mut(), more) {
        o.extend(m);
    }
    v
}

pub(super) fn knob(s: &mut Session, a: &Args) -> Result<Value> {
    let o = a.at();
    let (d, h, n) = (a.num("diameter"), a.num("height"), a.int("lobes"));
    let f = a.fastener()?;
    let pocket = f.nut[1] + 0.4;
    if d * 0.6 < f.nut[0] * 1.3 + 3.0 || h < pocket + 2.0 {
        return Err(a.fail("too small for that thread"));
    }
    let knob = cylinder(s, o, UP, d, h, Do::New("Knob"))?;
    // Scallops wide enough for half the pitch each.
    let scallop = std::f64::consts::PI * d / n as f64 * 0.55;
    for k in 0..n {
        let (sn, cs) = (std::f64::consts::TAU * k as f64 / n as f64).sin_cos();
        let r = d * 0.5 + scallop * 0.28;
        cylinder(s, o + v(r * cs, r * sn, -0.05), UP, scallop, h + 0.1, Do::Cut(&knob))?;
    }
    cylinder(s, o - UP * 0.05, UP, f.clearance, h + 0.1, Do::Cut(&knob))?;
    prism(s, Flat::level(o + UP * (h - pocket)), &hexagon(f.nut[0] + 0.2, [0.0, 0.0]), pocket + 0.1, Do::Cut(&knob))?;
    made(format!("{} knob, {d} mm", f.size), &[knob])
}

pub(super) fn hook(s: &mut Session, a: &Args) -> Result<Value> {
    let o = a.at();
    let (w, h, reach, t) = (a.num("width"), a.num("height"), a.num("reach"), a.num("thickness"));
    let f = a.fastener()?;
    if h < t * 3.0 + f.head[0] * 2.0 || w < f.head[0] + 2.0 {
        return Err(a.fail("too small for the screws: make it taller or wider"));
    }
    // Seen from the side, lying on the bed: the wall along Y, the arm along X, braced and upturned.
    let tip = t * 1.8;
    let points = [
        [0.0, 0.0],
        [t, 0.0],
        [t + reach - t, t * 0.35],
        [t + reach, t * 0.35],
        [t + reach, t * 1.35 + tip],
        [reach, t * 1.35 + tip],
        [reach, t * 1.35],
        [t * 1.8, t * 1.35],
        [t, t * 2.2],
        [t, h],
        [0.0, h],
    ];
    let hook = prism(s, Flat::level(o), &points, w, Do::New("Hook"))?;
    let x = v(1.0, 0.0, 0.0);
    for y in [h - f.head[0] * 0.9, h * 0.5 + t * 0.6] {
        let face = o + v(t, y, w * 0.5);
        cylinder(s, face + x * 0.05, x * -1.0, f.clearance + 0.2, t + 0.1, Do::Cut(&hook))?;
        let sink = (f.nominal + 0.2).min(t - 1.2);
        if sink > 0.3 {
            let top = f.clearance + 0.2 + 2.0 * sink;
            cone(s, face + x * 0.05, x * -1.0, top + 0.1, f.clearance + 0.2, sink + 0.05, Do::Cut(&hook))?;
        }
    }
    made(format!("Wall hook, {reach} mm reach: print it as it lies"), &[hook])
}

pub(super) fn spacer(s: &mut Session, a: &Args) -> Result<Value> {
    let o = a.at();
    let f = a.fastener()?;
    let outer = if a.num("outer") > 0.0 { a.num("outer") } else { f.clearance * 2.0 + 0.4 };
    if outer < f.clearance + 1.2 {
        return Err(a.fail("the wall would be too thin"));
    }
    let h = a.num("height");
    let body = cylinder(s, o, UP, outer, h, Do::New("Spacer"))?;
    cylinder(s, o - UP * 0.05, UP, f.clearance + 0.1, h + 0.1, Do::Cut(&body))?;
    made(format!("{} spacer, {outer:.1} × {h} mm", f.size), &[body])
}

pub(super) fn calibration(s: &mut Session, a: &Args) -> Result<Value> {
    let o = a.at();
    let p = setup();
    let line = p.nozzle * 1.05;
    let (bodies, note) = match a.text("kind") {
        "flow_cube" => (vec![cuboid(s, o + v(-15., -15., 0.), [30., 30., 10.], Do::New("Flow cube"))?], "Print with one wall, no top and no infill; measure the wall against your line width"),
        "tolerance" => {
            let body = cuboid(s, o + v(-60., -12., 0.), [120., 24., 6.], Do::New("Tolerance gauge"))?;
            let mut out = Vec::new();
            for (i, c) in [0.1, 0.15, 0.2, 0.25, 0.3, 0.4].into_iter().enumerate() {
                let at = o + v(-50.0 + 20.0 * i as f64, 0.0, 0.0);
                // Double cones held in their sockets, loose by a different gap each.
                let (wide, narrow) = (12.0, 8.0);
                cylinder(s, at - UP * 0.05, UP, narrow + 2.0 * c, 6.1, Do::Cut(&body))?;
                cone(s, at, UP, narrow + 2.0 * c, wide + 2.0 * c, 3.0, Do::Cut(&body))?;
                cone(s, at + UP * 3.0, UP, wide + 2.0 * c, narrow + 2.0 * c, 3.0, Do::Cut(&body))?;
                let name = format!("Gauge {c}");
                let spinner = cone(s, at, UP, narrow, wide, 3.0, Do::New(&name))?;
                cone(s, at + UP * 3.0, UP, wide, narrow, 3.0, Do::Join(&spinner))?;
                prism(s, Flat::level(at + UP * 4.0), &hexagon(5.0, [0., 0.]), 2.1, Do::Cut(&spinner))?;
                out.push(spinner);
            }
            out.insert(0, body);
            (out, "Left to right: 0.1, 0.15, 0.2, 0.25, 0.3 and 0.4 mm gaps; the first that turns is your clearance")
        }
        "overhang" => {
            let body = cuboid(s, o, [10., 90., 3.], Do::New("Overhang test"))?;
            for (i, deg) in [70.0, 60.0, 50.0, 45.0, 40.0, 35.0, 30.0, 20.0f64].into_iter().enumerate() {
                // A leaning wall: `deg` from horizontal.
                let run = 12.0 / deg.to_radians().tan();
                let y = o.y + 2.0 + 11.0 * i as f64;
                let flat = Flat { origin: v(o.x, y + 9.0, o.z), x: v(1., 0., 0.), y: UP };
                prism(s, flat, &[[0., 0.], [10., 0.], [10., 3.], [10. + run.min(34.), 3. + run.min(34.) * deg.to_radians().tan()], [0., 3. + run.min(34.) * deg.to_radians().tan()]], 9.0, Do::Join(&body))?;
            }
            (vec![body], "From 70° down to 20° (from horizontal); the last clean one is your overhang angle")
        }
        "bridging" => {
            let body = cuboid(s, o, [8., 60., 12.], Do::New("Bridging test"))?;
            for (i, span) in [10.0, 20.0, 30.0, 40.0, 60.0].into_iter().enumerate() {
                let y = o.y + 12.0 * i as f64;
                cuboid(s, v(o.x + 8.0 + span, y, o.z), [6., 10., 12.], Do::New("Bridge pier")).and_then(|pier| {
                    cuboid(s, v(o.x + 4.0, y, o.z + 11.0), [span + 10.0, 10., 1.0], Do::Join(&body))?;
                    s.execute("solid.combine", &json!({"target": body, "tools": [pier], "operation": "join"})).map(|_| ())
                })?;
            }
            (vec![body], "Bridges of 10, 20, 30, 40 and 60 mm")
        }
        "temperature_tower" => {
            let body = cuboid(s, o + v(-25., -6., 0.), [50., 12., 2.], Do::New("Temperature tower"))?;
            for i in 0..7 {
                let z = o.z + 2.0 + 10.0 * f64::from(i);
                for x in [-20.0, 10.0] {
                    cuboid(s, v(o.x + x, o.y - 5.0, z - 0.1), [10., 10., 10.1], Do::Join(&body))?;
                }
                cuboid(s, v(o.x - 10.1, o.y - 5.0, z + 8.5), [20.2, 10., 1.5], Do::Join(&body))?;
                let flat = Flat { origin: v(o.x - 10.1 - 9.9, o.y + 5.0, z), x: v(1., 0., 0.), y: UP };
                prism(s, flat, &[[0.2, 4.0], [0.2, 10.0], [-5.8, 10.0]], 10.0, Do::Join(&body))?;
            }
            (vec![body], "Seven 10 mm steps above a 2 mm base: lower the nozzle 5° at each (in your slicer)")
        }
        "retraction" => {
            let body = cuboid(s, o + v(-30., -6., 0.), [60., 12., 1.2], Do::New("Retraction test"))?;
            for x in [-22.0, 22.0] {
                cone(s, o + v(x, 0., 1.0), UP, 9.0, 4.0, 45.0, Do::Join(&body))?;
            }
            (vec![body], "Change retraction with height; strings between the towers show where it is too little")
        }
        "first_layer" => {
            let mut out = Vec::new();
            let (x, y) = ((p.bed[0] * 0.5 - 30.0).max(20.0), (p.bed[1] * 0.5 - 30.0).max(20.0));
            for c in [[0., 0.], [-1., -1.], [1., -1.], [1., 1.], [-1., 1.]] {
                out.push(cuboid(s, o + v(c[0] * x - 12.5, c[1] * y - 12.5, 0.), [25., 25., p.layer], Do::New("First layer"))?);
            }
            (out, "One layer in the middle and the corners of your bed")
        }
        "thin_walls" => {
            let body = cuboid(s, o, [60., 30., 1.2], Do::New("Thin wall test"))?;
            for i in 1..=8 {
                let t = line * 0.5 * f64::from(i);
                cuboid(s, o + v(2.0 + 7.0 * f64::from(i - 1), 3.0, 1.0), [t, 24., 15.2], Do::Join(&body))?;
            }
            (vec![body], "Walls of half to four line widths: the thinnest that prints solid is your minimum")
        }
        "hole_sizes" => {
            let body = cuboid(s, o, [100., 22., 5.], Do::New("Hole size test"))?;
            for (i, d) in [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 8.0, 10.0].into_iter().enumerate() {
                cylinder(s, o + v(6.0 + 12.4 * i as f64, 11., -0.05), UP, d, 5.1, Do::Cut(&body))?;
            }
            (vec![body], "Holes of 1, 2, 3, 4, 5, 6, 8 and 10 mm: measure them to find your hole compensation")
        }
        _ => {
            let body = cuboid(s, o + v(-10., -10., 0.), [20., 20., 20.], Do::New("Calibration cube"))?;
            // Notches tell the axes apart: one on X, two on Y.
            cuboid(s, o + v(9.4, -2., 14.), [1., 4., 2.], Do::Cut(&body))?;
            for z in [10.0, 14.0] {
                cuboid(s, o + v(-2., 9.4, z), [4., 1., 2.], Do::Cut(&body))?;
            }
            (vec![body], "20 mm each way; one notch marks +X, two mark +Y. Measure and set your steps or shrinkage")
        }
    };
    made(note.to_string(), &bodies)
}
