//! Hardware, and things for the home.

use super::*;

pub(super) fn bolt(s: &mut Session, a: &Args) -> Result<Value> {
    let o = a.at();
    let f = a.fastener()?;
    let (l, d, c) = (a.num("length"), f.nominal, a.num("clearance"));
    let head = d * 0.65;
    let mut bodies = Vec::new();
    let bolt = match a.text("head") {
        "knob" => {
            let b = cylinder(s, o, UP, d * 2.6, head * 1.4, Do::New("Bolt"))?;
            for k in 0..8 {
                let (sn, cs) = (f64::from(k) * std::f64::consts::TAU / 8.0).sin_cos();
                cylinder(s, o + v(d * 1.45 * cs, d * 1.45 * sn, -0.1), UP, d * 0.7, head * 1.4 + 0.2, Do::Cut(&b))?;
            }
            b
        }
        "none" => cylinder(s, o, UP, d - c * 0.5, 1.0, Do::New("Bolt"))?,
        _ => prism(s, Flat::level(o), &hexagon(f.nut[0], [0., 0.]), head, Do::New("Bolt"))?,
    };
    let base = match a.text("head") {
        "knob" => head * 1.4,
        "none" => 1.0,
        _ => head,
    };
    // The shank is the standard thread's, shrunk by the clearance.
    let shank = cylinder(s, o + v(0., 0., base + 40.0), UP, d, l, Do::New("Shank"))?;
    s.execute("solid.thread", &json!({"face": [o.x + d * 0.5, o.y, o.z + base + 40.0 + l * 0.5], "designation": f.size, "modeled": true}))?;
    let k = (d - c * 0.5) / d;
    let foot = [o.x, o.y, o.z + base + 40.0];
    s.execute("solid.scale", &json!({"bodies": [shank], "factors": [k, k, 1.0], "origin": foot}))?;
    s.execute("solid.move", &json!({"bodies": [shank], "translate": [mm(0.0), mm(0.0), mm(-40.0 - 0.2)]}))?;
    s.execute("solid.combine", &json!({"target": bolt, "tools": [shank], "operation": "join"}))?;
    bodies.push(bolt);
    if a.flag("nut") {
        let at = o + v(d * 3.5, 0.0, 0.0);
        let h = f.nut[1] * 1.2;
        let nut = prism(s, Flat::level(at), &hexagon(f.nut[0] + 1.0, [0., 0.]), h, Do::New("Nut"))?;
        // The same thread, grown by the clearance, is what the nut is cut with.
        let tap = cylinder(s, at + v(0., 0., 60.0), UP, d, h + 4.0, Do::New("Tap"))?;
        s.execute("solid.thread", &json!({"face": [at.x + d * 0.5, at.y, at.z + 60.0 + h * 0.5], "designation": f.size, "modeled": true}))?;
        let g = (d + c * 0.5) / d;
        s.execute("solid.scale", &json!({"bodies": [tap], "factors": [g, g, 1.0], "origin": [at.x, at.y, at.z + 60.0]}))?;
        s.execute("solid.move", &json!({"bodies": [tap], "translate": [mm(0.0), mm(0.0), mm(-62.0)]}))?;
        s.execute("solid.combine", &json!({"target": nut, "tools": [tap], "operation": "cut"}))?;
        bodies.push(nut);
    }
    made(format!("{} × {l} bolt{}", f.size, if a.flag("nut") { " and nut" } else { "" }), &bodies)
}

pub(super) fn bracket(s: &mut Session, a: &Args) -> Result<Value> {
    let o = a.at();
    let (la, lb, w, t) = (a.num("a"), a.num("b"), a.num("width"), a.num("thickness"));
    let f = a.fastener()?;
    let d = f.clearance + 0.2;
    let body = cuboid(s, o, [la, w, t], Do::New("Bracket"))?;
    cuboid(s, o, [t, w, lb], Do::Join(&body))?;
    let g = a.int("gussets");
    let gt = a.num("gusset");
    let reach = (la.min(lb) - t) * 0.75;
    if g as f64 * gt > w {
        return Err(a.fail("the gussets are wider than the bracket"));
    }
    for i in 0..g {
        let y = if g == 1 { (w - gt) * 0.5 } else { (w - gt) * i as f64 / (g as f64 - 1.0) };
        // Drawn in XZ; its normal is -Y, so it starts at the far side.
        let flat = Flat { origin: o + v(t - 0.1, y + gt, t - 0.1), x: v(1., 0., 0.), y: UP };
        prism(s, flat, &[[0., 0.], [reach, 0.], [0., reach]], gt, Do::Join(&body))?;
    }
    let n = a.int("holes");
    for i in 0..n {
        let along = |l: f64| t + (l - t) * (i as f64 + 1.0) / (n as f64 + 1.0);
        // Beside a middle gusset, not through it.
        let side = if g % 2 == 1 { if i % 2 == 0 { w * 0.25 } else { w * 0.75 } } else { w * 0.5 };
        cylinder(s, o + v(along(la), side, -0.1), UP, d, t + 0.2, Do::Cut(&body))?;
        cylinder(s, o + v(-0.1, side, along(lb)), v(1., 0., 0.), d, t + 0.2, Do::Cut(&body))?;
    }
    made(format!("{la} × {lb} × {w} mm bracket"), &[body])
}

pub(super) fn vase(s: &mut Session, a: &Args) -> Result<Value> {
    let o = a.at();
    let (h, t) = (a.num("height"), a.num("wall"));
    let (rb, rm, rt) = (a.num("bottom") * 0.5, a.num("middle") * 0.5, a.num("top") * 0.5);
    let (kind, n, depth, twist) = (a.text("pattern").to_string(), a.num("count"), a.num("depth"), a.num("twist").to_radians());
    let hole = if a.num("drain") > 0.0 { a.num("drain") * 0.5 } else { 0.02 };
    if rb.min(rm).min(rt) - t - depth < hole + 1.0 {
        return Err(a.fail("too narrow for its wall and pattern"));
    }
    let side = |z: f64| {
        let k = (z / h).clamp(0.0, 1.0);
        // The parabola through bottom, middle and top.
        rb * (1.0 - k) * (1.0 - 2.0 * k) + 4.0 * rm * k * (1.0 - k) + rt * k * (2.0 * k - 1.0)
    };
    let steps = ((h / 1.2).ceil() as usize).clamp(8, 300);
    // Up the outside, over the rim, down the inside and in under the floor to the drain.
    let mut profile: Vec<[f64; 2]> = vec![[hole, 0.0]];
    let mut outer = vec![false];
    for i in 0..=steps {
        let z = h * i as f64 / steps as f64;
        profile.push([side(z), z]);
        outer.push(true);
    }
    for i in (0..=steps).rev() {
        let z = t + (h - t) * i as f64 / steps as f64;
        profile.push([side(z) - t, z]);
        outer.push(true);
    }
    profile.push([hole, t]);
    outer.push(false);
    let around = ((n as usize) * 12).clamp(96, 720);
    let wave = |ang: f64, z: f64| {
        let q = n * (ang + twist * z / h);
        depth
            * match kind.as_str() {
                "ribs" => q.sin().abs() - 0.5,
                "waves" => (q.sin() + (z / h * n * 3.0).sin()) * 0.35,
                "petals" => q.sin() * 0.5 * (z / h),
                "facets" => {
                    let u = (q / std::f64::consts::TAU).rem_euclid(1.0) - 0.5;
                    -(2.0 * u).abs() * 0.5 + 0.25
                }
                _ => 0.0,
            }
    };
    let m = sdf::turned(&profile, around, |i, ang, z| {
        let r = profile.get(i).map_or(hole, |p| p[0]);
        if outer.get(i).copied().unwrap_or(false) { r + wave(ang, z.max(0.0)) } else { r }
    });
    let mut whole = vec![(m, o, "Vase")];
    if a.flag("saucer") {
        let r = rb + depth + 8.0;
        let ring = [[0.02, 0.0], [r, 0.0], [r + 4.0, 12.0], [r + 4.0 - t, 12.0], [r - t * 0.8, t], [0.02, t]];
        whole.push((sdf::turned(&ring, 128, |i, _, _| ring.get(i).map_or(0.02, |p| p[0])), o + v(rb.max(rm).max(rt) + r + depth + 20.0, 0., 0.), "Saucer"));
    }
    let mut bodies = Vec::new();
    for (mut m, at, name) in whole {
        for p in &mut m.positions {
            *p = *p + at;
        }
        bodies.extend(add_mesh_body(s, name, &m)?);
    }
    let what = if a.num("drain") > 0.0 { "Planter" } else { "Vase" };
    made(format!("{what}, {h:.0} mm tall, {kind}"), &bodies)
}

/// Unit face normals of a regular die.
pub(super) fn faces(sides: usize) -> Vec<Vec3> {
    let phi = (1.0 + 5f64.sqrt()) / 2.0;
    let mut out: Vec<Vec3> = Vec::new();
    match sides {
        4 => out.extend([v(1., 1., 1.), v(1., -1., -1.), v(-1., 1., -1.), v(-1., -1., 1.)]),
        6 => out.extend([v(1., 0., 0.), v(0., 1., 0.), v(0., 0., 1.), v(0., 0., -1.), v(0., -1., 0.), v(-1., 0., 0.)]),
        8 => {
            for i in 0..8 {
                let g = |b: i32| if i >> b & 1 == 0 { 1.0 } else { -1.0 };
                out.push(v(g(0), g(1), g(2)));
            }
        }
        12 => {
            for (x, y) in [(1.0, 1.0), (1.0, -1.0), (-1.0, 1.0), (-1.0, -1.0)] {
                out.extend([v(0., x, y * phi), v(x, y * phi, 0.), v(x * phi, 0., y)]);
            }
        }
        _ => {
            for i in 0..8 {
                let g = |b: i32| if i >> b & 1 == 0 { 1.0 } else { -1.0 };
                out.push(v(g(0), g(1), g(2)));
            }
            for (x, y) in [(1.0, 1.0), (1.0, -1.0), (-1.0, 1.0), (-1.0, -1.0)] {
                out.extend([v(0., x / phi, y * phi), v(x / phi, y * phi, 0.), v(x * phi, 0., y / phi)]);
            }
        }
    }
    let mut out: Vec<Vec3> = out.into_iter().filter_map(|n| n.normalized()).collect();
    // Opposite faces sit opposite in the list, so their numbers add up alike.
    if sides > 4 {
        let mut half: Vec<Vec3> = Vec::new();
        for n in &out {
            if !half.iter().any(|h| h.dot(*n) < -0.999) {
                half.push(*n);
            }
        }
        out = half.clone();
        out.extend(half.iter().rev().map(|n| *n * -1.0));
    }
    out
}

pub(super) fn dice(s: &mut Session, a: &Args) -> Result<Value> {
    let o = a.at();
    let sides: usize = a.text("sides").parse().unwrap_or(6);
    let (size, round, depth) = (a.num("size"), a.num("round"), a.num("depth"));
    // A four-sided die has no opposite faces: its size is its height, four times its inradius.
    let r = if sides == 4 { size * 0.25 } else { size * 0.5 };
    if round > r * 0.6 || depth > r * 0.4 {
        return Err(a.fail("too much rounding or depth for that size"));
    }
    let normals = faces(sides);
    let reach = match sides {
        4 => r * 3.1,
        6 => r * 1.75,
        8 => r * 1.75,
        _ => r * 1.3,
    };
    let b: rybocraft_geom::Aabb3 = [v(-reach, -reach, -reach), v(reach, reach, reach)].into_iter().collect();
    let mut warnings = Vec::new();
    let h = detail(a, &b, &mut warnings);
    // Each mark in its face's own frame: across, up, out.
    let frames: Vec<[Vec3; 3]> = normals
        .iter()
        .map(|n| {
            let hint = if n.z.abs() > 0.9 { v(0., 1., 0.) } else { UP };
            let x = hint.cross(*n).normalized().unwrap_or(v(1., 0., 0.));
            [x, n.cross(x), *n]
        })
        .collect();
    let mut marks: Vec<Option<Source>> = Vec::new();
    if sides != 6 {
        let face = match sides {
            4 => r * 1.1,
            8 => r * 0.62,
            12 => r * 0.5,
            _ => r * 0.36,
        };
        for i in 0..sides {
            // Told apart when the die is read either way up.
            let text = match i + 1 {
                6 | 9 if sides > 8 => format!("{}.", i + 1),
                n => n.to_string(),
            };
            let size = face * if text.len() > 1 { 0.85 } else { 1.0 };
            marks.push(Some(Source::new(letter_mesh(s, a, &text, size, 4.0 * depth + 2.0)?)));
        }
    }
    let fields: Vec<_> = marks.iter().map(|m| m.as_ref().map(Source::field)).collect();
    let pips = |n: usize| -> Vec<[f64; 2]> {
        let q = r * 0.5;
        let mut p = Vec::new();
        if n % 2 == 1 {
            p.push([0., 0.]);
        }
        if n >= 2 {
            p.extend([[-q, q], [q, -q]]);
        }
        if n >= 4 {
            p.extend([[q, q], [-q, -q]]);
        }
        if n == 6 {
            p.extend([[-q, 0.], [q, 0.]]);
        }
        p
    };
    let spots: Vec<Vec<[f64; 2]>> = if sides == 6 { (1..=6).map(pips).collect() } else { Vec::new() };
    let pip = r * 0.19;
    let m = sdf::polygonize(b, h, |p| {
        // Rounded by a smooth maximum of the face planes.
        let k = (round * 0.6).max(1e-6);
        let mut d = f64::NEG_INFINITY;
        for n in &normals {
            let x = p.dot(*n) - r;
            d = if round < 1e-6 || d == f64::NEG_INFINITY {
                d.max(x)
            } else {
                let t = (k - (d - x).abs()).max(0.0) / k;
                d.max(x) + t * t * k * 0.25
            };
        }
        if d > h * 3.0 || d < -(depth + 3.0 * h) {
            return d;
        }
        for (i, f) in frames.iter().enumerate() {
            let out = p.dot(f[2]) - r;
            if out.abs() > depth + 2.0 * h {
                continue;
            }
            let (x, y) = (p.dot(f[0]), p.dot(f[1]));
            if let Some(list) = spots.get(i) {
                for c in list {
                    // A shallow bowl.
                    let ball = pip * pip / (2.0 * depth.min(pip)) + depth.min(pip) * 0.5;
                    d = d.max(-sdf::sphere(v(x, y, out), v(c[0], c[1], ball - depth.min(pip)), ball));
                }
            } else if let Some(Some(field)) = fields.get(i) {
                d = d.max(-field.at(v(x, y, out + depth), 2.0 * depth + 2.0));
            }
        }
        d
    })
    .map_err(|e| a.fail(e))?;
    let low = m.bounds().min.z;
    let mut m = m;
    for p in &mut m.positions {
        *p = *p + o - UP * low;
    }
    mesh_made(s, "Die", &m, format!("{sides}-sided die, {size} mm"), warnings)
}

pub(super) fn marble(s: &mut Session, a: &Args) -> Result<Value> {
    let o = a.at();
    let (md, l, t, c) = (a.num("marble"), a.num("length"), a.num("wall"), a.num("clearance"));
    let bore = md + 2.0;
    let w = bore + 2.0 * t;
    let h = bore * 0.5 + t + md * 0.15;
    let groove = |s: &mut Session, from: Vec3, dir: Vec3, len: f64, body: &str| cylinder(s, from + UP * h, dir, bore, len, Do::Cut(body)).map(|_| ());
    // A dovetail key on one end, its socket on the other.
    let key = |x: f64, g: f64| vec![[x, -w * 0.14 - g], [x + 5.0 + g, -w * 0.22 - g], [x + 5.0 + g, w * 0.22 + g], [x, w * 0.14 + g]];
    let kh = h - bore * 0.5 - 0.6;
    if kh < 0.8 {
        return Err(a.fail("the wall is too thin for the joints"));
    }
    match a.text("kind") {
        "support" => {
            let body = cylinder(s, o, UP, w, 3.0, Do::New("Marble support"))?;
            cylinder(s, o, UP, 10.0, l, Do::Join(&body))?;
            cuboid(s, o + v(-8.0, -w * 0.5 - t - c, l - 0.1), [16.0, w + 2.0 * (t + c), t + h * 0.6], Do::Join(&body))?;
            cuboid(s, o + v(-8.1, -w * 0.5 - c, l - 0.1 + t), [16.2, w + 2.0 * c, h], Do::Cut(&body))?;
            made(format!("Track support, {l} mm"), &[body])
        }
        "curve" => {
            let (r, ang) = (a.num("radius"), a.num("angle").to_radians());
            if r < w * 0.5 + 5.0 {
                return Err(a.fail("the curve is too tight for its width"));
            }
            let n = ((ang.to_degrees() / 3.0).ceil() as usize).max(6);
            let arc = |rad: f64, back: bool| -> Vec<[f64; 2]> {
                (0..=n).map(|i| if back { n - i } else { i }).map(|i| ang * i as f64 / n as f64).map(|t| [rad * t.sin(), r - rad * t.cos()]).collect()
            };
            let mut pts = arc(r + w * 0.5, false);
            pts.extend(arc(r - w * 0.5, true));
            let body = prism(s, Flat::level(o), &pts, h, Do::New("Marble curve"))?;
            // The groove is the marble's path: a ring cut about the curve's centre.
            let before = names(s)?;
            s.execute("solid.torus", &json!({"major": mm(r), "minor": mm(bore * 0.5), "center": xyz(o + v(0., r, h)), "operation": "new", "body_name": "Groove"}))?;
            let ring = new_since(s, &before)?;
            s.execute("solid.combine", &json!({"target": body, "tools": ring, "operation": "cut"}))?;
            let mut p = key(0.0, 0.0);
            for q in &mut p {
                // Off the far end, turned with it.
                let (x, y) = (q[0] - 0.1, q[1]);
                *q = [r * ang.sin() + x * ang.cos() - y * ang.sin(), r - r * ang.cos() + x * ang.sin() + y * ang.cos()];
            }
            prism(s, Flat::level(o), &p, kh, Do::Join(&body))?;
            prism(s, Flat::level(o - UP * 0.1), &key(-0.1, c), kh + 0.3, Do::Cut(&body))?;
            made(format!("Curve, {r} mm radius"), &[body])
        }
        _ => {
            let body = cuboid(s, o + v(0., -w * 0.5, 0.), [l, w, h], Do::New("Marble track"))?;
            groove(s, o - v(0.1, 0., 0.), v(1., 0., 0.), l + 0.2, &body)?;
            prism(s, Flat::level(o), &key(l - 0.1, 0.0), kh, Do::Join(&body))?;
            prism(s, Flat::level(o - UP * 0.1), &key(-0.1, c), kh + 0.3, Do::Cut(&body))?;
            made(format!("Straight track, {l} mm"), &[body])
        }
    }
}

pub(super) fn stand(s: &mut Session, a: &Args) -> Result<Value> {
    let o = a.at();
    let (w, dev, h, lean, t) = (a.num("width"), a.num("device"), a.num("height"), a.num("angle").to_radians(), a.num("thickness"));
    let (sn, cs) = lean.sin_cos();
    // Seen from the side: a foot, a tray tilted with the back, and a lip in front.
    let back = [h * sn, h * cs];
    let foot = (h * sn + dev + 2.0 * t).max(h * 0.55);
    let y0 = dev + 2.0 * t;
    let pts = [
        [0.0, 0.0],
        [foot + y0, 0.0],
        [foot + y0, t],
        [y0 + t * 1.5, t],
        [y0 + back[0] + t * cs, t * 2.0 + back[1]],
        [y0 + back[0], t * 2.0 + back[1]],
        [y0 - t * 0.5, t * 2.0],
        [t, t * 2.0],
        [t, t * 2.0 + dev * 0.8],
        [0.0, t * 2.0 + dev * 0.8],
    ];
    let side = Flat { origin: o + v(-w * 0.5, 0., 0.), x: v(0., 1., 0.), y: UP };
    let body = prism(s, side, &pts, w, Do::New("Device stand"))?;
    let cable = a.num("cable").min(w * 0.6);
    if cable > 0.0 {
        cuboid(s, o + v(-cable * 0.5, -0.1, t * 0.8), [cable, dev + 2.0 * t * 0.75, dev + t * 4.0], Do::Cut(&body))?;
    }
    made(format!("Stand leaning {:.0}°", a.num("angle")), &[body])
}
