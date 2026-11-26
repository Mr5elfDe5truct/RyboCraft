//! Editing models, imported meshes above all. Exact solids are edited exactly where the kernel
//! can; everything else goes through a signed distance and comes back as a clean mesh.

use super::*;

/// Put an edited mesh in the place of the body it came from.
fn replace(s: &mut Session, body: &str, m: &PrintMesh, summary: String, warnings: Vec<String>) -> Result<Value> {
    remove(s, &[body.to_string()])?;
    mesh_made(s, body, m, summary, warnings)
}

fn unit(a: &Args) -> (usize, Vec3) {
    match a.text("axis") {
        "X" => (0, v(1., 0., 0.)),
        "Y" => (1, v(0., 1., 0.)),
        _ => (2, UP),
    }
}

fn load(s: &Session, a: &Args) -> Result<(String, Source)> {
    let body = a.body(s)?;
    let m = mesh_of(s, std::slice::from_ref(&body))?;
    if !rybocraft_print3d::analysis::topology(&m).watertight() && m.volume() < 1e-6 {
        return Err(a.fail("the model is not a closed solid: repair it first"));
    }
    Ok((body, Source::new(m)))
}

pub(super) fn add_text(s: &mut Session, a: &Args) -> Result<Value> {
    let (body, p, n) = a.spot(s)?;
    let t = words(a, "text")?;
    let (size, depth) = (a.num("size"), a.num("depth"));
    let turn = a.num("angle").to_radians();
    let engrave = a.text("mode") == "engrave";
    let sink = 0.3f64.min(depth * 0.5);
    let frame = Flat::into_face(p, n * -1.0);
    // Read from outside: into_face's x runs the other way seen from there.
    let (x, y) = (frame.x * -1.0, frame.y);
    let (x, y) = (x * turn.cos() + y * turn.sin(), y * turn.cos() - x * turn.sin());
    let flat = |from: Vec3| Flat { origin: from, x, y };
    let mesh = is_mesh(s, &body)?;
    if !engrave {
        if mesh || a.flag("separate") {
            let (bodies, _) = letters(s, a, flat(p - n * sink), &t, size, depth + sink, "Letters")?;
            return made(format!("“{t}” raised on `{body}`, as bodies of their own"), &bodies);
        }
        lettering(s, a, flat(p - n * sink), &t, size, depth + sink, Do::Join(&body))?;
        return Ok(json!({"summary": format!("“{t}” raised on `{body}`"), "body": body}));
    }
    if !mesh {
        lettering(s, a, flat(p - n * depth), &t, size, depth + 0.5, Do::Cut(&body))?;
        return Ok(json!({"summary": format!("“{t}” engraved in `{body}`"), "body": body}));
    }
    let src = Source::new(mesh_of(s, std::slice::from_ref(&body))?);
    let letters = Source::new(letter_mesh(s, a, &t, size, depth + 2.0)?);
    let (f, g) = (src.field(), letters.field());
    let mut warnings = Vec::new();
    let h = detail(a, &f.bounds, &mut warnings);
    let reach = 3.0 * h + depth;
    let m = sdf::polygonize(f.bounds, h, |q| {
        let d = f.at(q, reach);
        let w = q - (p - n * depth);
        d.max(-g.at(v(w.dot(x), w.dot(y), w.dot(n)), reach))
    })
    .map_err(|e| a.fail(e))?;
    replace(s, &body, &m, format!("“{t}” engraved in `{body}`"), warnings)
}

pub(super) fn texture(s: &mut Session, a: &Args) -> Result<Value> {
    let (body, src) = load(s, a)?;
    let f = src.field();
    let (kind, k, depth, keep) = (a.text("pattern"), std::f64::consts::TAU / a.num("scale"), a.num("depth"), a.num("keep_bottom"));
    let mut b = f.bounds;
    b = b.inflated(depth);
    let mut warnings = Vec::new();
    let h = detail(a, &b, &mut warnings);
    let low = f.bounds.min.z;
    let reach = depth * 2.0 + 3.0 * h;
    let m = sdf::polygonize(b, h, |p| {
        let d = f.at(p, reach);
        let fade = if keep > 0.0 { ((p.z - low - keep) / (keep * 0.5).max(0.2)).clamp(0.0, 1.0) } else { 1.0 };
        let cut = d - 0.5 * depth * fade * sdf::pattern(kind, p * k);
        if keep > 0.0 { cut.max(low - p.z) } else { cut }
    })
    .map_err(|e| a.fail(e))?;
    replace(s, &body, &m, format!("{kind} texture on `{body}`"), warnings)
}

/// Round spots on the plane `axis = at` inside the model, with room around them, far apart.
fn spots(f: &sdf::Field, axis: usize, at: f64, r: f64, deep: f64, want: usize) -> Vec<Vec3> {
    let b = &f.bounds;
    let place = |u: f64, w: f64| match axis {
        0 => v(at, u, w),
        1 => v(u, at, w),
        _ => v(u, w, at),
    };
    let (lo, hi) = match axis {
        0 => ([b.min.y, b.min.z], [b.max.y, b.max.z]),
        1 => ([b.min.x, b.min.z], [b.max.x, b.max.z]),
        _ => ([b.min.x, b.min.y], [b.max.x, b.max.y]),
    };
    let normal = [v(1., 0., 0.), v(0., 1., 0.), UP][axis.min(2)];
    let mut good = Vec::new();
    for i in 0..28 {
        for j in 0..28 {
            let p = place(lo[0] + (hi[0] - lo[0]) * (f64::from(i) + 0.5) / 28.0, lo[1] + (hi[1] - lo[1]) * (f64::from(j) + 0.5) / 28.0);
            if [-deep, 0.0, deep].iter().all(|w| f.at(p + normal * *w, r + 1.0) < -r) {
                good.push(p);
            }
        }
    }
    let mid = place((lo[0] + hi[0]) * 0.5, (lo[1] + hi[1]) * 0.5);
    let mut out: Vec<Vec3> = Vec::new();
    while out.len() < want {
        let apart = |p: &Vec3| if out.is_empty() { p.dist(mid) } else { out.iter().map(|q| q.dist(*p)).fold(f64::INFINITY, f64::min) };
        match good.iter().max_by(|p, q| apart(p).total_cmp(&apart(q))) {
            Some(p) if out.is_empty() || apart(p) > 2.0 * r + 1.0 => {
                let p = *p;
                out.push(p);
            }
            _ => break,
        }
    }
    out
}

pub(super) fn split(s: &mut Session, a: &Args) -> Result<Value> {
    let (body, src) = load(s, a)?;
    let f = src.field();
    let (axis, u) = unit(a);
    let (lo, hi) = (f.bounds.min.dot(u), f.bounds.max.dot(u));
    let at = lo + (hi - lo) * a.num("position") / 100.0;
    let (pr, pl, c) = (a.num("peg") * 0.5, a.num("peg_length"), a.num("clearance"));
    let pegs = spots(&f, axis, at, pr + 0.8, pl + 1.0, a.int("pegs"));
    let mut warnings = Vec::new();
    if pegs.len() < a.int("pegs") {
        warnings.push(format!("only {} peg(s) fit the cut", pegs.len()));
    }
    let gap = (hi - lo) * 0.15 + 2.0 * pl;
    let mut bodies = Vec::new();
    if !is_mesh(s, &body)? {
        // Exact: the kernel cuts, the pegs are cylinders.
        let before = names(s)?;
        s.execute("solid.split_body", &json!({"body": body, "plane": {"origin": xyz(u * at), "normal": xyz(u)}}))?;
        let mut halves = new_since(s, &before)?;
        if s.world_state()?.body(&body).is_some() {
            halves.push(body.clone());
        }
        let upper: Vec<String> = halves.iter().filter(|n| mesh_of(s, &[(*n).clone()]).is_ok_and(|m| m.bounds().center().dot(u) > at)).cloned().collect();
        let lower: Vec<String> = halves.iter().filter(|n| !upper.contains(n)).cloned().collect();
        for p in &pegs {
            for up in &upper {
                if mesh_of(s, std::slice::from_ref(up)).is_ok_and(|m| Source::new(m).field().at(*p + u * 0.5, 1.0) < 0.0) {
                    cylinder(s, *p - u * 0.05, u, 2.0 * (pr + c), pl + 0.5, Do::Cut(up))?;
                }
            }
            for low in &lower {
                if mesh_of(s, std::slice::from_ref(low)).is_ok_and(|m| Source::new(m).field().at(*p - u * 0.5, 1.0) < 0.0) {
                    cylinder(s, *p - u * 0.5, u, 2.0 * pr, pl + 0.5 - 0.2, Do::Join(low))?;
                }
            }
        }
        if a.flag("apart") && !upper.is_empty() {
            let t = u * gap;
            s.execute("solid.move", &json!({"bodies": upper, "translate": [mm(t.x), mm(t.y), mm(t.z)]}))?;
        }
        bodies.extend(halves);
    } else {
        let h = detail(a, &f.bounds, &mut warnings);
        let reach = 4.0 * h;
        for top in [false, true] {
            let mut b = f.bounds;
            let cut = |p: Vec3| if top { at - p.dot(u) } else { p.dot(u) - at };
            if !top {
                b = b.inflated(pl);
            }
            let m = sdf::polygonize(b, h, |p| {
                let mut d = f.at(p, reach).max(cut(p));
                for q in &pegs {
                    let peg = sdf::cylinder(p, *q - u, u, pr + if top { c } else { 0.0 }, pl + 1.0 + if top { 0.3 } else { -0.2 });
                    d = if top { d.max(-peg) } else { d.min(peg) };
                }
                d
            })
            .map_err(|e| a.fail(e))?;
            let m = if top && a.flag("apart") { moved(m, u * gap) } else { m };
            bodies.extend(add_mesh_body(s, &format!("{body} {}", if top { "top" } else { "base" }), &m)?);
        }
        remove(s, &[body.clone()])?;
    }
    Ok(json!({"summary": format!("`{body}` cut at {at:.1} mm with {} peg(s)", pegs.len()), "created_bodies": bodies, "warnings": warnings}))
}

fn moved(mut m: PrintMesh, by: Vec3) -> PrintMesh {
    for p in &mut m.positions {
        *p = *p + by;
    }
    m
}

pub(super) fn flexi(s: &mut Session, a: &Args) -> Result<Value> {
    let (body, src) = load(s, a)?;
    let f = src.field();
    let (axis, u) = unit(a);
    let (lo, hi) = (f.bounds.min.dot(u), f.bounds.max.dot(u));
    let (n, gap, c) = (a.int("joints"), a.num("gap"), a.num("clearance"));
    let mut warnings = Vec::new();
    let least = (f.bounds.size().x * f.bounds.size().y * f.bounds.size().z / 12_000_000.0).cbrt();
    // Fine enough to keep the clearances open.
    let h = a.num("detail").min(c * 0.5).max(least);
    if h > c * 0.7 {
        warnings.push("the model is large for these clearances: raise the gap and the ball clearance".into());
    }
    // At each cut, the deepest point inside is where the ball goes.
    let mut balls: Vec<(f64, Vec3, f64)> = Vec::new();
    for i in 1..=n {
        let at = lo + (hi - lo) * i as f64 / (n as f64 + 1.0);
        let (mut best, mut depth) = (u * at, 0.0);
        let b = &f.bounds;
        for j in 0..40 {
            for k in 0..40 {
                let (x, y) = ((f64::from(j) + 0.5) / 40.0, (f64::from(k) + 0.5) / 40.0);
                let p = match axis {
                    0 => v(at, b.min.y + b.size().y * x, b.min.z + b.size().z * y),
                    1 => v(b.min.x + b.size().x * x, at, b.min.z + b.size().z * y),
                    _ => v(b.min.x + b.size().x * x, b.min.y + b.size().y * y, at),
                };
                let d = -f.at(p, b.diagonal());
                if d > depth {
                    (best, depth) = (p, d);
                }
            }
        }
        // No larger than leaves a wall between one joint's socket and the next one's neck.
        let spacing = (hi - lo) / (n as f64 + 1.0);
        let r = (depth * a.num("ball") / 100.0).min((spacing - 1.2 - 2.0 * gap - c) / 2.4);
        if r < 1.4 + c {
            warnings.push(format!("joint {i} is too thin for a ball and was left out"));
            continue;
        }
        balls.push((at, best, r));
    }
    if balls.is_empty() {
        return Err(a.fail("the model is too thin for joints along that axis"));
    }
    let pitch = (hi - lo) / (n as f64 + 1.0);
    let reach = 4.0 * h + gap;
    let m = sdf::polygonize(f.bounds, h, |p| {
        let skin = f.at(p, reach);
        let mut d = skin;
        if skin >= reach {
            return skin;
        }
        let z = p.dot(u);
        for (at, c0, r) in &balls {
            if (z - at).abs() > pitch {
                continue;
            }
            // The ball sits past the cut, on a neck from before it.
            let centre = *c0 + u * (gap * 0.5 + r * 0.75);
            let neck = r * 0.5;
            let mouth = sdf::capsule(p, *c0 - u * (r * 0.3), centre, neck + c);
            let socket = sdf::sphere(p, centre, r + c);
            let slot = (z - at).abs() - gap * 0.5;
            let after = at - z;
            d = d.max(-slot).max(-socket).max(-mouth.max(after - gap));
            let ball = sdf::sphere(p, centre, *r).min(sdf::capsule(p, *c0 - u * (gap + r * 0.6), centre, neck));
            d = d.min(ball.max(skin));
        }
        d
    })
    .map_err(|e| a.fail(e))?;
    let parts = rybocraft_print3d::analysis::shells(&m);
    let whole: f64 = parts.iter().map(PrintMesh::volume).filter(|v| *v > 0.0).sum();
    // Crumbs left where the cut grazes the surface go; hollows inside stay.
    let mut kept = PrintMesh::default();
    let mut links = 0;
    for p in &parts {
        let v = p.volume();
        if v < 0.0 || v > whole * 0.002 {
            links += usize::from(v > 0.0);
            kept.append(&p.positions, &p.triangles);
        }
    }
    let m = kept;
    if links <= balls.len() {
        warnings.push("some links are not free: raise the clearance or lower the detail".into());
    }
    let joints = balls.len();
    replace(s, &body, &m, format!("`{body}` in {links} links with {joints} ball joint(s)"), warnings)
}

pub(super) fn cavity(s: &mut Session, a: &Args) -> Result<Value> {
    let (body, src) = load(s, a)?;
    let f = src.field();
    let (d, want) = match a.text("preset") {
        "can" => (67.5, 95.0),
        "slim can" => (59.0, 110.0),
        "tall can" => (67.5, 130.0),
        "tealight" => (39.5, 17.0),
        "pen" => (12.0, 80.0),
        "pint tub" => (98.0, 95.0),
        _ => (a.num("diameter"), a.num("depth")),
    };
    let b = f.bounds;
    let (c, top) = (b.center(), b.max.z);
    let deep = want.min(b.size().z - a.num("floor"));
    if deep < 1.0 {
        return Err(a.fail("the model is not tall enough for that floor"));
    }
    let mut warnings = Vec::new();
    if d + 2.0 > b.size().x.min(b.size().y) {
        warnings.push(format!("a {d} mm hole breaks through the sides: resize the model first"));
    }
    if deep + 0.01 < want {
        warnings.push(format!("only {deep:.0} of {want:.0} mm deep: the model is not taller"));
    }
    let base = v(c.x, c.y, top - deep);
    if !is_mesh(s, &body)? {
        cylinder(s, base, UP, d, deep + 1.0, Do::Cut(&body))?;
        return Ok(json!({"summary": format!("{d} mm hole, {deep:.0} mm deep"), "body": body, "warnings": warnings}));
    }
    let h = detail(a, &b, &mut warnings);
    let m = sdf::polygonize(b, h, |p| f.at(p, 4.0 * h).max(-sdf::cylinder(p, base, UP, d * 0.5, deep + 5.0))).map_err(|e| a.fail(e))?;
    replace(s, &body, &m, format!("{d} mm hole, {deep:.0} mm deep"), warnings)
}

pub(super) fn mold(s: &mut Session, a: &Args) -> Result<Value> {
    let (body, src) = load(s, a)?;
    let f = src.field();
    let (axis, u) = unit(a);
    let w = a.num("wall");
    let b = f.bounds.inflated(w);
    let at = f.bounds.center().dot(u);
    let (key, c, pour) = (a.num("key") * 0.5, a.num("clearance"), a.num("pour") * 0.5);
    // Keys in the corners of the parting face.
    let mid = b.center();
    let half = b.size() * 0.5;
    let off = w * 0.5;
    let mut keys = Vec::new();
    for (i, j) in [(-1.0, -1.0), (1.0, -1.0), (-1.0, 1.0), (1.0, 1.0)] {
        let p = match axis {
            0 => v(at, mid.y + i * (half.y - off), mid.z + j * (half.z - off)),
            1 => v(mid.x + i * (half.x - off), at, mid.z + j * (half.z - off)),
            _ => v(mid.x + i * (half.x - off), mid.y + j * (half.y - off), at),
        };
        keys.push(p);
    }
    let kr = key.min(off - 0.8).max(0.0);
    let mut warnings = Vec::new();
    let h = detail(a, &b, &mut warnings);
    let reach = 4.0 * h;
    let top = f.bounds.max.z;
    let mut bodies = Vec::new();
    for upper in [false, true] {
        let mut room = b;
        if !upper {
            room = room.inflated(kr);
        }
        let m = sdf::polygonize(room, h, |p| {
            let side = if upper { at - p.dot(u) } else { p.dot(u) - at };
            let mut d = sdf::cuboid(p, b.min, b.max, 0.0).max(-f.at(p, reach)).max(side);
            if pour > 0.0 {
                // A funnel down to the model's highest part.
                let open = pour + (p.z - top).max(0.0) * 0.4;
                let hole = (v(p.x - mid.x, p.y - mid.y, 0.).len() - open).max(top - pour - p.z);
                d = d.max(-hole);
            }
            if kr > 0.5 {
                for q in &keys {
                    let ball = sdf::sphere(p, *q, kr + if upper { c } else { 0.0 });
                    d = if upper { d.max(-ball) } else { d.min(ball) };
                }
            }
            d
        })
        .map_err(|e| a.fail(e))?;
        let m = if upper { moved(m, u * (b.size().dot(u) * 0.6 + 10.0)) } else { m };
        bodies.extend(add_mesh_body(s, &format!("{body} mould {}", if upper { "B" } else { "A" }), &m)?);
    }
    let _ = s.execute("browser.visibility", &json!({"items": [body], "visible": false}));
    Ok(json!({"summary": format!("Two-part mould of `{body}`"), "created_bodies": bodies, "warnings": warnings}))
}

pub(in crate::cmd) fn hollow_mesh(s: &mut Session, a: &Args, wall: f64, open: &str, diameter: f64, h: f64) -> Result<Value> {
    let (body, src) = load(s, a)?;
    let f = src.field();
    let before = src.mesh.volume();
    let b = f.bounds;
    let least = ((b.size().x + 1.0) * (b.size().y + 1.0) * (b.size().z + 1.0) / 12_000_000.0).cbrt();
    let mut warnings = Vec::new();
    if least > h {
        warnings.push(format!("detail raised to {least:.2} mm for a model this large"));
    }
    let h = h.max(least).min(wall * 0.8).max(least);
    let reach = wall + 4.0 * h;
    let c = b.center();
    let open = open.to_string();
    let m = sdf::polygonize(b, h, |p| {
        let d = f.at(p, reach);
        let mut out = d.max(-(d + wall));
        let radial = v(p.x - c.x, p.y - c.y, 0.).len() - diameter * 0.5;
        match open.as_str() {
            "top" => out = out.max(-radial.max(b.min.z + wall - p.z)),
            "bottom" => out = out.max(-radial.max(p.z - b.min.z - wall * 1.5)),
            _ => {}
        }
        out
    })
    .map_err(|e| a.fail(e))?;
    let saved = (1.0 - m.volume() / before.max(1e-9)) * 100.0;
    if open == "none" {
        warnings.push("closed inside: nothing can drain out".into());
    }
    replace(s, &body, &m, format!("`{body}` hollowed to {wall} mm: {saved:.0}% less material"), warnings)
}

pub(super) fn hollow(s: &mut Session, a: &Args) -> Result<Value> {
    hollow_mesh(s, a, a.num("wall"), a.text("open"), a.num("diameter"), a.num("detail"))
}

pub(super) fn puzzle(s: &mut Session, a: &Args) -> Result<Value> {
    let (body, src) = load(s, a)?;
    let f = src.field();
    let n = [a.int("nx"), a.int("ny"), a.int("nz")];
    if n.iter().product::<usize>() < 2 {
        return Err(a.fail("ask for at least two pieces"));
    }
    if n.iter().product::<usize>() > 60 {
        return Err(a.fail("at most 60 pieces"));
    }
    let b = f.bounds;
    let (c, r) = (a.num("clearance"), a.num("peg") * 0.5);
    let step = v(b.size().x / n[0] as f64, b.size().y / n[1] as f64, b.size().z / n[2] as f64);
    let mut warnings = Vec::new();
    let h = detail(a, &b, &mut warnings);
    let reach = 4.0 * h;
    let axes = [v(1., 0., 0.), v(0., 1., 0.), UP];
    let mut bodies = Vec::new();
    for i in 0..n[0] {
        for j in 0..n[1] {
            for k in 0..n[2] {
                let lo = b.min + v(step.x * i as f64, step.y * j as f64, step.z * k as f64);
                let hi = lo + step;
                let mid = (lo + hi) * 0.5;
                // A peg towards each higher neighbour, a hole towards each lower one.
                let (mut pegs, mut holes) = (Vec::new(), Vec::new());
                for (x, ax) in axes.iter().enumerate() {
                    let at = [i, j, k][x];
                    let half = step.dot(*ax) * 0.5;
                    if at + 1 < n[x] {
                        pegs.push((mid + *ax * half, *ax));
                    }
                    if at > 0 {
                        holes.push((mid - *ax * half, *ax));
                    }
                }
                let solid = |q: &Vec3| r > 0.0 && f.at(*q, r * 2.0 + 1.0) < -(r + 0.5);
                pegs.retain(|(q, _)| solid(q));
                holes.retain(|(q, _)| solid(q));
                let room: rybocraft_geom::Aabb3 = [lo, hi].into_iter().collect();
                let m = sdf::polygonize(room.inflated(r * 1.5), h, |p| {
                    let body_d = f.at(p, reach);
                    let mut d = body_d.max(sdf::cuboid(p, lo, hi, 0.0) + c * 0.5);
                    for (q, ax) in &pegs {
                        d = d.min(sdf::capsule(p, *q - *ax * (r + c), *q + *ax * (r * 0.6), r).max(body_d));
                    }
                    for (q, ax) in &holes {
                        d = d.max(-sdf::capsule(p, *q - *ax * r, *q + *ax * (r * 0.6 + c), r + c));
                    }
                    d
                });
                // A block the model does not reach is no piece.
                if let Ok(m) = m {
                    let away = (mid - b.center()) * 0.5;
                    bodies.extend(add_mesh_body(s, &format!("{body} piece {}", bodies.len() + 1), &moved(m, away))?);
                }
            }
        }
    }
    remove(s, &[body.clone()])?;
    Ok(json!({"summary": format!("{} pieces", bodies.len()), "created_bodies": bodies, "warnings": warnings}))
}

pub(super) fn flat_base(s: &mut Session, a: &Args) -> Result<Value> {
    let (body, src) = load(s, a)?;
    let f = src.field();
    let z = f.bounds.min.z + a.num("height");
    if z >= f.bounds.max.z - 0.1 {
        return Err(a.fail("that would leave nothing"));
    }
    if !is_mesh(s, &body)? {
        let b = f.bounds.inflated(5.0);
        let size = b.size();
        cuboid(s, b.min, [size.x, size.y, z - b.min.z], Do::Cut(&body))?;
        s.execute("solid.move", &json!({"bodies": [body], "translate": [mm(0.0), mm(0.0), mm(-z)]}))?;
        return Ok(json!({"summary": format!("`{body}` cut flat"), "body": body}));
    }
    let mut warnings = Vec::new();
    let h = detail(a, &f.bounds, &mut warnings);
    let m = sdf::polygonize(f.bounds, h, |p| f.at(p, 4.0 * h).max(z - p.z)).map_err(|e| a.fail(e))?;
    let low = m.bounds().min.z;
    replace(s, &body, &moved(m, UP * -low), format!("`{body}` cut flat"), warnings)
}

pub(super) fn resize(s: &mut Session, a: &Args) -> Result<Value> {
    let body = a.body(s)?;
    let b = mesh_of(s, std::slice::from_ref(&body))?.bounds();
    let (axis, u) = unit(a);
    let now = b.size().dot(u);
    if now < 1e-6 {
        return Err(a.fail("the model has no size along that axis"));
    }
    let k = a.num("size") / now;
    let origin = json!([b.center().x, b.center().y, b.min.z]);
    let p = if a.flag("even") {
        json!({"bodies": [body], "factor": k, "origin": origin})
    } else {
        let mut f = [1.0; 3];
        if let Some(x) = f.get_mut(axis) {
            *x = k;
        }
        json!({"bodies": [body], "factors": f, "origin": origin})
    };
    s.execute("solid.scale", &p)?;
    Ok(json!({"summary": format!("`{body}` is now {} mm along {} ({:.1}%)", a.num("size"), a.text("axis"), k * 100.0), "factor": k, "body": body}))
}

pub(super) fn ring(s: &mut Session, a: &Args) -> Result<Value> {
    let body = a.body(s)?;
    let m = mesh_of(s, std::slice::from_ref(&body))?;
    let t = a.num("thickness");
    let major = a.num("diameter") * 0.5 + t * 0.5;
    // Above the highest point, its foot sunk into the model.
    let top = m.positions.iter().copied().max_by(|p, q| p.z.total_cmp(&q.z)).unwrap_or(v(0., 0., 0.));
    let near: Vec<Vec3> = m.positions.iter().copied().filter(|p| p.z > top.z - 0.05).collect();
    let mid = near.iter().fold(v(0., 0., 0.), |s, p| s + *p) * (1.0 / near.len().max(1) as f64);
    let c = v(mid.x, mid.y, top.z + major - t * 0.2);
    let axis = if a.text("facing") == "X" { v(1., 0., 0.) } else { v(0., 1., 0.) };
    let b: rybocraft_geom::Aabb3 = [c].into_iter().collect::<rybocraft_geom::Aabb3>().inflated(major + t);
    let loop_ = sdf::polygonize(b, (t * 0.15).clamp(0.12, 0.5), |p| sdf::torus(p, c, axis, major, t * 0.5)).map_err(|e| a.fail(e))?;
    if is_mesh(s, &body)? {
        // One body of two overlapping shells: slicers join them.
        let mut whole = m;
        whole.append(&loop_.positions, &loop_.triangles);
        return replace(s, &body, &whole, format!("Loop on `{body}`"), Vec::new());
    }
    mesh_made(s, "Loop", &loop_, format!("Loop on `{body}`"), vec!["the loop is its own body: export both together".into()])
}

pub(super) fn fuse(s: &mut Session, a: &Args) -> Result<Value> {
    let bodies = a.bodies(s)?;
    if bodies.len() < 2 {
        return Err(a.fail("select two or more models"));
    }
    let sources: Vec<Source> = bodies.iter().map(|b| mesh_of(s, std::slice::from_ref(b)).map(Source::new)).collect::<Result<_>>()?;
    let fields: Vec<_> = sources.iter().map(Source::field).collect();
    let k = a.num("blend");
    let b = fields.iter().fold(rybocraft_geom::Aabb3::EMPTY, |b, f| b.union(&f.bounds)).inflated(k * 0.3);
    let mut warnings = Vec::new();
    let h = detail(a, &b, &mut warnings);
    let reach = 4.0 * h + 2.0 * k;
    let m = sdf::polygonize(b, h, |p| {
        let mut d = f64::INFINITY;
        for f in &fields {
            let x = f.at(p, reach);
            d = if k > 0.0 && d.is_finite() {
                let t = (k - (d - x).abs()).max(0.0) / k;
                d.min(x) - t * t * k * 0.25
            } else {
                d.min(x)
            };
        }
        d
    })
    .map_err(|e| a.fail(e))?;
    let n = bodies.len();
    remove(s, &bodies)?;
    mesh_made(s, "Fused", &m, format!("{n} models fused into one"), warnings)
}

pub(super) fn cut_model(s: &mut Session, a: &Args) -> Result<Value> {
    let bodies = a.bodies(s)?;
    let [keep, cutters @ ..] = bodies.as_slice() else { return Err(a.fail("select the model to keep, then the cutter")) };
    if cutters.is_empty() {
        return Err(a.fail("select the model to keep, then the cutter"));
    }
    let src = Source::new(mesh_of(s, std::slice::from_ref(keep))?);
    let tool = Source::new(mesh_of(s, cutters)?);
    let (f, g) = (src.field(), tool.field());
    let c = a.num("clearance");
    let mut warnings = Vec::new();
    let h = detail(a, &f.bounds, &mut warnings);
    let reach = 4.0 * h + c;
    let m = sdf::polygonize(f.bounds, h, |p| f.at(p, reach).max(-(g.at(p, reach) - c))).map_err(|e| a.fail(e))?;
    if !a.flag("keep") {
        remove(s, cutters)?;
    }
    replace(s, keep, &m, format!("`{keep}` cut by {}", cutters.join(", ")), warnings)
}
