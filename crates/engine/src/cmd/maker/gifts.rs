//! Names, letters and small presents.

use super::*;

pub(super) fn text(s: &mut Session, a: &Args) -> Result<Value> {
    let o = a.at();
    let t = words(a, "text")?;
    let standing = a.flag("standing");
    let flat = if standing { Flat { origin: o + UP * a.num("size"), x: v(1., 0., 0.), y: UP } } else { Flat::level(o) };
    let (mut bodies, b) = letters(s, a, flat, &t, a.num("size"), a.num("depth"), "Text")?;
    if a.text("base") != "none" {
        let (m, bd) = (a.num("margin"), a.num("base_depth"));
        let (c, size) = (b.center(), b.size());
        let r = |hh: f64| if a.text("base") == "rounded" { (m + hh * 0.25).min(hh * 0.5 + m - 0.1) } else { 0.0 };
        let (frame, wh) = if standing {
            (Flat { origin: v(c.x, b.max.y + bd - 0.1, c.z), x: v(1., 0., 0.), y: UP }, [size.x + 2.0 * m, size.z + 2.0 * m])
        } else {
            (Flat::level(v(c.x, c.y, o.z - bd + 0.1)), [size.x + 2.0 * m, size.y + 2.0 * m])
        };
        bodies.push(rounded(s, frame, wh, r(wh[1] - 2.0 * m), bd, 0.0, Do::New("Text backing"))?);
    }
    made(format!("“{}”, {:.0} mm wide", t.replace('\n', " "), b.size().x), &bodies)
}

pub(super) fn keychain(s: &mut Session, a: &Args) -> Result<Value> {
    let o = a.at();
    let t = words(a, "text")?;
    let (base, m, hole) = (a.num("base"), a.num("margin"), a.num("hole"));
    let (mut bodies, b) = letters(s, a, Flat::level(o + UP * (base - 0.05)), &t, a.num("size"), a.num("raised") + 0.05, "Letters")?;
    let (w, h) = (b.size().x + 2.0 * m, b.size().y + 2.0 * m);
    let c = v(b.center().x, b.center().y, o.z);
    let tag = rounded(s, Flat::level(c), [w, h], h * 0.35, base, 0.0, Do::New("Keychain tag"))?;
    let ring = v(c.x - w * 0.5 - hole * 0.15, c.y, o.z);
    cylinder(s, ring, UP, hole + 5.0, base, Do::Join(&tag))?;
    cylinder(s, ring - UP * 0.1, UP, hole, base + 0.2, Do::Cut(&tag))?;
    bodies.push(tag);
    made(format!("“{t}” keychain, {w:.0} × {h:.0} mm"), &bodies)
}

pub(super) fn nameplate(s: &mut Session, a: &Args) -> Result<Value> {
    let o = a.at();
    let t = words(a, "text")?;
    let (m, raised) = (a.num("margin"), a.num("raised"));
    let probe = letter_mesh(s, a, &t, a.num("size"), 1.0)?.bounds().size();
    let (w, h) = (probe.x + 2.0 * m, probe.y + 2.0 * m);
    if a.text("style") == "wall" {
        let th = a.num("thickness");
        let plate = rounded(s, Flat::level(o), [w + 16.0, h], (h * 0.2).min(10.0), th, 0.0, Do::New("Nameplate"))?;
        for x in [-1.0, 1.0] {
            cylinder(s, o + v(x * (w * 0.5 + 2.0), 0.0, -0.1), UP, 4.4, th + 0.2, Do::Cut(&plate))?;
        }
        let (mut bodies, _) = letters(s, a, Flat::level(o + UP * (th - 0.05)), &t, a.num("size"), raised + 0.05, "Letters")?;
        bodies.push(plate);
        return made(format!("“{t}” wall plate"), &bodies);
    }
    // A wedge leaning back 20°, its face carrying the letters.
    let lean = 20f64.to_radians();
    let (dy, dz) = (h * lean.sin(), h * lean.cos());
    let depth = dy + 12.0;
    let side = Flat { origin: o + v(-w * 0.5, 0.0, 0.0), x: v(0., 1., 0.), y: UP };
    let plate = prism(s, side, &[[0., 0.], [depth, 0.], [depth, 3.0], [dy + 4.0, dz], [dy, dz]], w, Do::New("Nameplate"))?;
    let up = v(0.0, lean.sin(), lean.cos());
    let out = v(0.0, -lean.cos(), lean.sin());
    let flat = Flat { origin: o + v(0.0, dy * 0.5, dz * 0.5) - out * 0.1, x: v(1., 0., 0.), y: up };
    let (mut bodies, _) = letters(s, a, flat, &t, a.num("size"), raised + 0.1, "Letters")?;
    bodies.push(plate);
    made(format!("“{t}” desk nameplate, {w:.0} mm wide"), &bodies)
}

pub(super) fn cake_topper(s: &mut Session, a: &Args) -> Result<Value> {
    let o = a.at();
    let t = words(a, "text")?;
    let (size, d) = (a.num("size"), a.num("depth"));
    let (bodies, b) = letters(s, a, Flat::level(o), &t, size, d, "Topper")?;
    // A bar through the foot of every letter ties them together.
    let bar = (size * 0.14).max(2.4);
    let lowest = o.y - size * (0.75 * (t.lines().count().max(1) as f64 - 1.0) + 0.5);
    let first = cuboid(s, v(b.min.x, lowest - bar * 0.6, o.z), [b.size().x, bar, d], Do::New("Topper bar"))?;
    let n = a.int("stakes");
    let len = a.num("stake");
    for i in 0..n {
        let x = b.min.x + b.size().x * (i as f64 + 1.0) / (n as f64 + 1.0);
        let pts = [[x - 2.0, lowest], [x - 2.0, lowest - len + 6.0], [x, lowest - len], [x + 2.0, lowest - len + 6.0], [x + 2.0, lowest]];
        prism(s, Flat::level(v(0., 0., o.z)), &pts, d, Do::Join(&first))?;
    }
    s.execute("solid.combine", &json!({"target": first, "tools": bodies, "operation": "join"}))?;
    let mut warnings = Vec::new();
    if t.lines().count() > 1 {
        warnings.push("only the last line meets the bar: use a script font or join the lines with a shape");
    }
    Ok(json!({"summary": format!("“{}” topper, {:.0} mm wide", t.replace('\n', " "), b.size().x), "created_bodies": [first], "warnings": warnings}))
}

pub(super) fn balloon(s: &mut Session, a: &Args) -> Result<Value> {
    let o = a.at();
    let t = words(a, "text")?;
    let puff = a.num("puff");
    // Deep letters, so the distance at mid-height is to the sides alone.
    let deep = 6.0 * puff + 20.0;
    let src = Source::new(letter_mesh(s, a, &t, a.num("size"), deep)?);
    let f = src.field();
    let reach = a.num("size");
    let mut warnings = Vec::new();
    let mut b = f.bounds;
    (b.min.z, b.max.z) = (if a.flag("flat") { 0.0 } else { -puff }, puff);
    let h = detail(a, &b, &mut warnings);
    // The widest stroke sets how round the letters get.
    let widest = {
        let mut most: f64 = 0.1;
        for i in 0..160 {
            for j in 0..60 {
                let p = v(b.min.x + b.size().x * (f64::from(i) + 0.5) / 160.0, b.min.y + b.size().y * (f64::from(j) + 0.5) / 60.0, deep * 0.5);
                most = most.max(-f.at(p, reach));
            }
        }
        most
    };
    let flat = a.flag("flat");
    let m = sdf::polygonize(b, h, |p| {
        let d = f.at(v(p.x, p.y, deep * 0.5), reach);
        let k = (-d / widest).clamp(0.0, 1.0);
        let height = puff * (1.0 - (1.0 - k).powi(2)).sqrt();
        let over = p.z.abs() - height;
        let body = if d > 0.0 { d.max(over) } else { over.max(d) };
        if flat { body.max(-p.z) } else { body }
    })
    .map_err(|e| a.fail(e))?;
    let m = shifted(m, o + if flat { v(0., 0., 0.) } else { UP * puff });
    mesh_made(s, "Balloon letters", &m, format!("“{t}” balloons, {:.0} mm wide", b.size().x), warnings)
}

fn shifted(mut m: PrintMesh, by: Vec3) -> PrintMesh {
    for p in &mut m.positions {
        *p = *p + by;
    }
    m
}

pub(super) fn ornament(s: &mut Session, a: &Args) -> Result<Value> {
    let o = a.at();
    let (r, t, n, twist, sq) = (a.num("diameter") * 0.5, a.num("thickness") * 0.5, a.num("strands"), a.num("twist"), a.num("squash") / 100.0);
    let loop_ = a.flag("loop");
    let pi = std::f64::consts::PI;
    let rise = r * sq;
    let mut b: rybocraft_geom::Aabb3 = [v(-r - t, -r - t, -rise - t), v(r + t, r + t, rise + t)].into_iter().collect();
    let (lr, lt) = ((r * 0.1).max(2.5), t.max(1.0));
    if loop_ {
        b.max.z += 2.0 * (lr + lt);
    }
    let mut warnings = Vec::new();
    let h = detail(a, &b, &mut warnings);
    let m = sdf::polygonize(b, h, |p| {
        let q = v(p.x, p.y, p.z / sq);
        let rho = q.len().max(1e-9);
        let polar = (q.z / rho).clamp(-1.0, 1.0).acos();
        let around = q.y.atan2(q.x);
        let ring = r * polar.sin();
        let mut d = f64::INFINITY;
        for dir in [1.0, -1.0] {
            let phase = (around - dir * twist * 2.0 * pi * polar / pi) * n / (2.0 * pi);
            let off = (phase - phase.round()) * 2.0 * pi / n * ring;
            // Across the strand, not around the ring.
            let slope = 2.0 * twist * polar.sin();
            let off = off / (1.0 + slope * slope).sqrt();
            d = d.min(off.hypot((rho - r) * sq.min(1.0)) - t);
            if twist == 0.0 {
                break;
            }
        }
        if twist == 0.0 {
            // Straight ribs need hoops to hold together.
            for k in 1..4 {
                let at = pi * f64::from(k) / 4.0;
                d = d.min((r * (polar - at)).hypot(rho - r) - t);
            }
        }
        // Caps where the strands meet.
        for z in [1.0, -1.0] {
            d = d.min(sdf::sphere(p, v(0., 0., z * rise), t * 1.8));
        }
        if loop_ {
            d = d.min(sdf::torus(p, v(0., 0., rise + lr + lt * 0.5), v(0., 1., 0.), lr, lt));
        }
        d
    })
    .map_err(|e| a.fail(e))?;
    mesh_made(s, "Ornament", &shifted(m, o + UP * (rise + t)), format!("{:.0} mm woven ornament", 2.0 * r), warnings)
}

pub(super) fn snowflake(s: &mut Session, a: &Args) -> Result<Value> {
    let o = a.at();
    let (r, w, th, arms) = (a.num("diameter") * 0.5, a.num("line") * 0.5, a.num("thickness"), a.num("arms"));
    let mut seed = a.num("seed") as u64 * 2_654_435_761 + 12345;
    let mut rnd = || {
        seed = seed.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1_442_695_040_888_963_407);
        ((seed >> 33) % 10_000) as f64 / 10_000.0
    };
    // One half of one arm: the stem along X and its branches, mirrored and repeated by folding.
    let mut lines: Vec<([f64; 2], [f64; 2])> = vec![([0., 0.], [r - w, 0.])];
    let n = a.int("branches");
    let sector = std::f64::consts::PI / arms;
    for i in 0..n {
        let at = r * (0.22 + 0.68 * (i as f64 + 0.3 + 0.5 * rnd()) / n as f64);
        let angle = (45.0 + 25.0 * rnd()).to_radians();
        let most = (at * sector.sin() / angle.sin()).min((r - at) * 1.1);
        let l = most * (0.45 + 0.5 * rnd());
        let end = [at + l * angle.cos(), l * angle.sin()];
        lines.push(([at, 0.], end));
        if rnd() > 0.45 {
            let k = 0.55;
            let mid = [at + l * k * angle.cos(), l * k * angle.sin()];
            lines.push((mid, [mid[0] + l * 0.35, mid[1]]));
        }
    }
    if rnd() > 0.4 {
        let c = r * (0.12 + 0.15 * rnd());
        lines.push(([c, 0.], [c * sector.cos(), c * sector.sin()]));
    }
    let hang = a.flag("loop");
    let lr = (w * 1.6).max(2.0);
    let mut b: rybocraft_geom::Aabb3 = [v(-r - w, -r - w, 0.), v(r + w, r + w, th)].into_iter().collect();
    if hang {
        b.max.x += 2.0 * lr + w;
    }
    let mut warnings = Vec::new();
    let h = detail(a, &b, &mut warnings).min(th * 0.5);
    let m = sdf::polygonize(b, h, |p| {
        let (rad, ang) = (p.x.hypot(p.y), p.y.atan2(p.x));
        let turn = 2.0 * sector;
        let fold = (ang - (ang / turn).round() * turn).abs();
        let q = v(rad * fold.cos(), rad * fold.sin(), 0.0);
        let mut d = f64::INFINITY;
        for (x, y) in &lines {
            d = d.min(sdf::capsule(q, v(x[0], x[1], 0.), v(y[0], y[1], 0.), w));
        }
        if hang {
            d = d.min(((v(p.x, p.y, 0.) - v(r + lr, 0., 0.)).len() - lr).abs() - w);
        }
        d.max(p.z - th).max(-p.z)
    })
    .map_err(|e| a.fail(e))?;
    mesh_made(s, "Snowflake", &shifted(m, o), format!("{:.0} mm snowflake, {arms:.0} arms", 2.0 * r), warnings)
}

pub(super) fn coaster(s: &mut Session, a: &Args) -> Result<Value> {
    let o = a.at();
    let (d, t, rim) = (a.num("diameter"), a.num("thickness"), a.num("rim"));
    let edge = 3.0f64.min(d * 0.1);
    let plate = |s: &mut Session, z: f64, d: f64, h: f64, what: Do| match a.text("shape") {
        "square" => rounded(s, Flat::level(o + UP * z), [d, d], d * 0.1, h, 0.0, what),
        "hexagon" => prism(s, Flat::level(o + UP * z), &hexagon(d, [0., 0.]), h, what),
        _ => cylinder(s, o + UP * z, UP, d, h, what),
    };
    let body = plate(s, 0.0, d, t + rim, Do::New("Coaster"))?;
    if rim > 0.0 {
        plate(s, t, d - 2.0 * edge, rim + 0.1, Do::Cut(&body))?;
    }
    let mut bodies = vec![body];
    if !a.text("text").trim().is_empty() {
        let w = words(a, "text")?;
        bodies.extend(letters(s, a, Flat::level(o + UP * (t - 0.05)), &w, a.num("size"), rim.max(0.8) + 0.05, "Letters")?.0);
    }
    made(format!("{d} mm {} coaster", a.text("shape")), &bodies)
}

pub(super) fn qr(s: &mut Session, a: &Args) -> Result<Value> {
    let o = a.at();
    let t = words(a, "text")?;
    let code = qrcodegen::QrCode::encode_text(&t, qrcodegen::QrCodeEcc::Medium).map_err(|_| a.fail("that is too much for a QR code"))?;
    let n = code.size();
    let (size, base, raised) = (a.num("size"), a.num("base"), a.num("raised"));
    let cell = size / f64::from(n + 8);
    let plate = rounded(s, Flat::level(o), [size, size], size * 0.06, base, 0.0, Do::New("QR plate"))?;
    if a.num("hole") > 0.0 {
        let hd = a.num("hole");
        let at = o + v(0.0, size * 0.5 + hd * 0.2, 0.0);
        cylinder(s, at, UP, hd + 6.0, base, Do::Join(&plate))?;
        cylinder(s, at - UP * 0.1, UP, hd, base + 0.2, Do::Cut(&plate))?;
    }
    // One thin box per run of dark cells, a hair apart from its neighbours' edges in the mesh but
    // overlapping in print.
    let mut m = PrintMesh::default();
    let z = (o.z + base - 0.02, o.z + base + raised);
    let grow = cell * 0.01;
    for y in 0..n {
        let mut x = 0;
        while x < n {
            if !code.get_module(x, y) {
                x += 1;
                continue;
            }
            let from = x;
            while x < n && code.get_module(x, y) {
                x += 1;
            }
            let x0 = o.x - size * 0.5 + f64::from(from + 4) * cell - grow;
            let x1 = o.x - size * 0.5 + f64::from(x + 4) * cell + grow;
            let y1 = o.y + size * 0.5 - f64::from(y + 4) * cell + grow;
            block(&mut m, v(x0, y1 - cell - 2.0 * grow, z.0), v(x1, y1, z.1));
        }
    }
    let mut bodies = add_mesh_body(s, "QR code", &m)?;
    bodies.push(plate);
    let warnings = if cell < 0.8 { vec!["the squares are under 0.8 mm: make it larger or the text shorter"] } else { Vec::new() };
    Ok(json!({"summary": format!("QR code, {n} × {n} squares of {cell:.2} mm"), "created_bodies": bodies, "warnings": warnings}))
}

pub(super) fn block(m: &mut PrintMesh, lo: Vec3, hi: Vec3) {
    let p: Vec<Vec3> =
        (0..8).map(|i| v(if i & 1 == 0 { lo.x } else { hi.x }, if i & 2 == 0 { lo.y } else { hi.y }, if i & 4 == 0 { lo.z } else { hi.z })).collect();
    let q = [[0, 2, 3, 1], [4, 5, 7, 6], [0, 1, 5, 4], [2, 6, 7, 3], [0, 4, 6, 2], [1, 3, 7, 5]];
    let t: Vec<[u32; 3]> = q.iter().flat_map(|f| [[f[0], f[1], f[2]], [f[0], f[2], f[3]]]).collect();
    m.append(&p, &t);
}
