//! Trays, drawers and boxes.

use super::*;

pub(super) fn tray(s: &mut Session, a: &Args) -> Result<Value> {
    let o = a.at();
    let (l, w, h, t, d) = (a.num("length"), a.num("width"), a.num("height"), a.num("wall"), a.num("divider"));
    let spaces: Vec<usize> = list(a.text("columns")).iter().map(|x| *x as usize).filter(|n| *n > 0).collect();
    let n = spaces.len();
    if n == 0 || n > 20 || spaces.iter().sum::<usize>() > 100 {
        return Err(a.fail("`columns` must list 1 to 20 columns, at most 100 spaces in all, such as 2,2,2"));
    }
    let mut shares = list(a.text("widths"));
    if shares.len() != n {
        shares = vec![1.0; n];
    }
    let (il, iw) = (l - 2.0 * t, w - 2.0 * t);
    let room = il - d * (n as f64 - 1.0);
    if room < 5.0 * n as f64 || spaces.iter().any(|k| (iw - d * (*k as f64 - 1.0)) < 5.0 * *k as f64) {
        return Err(a.fail("the spaces would be smaller than 5 mm"));
    }
    let r = a.num("radius");
    let body = rounded(s, Flat::level(o), [l, w], r, h, 0.0, Do::New("Tray"))?;
    rounded(s, Flat::level(o + UP * t), [il, iw], (r - t).max(if r > 0.0 { 0.5 } else { 0.0 }), h, 0.0, Do::Cut(&body))?;
    let top = h - t - a.num("drop");
    if top <= 0.5 {
        return Err(a.fail("the dividers would have no height"));
    }
    let sum: f64 = shares.iter().sum();
    let mut x = o.x - il * 0.5;
    let y0 = o.y - iw * 0.5;
    // Dividers reach into walls and floor, so they join.
    let sink = t * 0.5;
    for (i, k) in spaces.iter().enumerate() {
        let cw = room * shares.get(i).copied().unwrap_or(1.0) / sum;
        for j in 1..*k {
            let y = y0 + iw * j as f64 / *k as f64 - d * 0.5;
            cuboid(s, v(x - sink, y, o.z + t - sink), [cw + 2.0 * sink.min(d.max(sink)), d, top + sink], Do::Join(&body))?;
        }
        x += cw;
        if i + 1 < n {
            cuboid(s, v(x, y0 - sink, o.z + t - sink), [d, iw + 2.0 * sink, top + sink], Do::Join(&body))?;
            x += d;
        }
    }
    let mut bodies = vec![body];
    if a.flag("lid") {
        let at = o + v(l + 10.0, 0.0, 0.0);
        let (c, skirt) = (0.3, 6.0f64.min(h * 0.4));
        let lid = rounded(s, Flat::level(at), [l + 2.0 * (t + c), w + 2.0 * (t + c)], r + t, t + skirt, 0.0, Do::New("Tray lid"))?;
        rounded(s, Flat::level(at + UP * t), [l + 2.0 * c, w + 2.0 * c], (r + c).max(0.3), skirt + 0.1, 0.0, Do::Cut(&lid))?;
        bodies.push(lid);
    }
    made(format!("{l} × {w} × {h} mm tray, {} spaces", spaces.iter().sum::<usize>()), &bodies)
}

pub(super) fn cabinet(s: &mut Session, a: &Args) -> Result<Value> {
    let o = a.at();
    let (nc, nr) = (a.int("columns"), a.int("rows"));
    let (cw, ch, depth, t, c) = (a.num("cell_w"), a.num("cell_h"), a.num("depth"), a.num("wall"), a.num("clearance"));
    if nc * nr > 40 {
        return Err(a.fail("at most 40 cells"));
    }
    let kinds: Vec<String> = a.text("cells").split(',').map(|k| k.trim().to_ascii_lowercase()).filter(|k| !k.is_empty()).collect();
    if kinds.is_empty() || kinds.iter().any(|k| !["drawer", "tray", "cubby"].contains(&k.as_str())) {
        return Err(a.fail("`cells` takes drawer, tray and cubby, separated by commas"));
    }
    if cw < 4.0 * t + 8.0 || ch < 2.0 * t + 8.0 {
        return Err(a.fail("the cells are too small for their walls"));
    }
    let (w, h) = (nc as f64 * cw + (nc as f64 + 1.0) * t, nr as f64 * ch + (nr as f64 + 1.0) * t);
    // Front faces -Y; the back is closed.
    let house = cuboid(s, o + v(-w * 0.5, 0.0, 0.0), [w, depth, h], Do::New("Cabinet"))?;
    let mut bodies = vec![house.clone()];
    let mut drawers = 0usize;
    for r in 0..nr {
        for k in 0..nc {
            let at = o + v(-w * 0.5 + t + k as f64 * (cw + t), -0.1, t + r as f64 * (ch + t));
            cuboid(s, at, [cw, depth - t + 0.1, ch], Do::Cut(&house))?;
            let kind = kinds.get(r * nc + k).or(kinds.last()).map_or("drawer", String::as_str);
            if kind == "cubby" {
                continue;
            }
            // The drawers stand in front, in the order of their cells.
            let dh = if kind == "tray" { (ch * 0.35).max(8.0) } else { ch } - 2.0 * c;
            let (dw, dd) = (cw - 2.0 * c, depth - t - c);
            let p = o + v(-w * 0.5 + t + c + k as f64 * (cw + t), -(dd + 15.0) * (r as f64 + 1.0), 0.0);
            let name = format!("{} {}", if kind == "tray" { "Tray" } else { "Drawer" }, drawers + 1);
            let box_ = cuboid(s, p, [dw, dd, dh], Do::New(&name))?;
            cuboid(s, p + v(t, t, t), [dw - 2.0 * t, dd - 2.0 * t, dh], Do::Cut(&box_))?;
            let mid = p.x + dw * 0.5;
            match a.text("handle") {
                "knob" => {
                    let kd = (dh * 0.35).min(14.0);
                    cylinder(s, v(mid, p.y + 0.5, p.z + dh * 0.5), v(0., -1., 0.), kd * 0.6, 8.5, Do::Join(&box_))?;
                    cylinder(s, v(mid, p.y - 6.0, p.z + dh * 0.5), v(0., -1., 0.), kd, 4.0, Do::Join(&box_))?;
                }
                "scoop" => {
                    let sd = (dw * 0.4).min(dh * 0.9).min(40.0);
                    cylinder(s, v(mid, p.y - 0.1, p.z + dh), v(0., 1., 0.), sd, t + 0.2, Do::Cut(&box_))?;
                }
                _ => {
                    let (sw, sh) = ((dw * 0.4).min(45.0), (dh * 0.2).clamp(4.0, 10.0));
                    let z = p.z + dh * 0.65;
                    cuboid(s, v(mid - sw * 0.5 + sh * 0.5, p.y - 0.1, z - sh * 0.5), [sw - sh, t + 0.2, sh], Do::Cut(&box_))?;
                    for x in [-1.0, 1.0] {
                        cylinder(s, v(mid + x * (sw - sh) * 0.5, p.y - 0.1, z), v(0., 1., 0.), sh, t + 0.2, Do::Cut(&box_))?;
                    }
                }
            }
            let nd = a.int("dividers");
            for i in 1..=nd {
                let y = p.y + dd * i as f64 / (nd as f64 + 1.0) - t * 0.4;
                cuboid(s, v(p.x + t * 0.5, y, p.z + t * 0.5), [dw - t, t * 0.8, (dh - t) * 0.85], Do::Join(&box_))?;
            }
            drawers += 1;
            bodies.push(box_);
        }
    }
    made(format!("{w:.0} × {depth:.0} × {h:.0} mm cabinet with {drawers} drawer(s)"), &bodies)
}

pub(super) fn swing_tower(s: &mut Session, a: &Args) -> Result<Value> {
    let o = a.at();
    let (n, w, d, dh, t, c) = (a.int("drawers"), a.num("width"), a.num("depth"), a.num("drawer_h"), a.num("wall"), a.num("clearance"));
    let r = 8.0f64.max(t * 2.0 + 4.0);
    let pin = 8.0;
    if w < 4.0 * r || d < 3.0 * r {
        return Err(a.fail("too small to turn: make it wider or deeper"));
    }
    // The post stands at the front right; the left side stays open so the drawers can turn.
    let pitch = dh + c + t;
    let h = t + n as f64 * pitch;
    let post = v(o.x + w * 0.5 - t - c - r, o.y - d * 0.5 + r, o.z);
    let (x0, y0) = (o.x - w * 0.5, o.y - d * 0.5);
    let house = cuboid(s, v(x0, y0, o.z), [w, d, t], Do::New("Tower"))?;
    cuboid(s, v(x0, y0 + d - t, o.z), [w, t, h], Do::Join(&house))?;
    cuboid(s, v(x0 + w - t, y0 + r, o.z), [t, d - r, h], Do::Join(&house))?;
    for i in 1..=n {
        cuboid(s, v(x0, y0, o.z + i as f64 * pitch), [w, d, t], Do::Join(&house))?;
    }
    cylinder(s, post - UP * 0.1, UP, pin + 2.0 * c, h + 0.2, Do::Cut(&house))?;
    let labels: Vec<&str> = a.text("labels").split(',').map(str::trim).collect();
    let mut bodies = vec![house];
    for i in 0..n {
        let z = o.z + t + i as f64 * pitch + c * 0.5;
        let (dw, dd) = (w - t - c - 0.5, d - t - c);
        let (bx, by) = (post.x + r - dw, y0);
        let name = format!("Swing drawer {}", i + 1);
        let body = cuboid(s, v(bx, by + r, z), [dw, dd - r, dh], Do::New(&name))?;
        cuboid(s, v(bx, by, z), [dw - r, r + 0.1, dh], Do::Join(&body))?;
        cylinder(s, v(post.x, post.y, z), UP, 2.0 * r, dh, Do::Join(&body))?;
        cuboid(s, v(bx + t, by + t, z + t), [dw - 2.0 * t, dd - 2.0 * t, dh], Do::Cut(&body))?;
        // The corner stays solid around the post.
        let boss = cylinder(s, v(post.x, post.y, z), UP, 2.0 * r, dh, Do::Join(&body))?;
        cylinder(s, v(post.x, post.y, z - 0.1), UP, pin + 2.0 * c, dh + 0.2, Do::Cut(&boss))?;
        let scoop = (dh * 0.8).min(22.0);
        cylinder(s, v(bx + 12.0 + scoop * 0.5, by - 0.1, z + dh), v(0., 1., 0.), scoop, t + 0.2, Do::Cut(&body))?;
        if let Some(text) = labels.get(i).filter(|l| !l.is_empty()) {
            let size = (dh * 0.38).min(dw * 0.75 / (text.chars().count() as f64 * 0.8));
            let flat = Flat { origin: v(bx + dw * 0.52, by + 0.2, z + dh * 0.45), x: v(1., 0., 0.), y: UP };
            lettering(s, a, flat, text, size, 1.0, Do::Join(&body))?;
        }
        if i + 1 == n && a.num("open") > 0.1 {
            turn_about(s, &body, v(post.x, post.y, 0.0), a.num("open"))?;
        }
        bodies.push(body);
    }
    // The post is printed beside it and dropped in from above.
    let at = v(x0 - 25.0, o.y, o.z);
    let pin_body = cylinder(s, at, UP, pin + 5.0, 2.0, Do::New("Tower post"))?;
    cylinder(s, at + UP * 1.5, UP, pin, h + 0.5, Do::Join(&pin_body))?;
    bodies.push(pin_body);
    made(format!("{w} × {d} × {h:.1} mm tower, {n} swing-out drawers and a post"), &bodies)
}

/// Turn a body about a vertical line.
fn turn_about(s: &mut Session, body: &str, at: Vec3, degrees: f64) -> Result<()> {
    s.execute("solid.move", &json!({"bodies": [body], "translate": [mm(-at.x), mm(-at.y), mm(0.0)]}))?;
    s.execute("solid.move", &json!({"bodies": [body], "axis": [0, 0, 1], "angle": format!("{degrees} deg")}))?;
    s.execute("solid.move", &json!({"bodies": [body], "translate": [mm(at.x), mm(at.y), mm(0.0)]})).map(|_| ())
}

pub(super) fn hinged_box(s: &mut Session, a: &Args) -> Result<Value> {
    let o = a.at();
    let (l, w, h, t, r, c) = (a.num("length"), a.num("width"), a.num("height"), a.num("wall"), a.num("radius"), a.num("clearance"));
    let br = 3.6;
    let n = 5usize;
    let hinge = (l - 2.0 * r - 4.0).min(60.0);
    let seg = (hinge - c * (n as f64 - 1.0)) / n as f64;
    let gap = br + c;
    let (big, small, reach) = (br - 0.8 - c, (br - 0.8 - c) * 0.45, 1.6);
    if seg < 2.0 * reach + 1.5 || h < br + 3.0 {
        return Err(a.fail("too small for the hinge: make it longer or taller"));
    }
    let axis = v(1.0, 0.0, 0.0);
    let z = o.z + h;
    let x = |i: usize| o.x - hinge * 0.5 + i as f64 * (seg + c);
    let mut bodies = Vec::new();
    for (side, name) in ["Box", "Box lid"].into_iter().enumerate() {
        let sign = if side == 0 { -1.0 } else { 1.0 };
        let mid = v(o.x, o.y + sign * (gap + w * 0.5), o.z);
        let body = rounded(s, Flat::level(mid), [l, w], r, h, 0.0, Do::New(name))?;
        rounded(s, Flat::level(mid + UP * t), [l - 2.0 * t, w - 2.0 * t], (r - t).max(0.0), h, 0.0, Do::Cut(&body))?;
        if side == 0 {
            let (nc, nr) = (a.int("cols"), a.int("rows"));
            let d = 1.2f64.max(t * 0.6);
            for i in 1..nc {
                let px = mid.x - (l - 2.0 * t) * 0.5 + (l - 2.0 * t) * i as f64 / nc as f64 - d * 0.5;
                cuboid(s, v(px, mid.y - w * 0.5 + t * 0.5, o.z + t * 0.5), [d, w - t, h - t - 0.5], Do::Join(&body))?;
            }
            for j in 1..nr {
                let py = mid.y - (w - 2.0 * t) * 0.5 + (w - 2.0 * t) * j as f64 / nr as f64 - d * 0.5;
                cuboid(s, v(mid.x - l * 0.5 + t * 0.5, py, o.z + t * 0.5), [l - t, d, h - t - 0.5], Do::Join(&body))?;
            }
        }
        for i in (side..n).step_by(2) {
            // A knuckle on a 45° bracket, so it prints without support.
            let wall_y = o.y + sign * (gap + 0.4);
            let flat = Flat { origin: v(x(i), o.y, z), x: v(0., sign, 0.), y: UP };
            let out = gap + 0.4;
            let drop = (out + br).min(h - 0.5);
            let pts = [[-br * 0.7, -br * 0.7], [out, -drop], [out, 0.0], [0.0, 0.0]];
            let pts: Vec<[f64; 2]> = if sign > 0.0 { pts.to_vec() } else { pts.iter().rev().copied().collect() };
            // The frame's normal is ±X by its handedness: grow along +X from the right end.
            let (origin, _) = if sign > 0.0 { (v(x(i), o.y, z), 1.0) } else { (v(x(i) + seg, o.y, z), -1.0) };
            let _ = wall_y;
            prism(s, Flat { origin, ..flat }, &pts, seg, Do::Join(&body))?;
            cylinder(s, v(x(i), o.y, z), axis, 2.0 * br, seg, Do::Join(&body))?;
        }
        bodies.push(body);
    }
    for i in (1..n).step_by(2) {
        for (face, dir) in [(x(i), 1.0), (x(i) + seg, -1.0)] {
            cone(s, v(face - dir * 0.01, o.y, z), axis * dir, 2.0 * (big + c), 2.0 * (small + c), reach, Do::Cut(&bodies[1]))?;
            let from = face - dir * c;
            cylinder(s, v(from - dir * 0.2, o.y, z), axis * dir, 2.0 * big, 0.2, Do::Join(&bodies[0]))?;
            cone(s, v(from, o.y, z), axis * dir, 2.0 * big, 2.0 * small, reach, Do::Join(&bodies[0]))?;
        }
    }
    // A bump on the box's front and a dimple in the lid's.
    let front = o.y - gap - w;
    let tab = (l * 0.25).min(20.0);
    cuboid(s, v(o.x - tab * 0.5, front - 1.2, o.z + h - 5.0), [tab, 1.4, 3.0], Do::Join(&bodies[0]))?;
    cuboid(s, v(o.x - tab * 0.5 - 1.0, o.y + gap + w - 0.2, o.z + h - 7.0), [tab + 2.0, 1.6 + c, 11.0], Do::Join(&bodies[1]))?;
    cuboid(s, v(o.x - tab * 0.5 - 0.5, o.y + gap + w - 0.1, o.z + h + 0.01), [tab + 1.0, 1.0 + c, 4.2], Do::Cut(&bodies[1]))?;
    made(format!("Hinged box, {l} × {w} × {} mm closed: print it open as it lies", 2.0 * h), &bodies)
}

pub(super) fn pegboard(s: &mut Session, a: &Args) -> Result<Value> {
    let o = a.at();
    let (n, reach, h, peg) = (a.int("holes"), a.num("reach"), a.num("height").max(31.0), a.num("peg"));
    const PITCH: f64 = 25.4;
    let w = (n as f64 - 1.0) * PITCH + 16.0;
    let t = 4.0;
    // Lies with the board plane at Y = 0: the plate in front, the pegs behind.
    let body = cuboid(s, o + v(-w * 0.5, -t, 0.0), [w, t, h], Do::New("Pegboard holder"))?;
    for i in 0..n {
        let x = -w * 0.5 + 8.0 + i as f64 * PITCH;
        let top = o + v(x, -0.5, h - 6.0);
        // The upper peg turns up behind the board; the lower one only locates.
        cylinder(s, top, v(0., 1., 0.), peg, 9.0, Do::Join(&body))?;
        cylinder(s, top + v(0., 8.0, -peg * 0.5 + 0.2), UP, peg, 11.0, Do::Join(&body))?;
        if h - 6.0 >= PITCH + 4.0 {
            cylinder(s, o + v(x, -0.5, h - 6.0 - PITCH), v(0., 1., 0.), peg, 6.0, Do::Join(&body))?;
        }
    }
    match a.text("kind") {
        "bin" => {
            let bin = cuboid(s, o + v(-w * 0.5, -t - reach + 0.1, 0.0), [w, reach, h * 0.75], Do::Join(&body))?;
            cuboid(s, o + v(-w * 0.5 + 2.0, -t - reach + 2.1, 2.0), [w - 4.0, reach - 2.2, h], Do::Cut(&bin))?;
        }
        "shelf" => {
            cuboid(s, o + v(-w * 0.5, -t - reach + 0.1, 0.0), [w, reach, t], Do::Join(&body))?;
            cuboid(s, o + v(-w * 0.5, -t - reach + 0.1, 0.0), [w, 2.4, t + 6.0], Do::Join(&body))?;
        }
        _ => {
            for i in 0..n {
                let x = -w * 0.5 + 8.0 + i as f64 * PITCH;
                cuboid(s, o + v(x - 4.0, -t - reach + 0.1, 4.0), [8.0, reach, 8.0], Do::Join(&body))?;
                cuboid(s, o + v(x - 4.0, -t - reach + 0.1, 4.0), [8.0, 6.0, 18.0], Do::Join(&body))?;
            }
        }
    }
    made(format!("Pegboard {} on {n} peg(s)", a.text("kind")), &[body])
}

pub(super) fn cup(s: &mut Session, a: &Args) -> Result<Value> {
    let o = a.at();
    let (n, d, h, t) = (a.int("sides"), a.num("diameter"), a.num("height"), a.num("wall"));
    let k = (std::f64::consts::PI / n as f64).cos();
    if d * 0.5 * k < t * 2.0 || h < t * 2.0 {
        return Err(a.fail("too small for its wall"));
    }
    let ring = |r: f64| -> Vec<[f64; 2]> {
        (0..n)
            .map(|i| {
                // A side, not a corner, faces the front.
                let ang = -std::f64::consts::FRAC_PI_2 + std::f64::consts::TAU * (i as f64 + 0.5) / n as f64;
                [r * ang.cos(), r * ang.sin()]
            })
            .collect()
    };
    let body = if n >= 64 {
        let b = cylinder(s, o, UP, d, h, Do::New("Cup"))?;
        cylinder(s, o + UP * t, UP, d - 2.0 * t, h, Do::Cut(&b))?;
        b
    } else {
        let b = prism(s, Flat::level(o), &ring(d * 0.5), h, Do::New("Cup"))?;
        prism(s, Flat::level(o + UP * t), &ring(d * 0.5 - t / k), h, Do::Cut(&b))?;
        b
    };
    if !a.text("text").trim().is_empty() {
        let text = words(a, "text")?;
        // Sunk well in, so letters reach a round wall too.
        let sink = if n >= 16 { t * 0.9 } else { 0.3 };
        let flat = Flat { origin: o + v(0.0, -d * 0.5 * k + sink, h * 0.5), x: v(1., 0., 0.), y: UP };
        // No wider than the side it is on.
        let want = a.num("size");
        let wide = letter_mesh(s, a, &text, want, 1.0)?.bounds().size().x;
        let side = if n >= 16 { d * 0.55 } else { d * (std::f64::consts::PI / n as f64).sin() - 4.0 };
        lettering(s, a, flat, &text, want * (side / wide).min(1.0), 1.2 + sink, Do::Join(&body))?;
    }
    made(format!("{n}-sided cup, {d} × {h} mm"), &[body])
}
