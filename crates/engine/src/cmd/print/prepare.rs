//! Analysing bodies for printing, and getting them onto the bed.

use rybocraft_print3d::orient::{Weights, axis_angle, orientations, rotate};
use rybocraft_print3d::slice::{PrintSettings, Slicer};
use rybocraft_print3d::supports::{SupportSettings, columns};
use rybocraft_print3d::{PrintMesh, analysis, pack, slice, tables};
use serde_json::{Value, json};
use rybocore_record::Vec3;
use rybocraft_geom::prelude::*;

use super::*;

/// At most this many triangles are highlighted.
const MAX_SHOWN: usize = 20_000;

fn overhang_angle(a: &Args) -> f64 {
    let given = a.num("angle");
    if given > 0.0 { given } else { setup().filament().overhang }
}

fn or_setup(given: f64, own: f64) -> f64 {
    if given > 0.0 { given } else { own }
}

fn r2(x: f64) -> f64 {
    (x * 100.0).round() / 100.0
}

fn paint(o: &mut PrintOverlay, m: &PrintMesh, list: &[usize], role: OverlayRole) {
    let room = MAX_SHOWN.saturating_sub(o.triangles.len());
    o.triangles.extend(list.iter().take(room).filter_map(|i| Some((role, m.tri(*i)?))));
}

pub(super) fn overhangs(s: &mut Session, a: &Args) -> Result<Value> {
    let m = mesh_of(s, &a.bodies(s)?)?;
    let angle = overhang_angle(a);
    let bed = m.bounds().min.z;
    let (list, area) = analysis::overhangs(&m, angle, bed);
    let support = slice::support_volume(&m, &list, bed, 0.15) / 1000.0;
    let mut o = PrintOverlay { label: format!("Overhangs under {angle:.0}°"), ..PrintOverlay::default() };
    paint(&mut o, &m, &list, OverlayRole::Overhang);
    show(s, o);
    let summary = if list.is_empty() {
        format!("No overhangs under {angle:.0}°: prints without support")
    } else {
        format!("{:.1} cm² of overhangs under {angle:.0}°, wanting about {support:.1} cm³ of support", area / 100.0)
    };
    Ok(json!({"summary": summary, "angle": angle, "overhang_area_mm2": r2(area), "share_percent": r2(area / m.area().max(1e-9) * 100.0), "support_cm3": r2(support), "faces": list.len()}))
}

pub(super) fn thin_walls(s: &mut Session, a: &Args) -> Result<Value> {
    let m = mesh_of(s, &a.bodies(s)?)?;
    let limit = a.num("min_wall");
    let t = analysis::thickness(&m, limit);
    let mut o = PrintOverlay { label: format!("Walls under {limit} mm"), ..PrintOverlay::default() };
    paint(&mut o, &m, &t.thin, OverlayRole::Thin);
    o.points.extend(t.at.filter(|_| !t.thin.is_empty()).map(|p| (OverlayRole::Thin, p)));
    show(s, o);
    let summary = if t.thin.is_empty() {
        format!("Every wall is at least {limit} mm (thinnest {:.2} mm)", t.min)
    } else {
        format!("{:.1} cm² is thinner than {limit} mm; the thinnest is {:.2} mm", t.thin_area / 100.0, t.min)
    };
    Ok(json!({"summary": summary, "thinnest_mm": r2(t.min), "at": t.at.map(xyz), "thin_area_mm2": r2(t.thin_area), "faces": t.thin.len()}))
}

pub(super) fn islands(s: &mut Session, a: &Args) -> Result<Value> {
    let m = mesh_of(s, &a.bodies(s)?)?;
    let sl = Slicer::new(&m, or_setup(a.num("layer"), setup().layer));
    let found = slice::islands(&sl, a.num("min_area"));
    let mut o = PrintOverlay { label: "Islands".into(), ..PrintOverlay::default() };
    o.points.extend(found.iter().map(|i| (OverlayRole::Island, Vec3::new(i.x, i.y, i.z))));
    show(s, o);
    let summary = match found.first() {
        None => format!("No islands in {} layers", sl.layers()),
        Some(f) => format!("{} island(s) start in mid-air; the first at layer {} (Z {:.2} mm)", found.len(), f.layer + 1, f.z),
    };
    Ok(json!({"summary": summary, "layers": sl.layers(), "islands": found}))
}

pub(super) fn layer(s: &mut Session, a: &Args) -> Result<Value> {
    let m = mesh_of(s, &a.bodies(s)?)?;
    let sl = Slicer::new(&m, or_setup(a.num("height"), setup().layer));
    if sl.layers() == 0 {
        return Err(a.fail("the bodies have no height"));
    }
    let i = a.int("layer").clamp(1, sl.layers()) - 1;
    let seg = sl.segments(i);
    let z = sl.z(i);
    let mut o = PrintOverlay { label: format!("Layer {} of {} · Z {z:.2} mm", i + 1, sl.layers()), ..PrintOverlay::default() };
    o.lines.extend(seg.iter().take(MAX_SHOWN).map(|[a, b]| (OverlayRole::Layer, [Vec3::new(a[0], a[1], z), Vec3::new(b[0], b[1], z)])));
    show(s, o);
    let (area, perimeter) = (slice::area(&seg), slice::perimeter(&seg));
    Ok(json!({
        "summary": format!("Layer {} of {} at Z {z:.2} mm: {area:.1} mm², outline {perimeter:.1} mm", i + 1, sl.layers()),
        "layer": i + 1, "layers": sl.layers(), "z": r2(z), "area_mm2": r2(area), "perimeter_mm": r2(perimeter),
    }))
}

fn settings(p: &PrintSetup, f: &tables::Filament, infill: f64, walls: usize, support_volume: f64) -> PrintSettings {
    let (speed, max_flow) = p.speeds();
    PrintSettings {
        line_width: p.nozzle * 1.05,
        walls,
        skins: p.skins,
        infill: infill / 100.0,
        wall_speed: speed[0],
        infill_speed: speed[1],
        max_flow,
        density: f.density,
        price: p.price,
        filament_diameter: 1.75,
        support_volume,
    }
}

fn clock(minutes: f64) -> String {
    let m = minutes.round().max(0.0) as u64;
    if m >= 60 { format!("{} h {:02} min", m / 60, m % 60) } else { format!("{m} min") }
}

pub(super) fn estimate(s: &mut Session, a: &Args) -> Result<Value> {
    let m = mesh_of(s, &a.bodies(s)?)?;
    let p = setup();
    let f = p.filament();
    let sl = Slicer::new(&m, or_setup(a.num("layer"), p.layer));
    let infill = if a.num("infill") >= 0.0 { a.num("infill") } else { p.infill };
    let walls = if a.int("walls") > 0 { a.int("walls") } else { p.walls };
    let support = if a.flag("supports") {
        let (list, _) = analysis::overhangs(&m, f.overhang, sl.bed);
        slice::support_volume(&m, &list, sl.bed, 0.15)
    } else {
        0.0
    };
    let e = slice::estimate(&sl, &settings(&p, &f, infill, walls, support));
    let n = a.int("copies").max(1) as f64;
    let summary = format!(
        "{}: about {} and {:.0} g of {} ({:.1} m), costing {:.2}",
        if n > 1.0 { format!("{n:.0} copies") } else { "Print".into() },
        clock(e.minutes * n),
        e.mass_g * n,
        f.name,
        e.filament_m * n,
        e.cost * n
    );
    Ok(json!({
        "summary": summary, "material": f.name, "layers": e.layers, "height_mm": r2(e.height), "minutes": r2(e.minutes * n), "time": clock(e.minutes * n),
        "mass_g": r2(e.mass_g * n), "filament_m": r2(e.filament_m * n), "volume_cm3": r2(e.volume_cm3 * n), "support_cm3": r2(e.support_cm3 * n), "cost": r2(e.cost * n),
        "warnings": ["An estimate: your slicer's speeds and patterns decide the real time"],
    }))
}

pub(super) fn compare_materials(s: &mut Session, a: &Args) -> Result<Value> {
    let m = mesh_of(s, &a.bodies(s)?)?;
    let p = setup();
    let sl = Slicer::new(&m, p.layer);
    let rows: Vec<Value> = tables::FILAMENTS
        .iter()
        .map(|f| {
            let mut own = settings(&p, f, p.infill, p.walls, 0.0);
            own.price = f.price;
            let e = slice::estimate(&sl, &own);
            json!({"material": f.name, "mass_g": r2(e.mass_g), "filament_m": r2(e.filament_m), "cost": r2(e.cost), "shrinkage_percent": f.shrinkage})
        })
        .collect();
    let cheapest = rows.iter().filter_map(|r| Some((r["cost"].as_f64()?, r["material"].as_str()?))).min_by(|a, b| a.0.total_cmp(&b.0));
    let summary = cheapest.map_or("No filaments known".into(), |(c, n)| format!("Cheapest at typical prices: {n}, {c:.2}"));
    Ok(json!({"summary": summary, "materials": rows}))
}

pub(super) fn check(s: &mut Session, a: &Args) -> Result<Value> {
    let names = a.bodies(s)?;
    let m = mesh_of(s, &names)?;
    let p = setup();
    let angle = overhang_angle(a);
    let min_wall = a.num("min_wall");
    let b = m.bounds();
    let (bed, size) = (b.min.z, b.size());
    let mut issues = Vec::new();
    let mut score = 100.0;
    let mut issue = |level: &str, cost: f64, text: String, fix: &str| {
        score -= cost;
        issues.push(json!({"level": level, "issue": text, "fix": fix}));
    };
    for n in &names {
        let t = analysis::topology(&mesh_of(s, std::slice::from_ref(n))?);
        if !t.watertight() {
            issue(
                "error",
                30.0,
                format!("`{n}` is not watertight: {} open, {} non-manifold and {} flipped edges", t.open_edges, t.non_manifold_edges, t.flipped_edges),
                "Use Repair (body.repair), or close the gaps",
            );
        }
        if t.shells > 1 {
            issue("info", 0.0, format!("`{n}` has {} separate shells", t.shells), "Join them if they are meant to be one part");
        }
    }
    if size.x > p.bed[0] || size.y > p.bed[1] || size.z > p.bed[2] {
        let text = format!("{:.0} × {:.0} × {:.0} mm is larger than {}'s {:.0} × {:.0} × {:.0}", size.x, size.y, size.z, p.printer, p.bed[0], p.bed[1], p.bed[2]);
        issue("error", 25.0, text, "Use Scale to Bed or Split to Fit Bed");
    }
    let (over, over_area) = analysis::overhangs(&m, angle, bed);
    let share = over_area / m.area().max(1e-9) * 100.0;
    if share > 0.05 {
        let level = if share > 10.0 { "warning" } else { "info" };
        issue(level, share.min(20.0), format!("{:.1} cm² ({share:.1}%) overhangs more than {angle:.0}° allows", over_area / 100.0), "Use Auto Orient, chamfer it, or add supports");
    }
    let thin = analysis::thickness(&m, min_wall);
    if !thin.thin.is_empty() {
        let cost = if thin.min < p.nozzle { 20.0 } else { 8.0 };
        issue(
            if thin.min < p.nozzle { "error" } else { "warning" },
            cost,
            format!("{:.1} cm² is thinner than {min_wall} mm (thinnest {:.2} mm)", thin.thin_area / 100.0, thin.min),
            "Thicken it to at least two line widths",
        );
    }
    let sl = Slicer::new(&m, p.layer);
    let isles = slice::islands(&sl, 0.5);
    if let Some(first) = isles.first() {
        issue("warning", 12.0, format!("{} island(s) start in mid-air, the first at Z {:.2} mm", isles.len(), first.z), "They need supports");
    }
    let contact = analysis::contact_area(&m, bed);
    if contact < 25.0 || contact / (size.x * size.y).max(1e-9) < 0.02 {
        issue("warning", 12.0, format!("Only {contact:.0} mm² touches the bed"), "Use Auto Orient, Lay Face Down, or Brim Ears");
    }
    let tip = analysis::tip_angle(&m, bed);
    if tip < 8.0 {
        issue("warning", 10.0, format!("It tips over at {tip:.0}°"), "Lay it lower, or add a brim");
    }
    if bed.abs() > 0.01 {
        issue("info", 0.0, format!("Its lowest point is at Z {bed:.2} mm"), "Use Drop to Bed");
    }
    if sl.layer > p.layer * 1.001 {
        issue("info", 0.0, "It was checked with thicker layers, being very tall".into(), "");
    }
    let score = score.clamp(0.0, 100.0_f64).round();
    let mut o = PrintOverlay { label: format!("Printability {score:.0}/100"), ..PrintOverlay::default() };
    paint(&mut o, &m, &thin.thin, OverlayRole::Thin);
    paint(&mut o, &m, &over, OverlayRole::Overhang);
    o.points.extend(isles.iter().map(|i| (OverlayRole::Island, Vec3::new(i.x, i.y, i.z))));
    show(s, o);
    let word = match score {
        x if x >= 90.0 => "ready to print",
        x if x >= 70.0 => "printable with care",
        x if x >= 45.0 => "needs work",
        _ => "not printable as it is",
    };
    let real = issues.iter().filter(|i| i["level"] != "info").count();
    Ok(json!({
        "summary": format!("{score:.0}/100: {word}, {real} issue(s)"), "score": score, "issues": issues,
        "size_mm": [r2(size.x), r2(size.y), r2(size.z)], "volume_cm3": r2(m.volume() / 1000.0), "overhang_area_mm2": r2(over_area),
        "thinnest_mm": r2(thin.min), "islands": isles.len(), "contact_area_mm2": r2(contact), "tip_angle_deg": r2(tip),
    }))
}

// ---- moving bodies -----------------------------------------------------------------------------

fn shift(s: &mut Session, body: &str, by: Vec3) -> Result<()> {
    if by.len() < 1e-7 {
        return Ok(());
    }
    s.execute("solid.move", &json!({"bodies": [body], "translate": [mm(by.x), mm(by.y), mm(by.z)]})).map(|_| ())
}

/// Turn a body so `down` faces the bed, set it on Z = 0, and keep its middle where it was.
fn stand(s: &mut Session, body: &str, down: Vec3) -> Result<()> {
    let before = mesh_of(s, &[body.to_string()])?;
    let c = before.bounds().center();
    let (axis, angle) = axis_angle(down);
    let turned: rybocraft_geom::Aabb3 = before.positions.iter().map(|p| rotate(*p, axis, angle)).collect();
    if angle > 1e-9 {
        s.execute("solid.move", &json!({"bodies": [body], "axis": xyz(axis), "angle": format!("{} deg", angle.to_degrees())}))?;
    }
    let t = turned.center();
    shift(s, body, Vec3::new(c.x - t.x, c.y - t.y, -turned.min.z))
}

pub(super) fn auto_orient(s: &mut Session, a: &Args) -> Result<Value> {
    let w = match a.text("priority") {
        "least_support" => Weights { support: 3.0, height: 0.3, contact: 0.4 },
        "fastest" => Weights { support: 0.8, height: 3.0, contact: 0.4 },
        "best_adhesion" => Weights { support: 0.6, height: 0.5, contact: 3.0 },
        _ => Weights { support: 1.2, height: 1.0, contact: 0.9 },
    };
    let angle = overhang_angle(a);
    let mut made = Vec::new();
    for n in a.bodies(s)? {
        let m = mesh_of(s, std::slice::from_ref(&n))?;
        let before = analysis::overhangs(&m, angle, m.bounds().min.z).1;
        let list = orientations(&m, angle, w);
        let best = list.first().ok_or_else(|| a.fail(format!("`{n}` has no surface")))?;
        let down = Vec3::new(best.down[0], best.down[1], best.down[2]);
        stand(s, &n, down)?;
        made.push(json!({"body": n, "overhang_before_mm2": r2(before), "overhang_after_mm2": r2(best.overhang_area), "height_mm": r2(best.height), "contact_area_mm2": r2(best.contact_area), "turned": down.z > -1.0 + 1e-9}));
    }
    s.print_overlay = None;
    let turned = made.iter().filter(|m| m["turned"] == true).count();
    Ok(json!({"summary": format!("{turned} of {} bodies turned; all set on the bed", made.len()), "bodies": made}))
}

pub(super) fn lay_flat(s: &mut Session, a: &Args) -> Result<Value> {
    let (body, _, n) = a.spot(s)?;
    stand(s, &body, n)?;
    Ok(json!({"summary": format!("`{body}` lies on that face"), "body": body}))
}

pub(super) fn drop_to_bed(s: &mut Session, a: &Args) -> Result<Value> {
    let names = a.bodies(s)?;
    let all = mesh_of(s, &names)?.bounds().center();
    for n in &names {
        let b = mesh_of(s, std::slice::from_ref(n))?.bounds();
        let side = if a.flag("center") { Vec3::new(-all.x, -all.y, 0.0) } else { Vec3::new(0.0, 0.0, 0.0) };
        shift(s, n, side + Vec3::new(0.0, 0.0, -b.min.z))?;
    }
    Ok(json!({"summary": format!("{} bodies on the bed", names.len()), "bodies": names}))
}

pub(super) fn arrange(s: &mut Session, a: &Args) -> Result<Value> {
    let names = a.bodies(s)?;
    let p = setup();
    let mut boxes = Vec::new();
    for n in &names {
        boxes.push(mesh_of(s, std::slice::from_ref(n))?.bounds());
    }
    let sizes: Vec<[f64; 2]> = boxes.iter().map(|b| [b.size().x, b.size().y]).collect();
    let places = pack::arrange(&sizes, [p.bed[0], p.bed[1]], a.num("gap"), a.num("margin"));
    let (mut plates, mut warnings, mut out) = (0, Vec::new(), Vec::new());
    for ((n, b), place) in names.iter().zip(&boxes).zip(&places) {
        let Some((plate, at)) = place else {
            warnings.push(format!("`{n}` is larger than the bed and was left where it is"));
            continue;
        };
        plates = plates.max(plate + 1);
        // The bed is centred on the origin; more plates stand to the right.
        let x = at[0] - p.bed[0] * 0.5 + *plate as f64 * (p.bed[0] + 20.0);
        let y = at[1] - p.bed[1] * 0.5;
        shift(s, n, Vec3::new(x - b.min.x, y - b.min.y, -b.min.z))?;
        out.push(json!({"body": n, "plate": plate + 1}));
    }
    Ok(json!({"summary": format!("{} bodies on {plates} plate(s) of {}", out.len(), p.printer), "bodies": out, "plates": plates, "warnings": warnings}))
}

pub(super) fn fit_to_bed(s: &mut Session, a: &Args) -> Result<Value> {
    let names = a.bodies(s)?;
    let p = setup();
    let b = mesh_of(s, &names)?.bounds();
    let (size, m) = (b.size(), a.num("margin"));
    let room = [p.bed[0] - 2.0 * m, p.bed[1] - 2.0 * m, p.bed[2] - m];
    let k = [room[0] / size.x, room[1] / size.y, room[2] / size.z].into_iter().fold(f64::INFINITY, f64::min);
    if !(k.is_finite() && k > 0.0) {
        return Err(a.fail("the margin leaves no room on the bed"));
    }
    if k >= 1.0 && !a.flag("enlarge") {
        return Ok(json!({"summary": "Already fits: nothing was scaled", "factor": 1.0}));
    }
    s.execute("solid.scale", &json!({"bodies": names, "factor": k, "origin": [b.center().x, b.center().y, b.min.z]}))?;
    Ok(json!({"summary": format!("Scaled to {:.1}% to fit {}", k * 100.0, p.printer), "factor": k, "bodies": names}))
}

pub(super) fn shrinkage(s: &mut Session, a: &Args) -> Result<Value> {
    let names = a.bodies(s)?;
    let f = setup().filament();
    let xy = if a.num("percent") >= 0.0 { a.num("percent") } else { f.shrinkage };
    let z = if a.num("percent_z") >= 0.0 { a.num("percent_z") } else { xy };
    let b = mesh_of(s, &names)?.bounds();
    let k = |p: f64| 1.0 / (1.0 - p / 100.0);
    let origin = json!([b.center().x, b.center().y, b.min.z]);
    // An even scale keeps every surface kind; an uneven one is only made when it has to be.
    let p = if (xy - z).abs() < 1e-12 {
        json!({"bodies": names, "factor": k(xy), "origin": origin})
    } else {
        json!({"bodies": names, "factors": [k(xy), k(xy), k(z)], "origin": origin})
    };
    s.execute("solid.scale", &p)?;
    Ok(json!({"summary": format!("Scaled up for {xy}% (Z {z}%) shrinkage"), "factor_xy": k(xy), "factor_z": k(z), "bodies": names}))
}

pub(super) fn supports(s: &mut Session, a: &Args) -> Result<Value> {
    let m = mesh_of(s, &a.bodies(s)?)?;
    let set = SupportSettings {
        angle: overhang_angle(a),
        spacing: a.num("spacing"),
        diameter: a.num("diameter"),
        tip: a.num("tip").min(a.num("diameter")),
        gap: a.num("gap"),
        foot: a.num("foot"),
        bed_only: a.flag("bed_only"),
    };
    let (posts, n) = columns(&m, &set).map_err(|e| a.fail(e))?;
    if n == 0 {
        return Ok(json!({"summary": "Nothing needs support", "posts": 0, "created_bodies": []}));
    }
    let made = add_mesh_body(s, "Supports", &posts)?;
    Ok(json!({"summary": format!("{n} support posts, {:.1} cm³", posts.volume() / 1000.0), "posts": n, "volume_cm3": r2(posts.volume() / 1000.0), "created_bodies": made}))
}

pub(super) fn mouse_ears(s: &mut Session, a: &Args) -> Result<Value> {
    let body = a.body(s)?;
    let m = mesh_of(s, std::slice::from_ref(&body))?;
    let bed = m.bounds().min.z;
    let d = a.num("diameter");
    let hull = analysis::convex_hull(m.positions.iter().filter(|p| p.z <= bed + analysis::ON_BED).map(|p| [p.x, p.y]).collect());
    let n = hull.len();
    let mut ears: Vec<[f64; 2]> = Vec::new();
    for i in 0..n {
        let (p, q, r) = (hull[(i + n - 1) % n], hull[i], hull[(i + 1) % n]);
        let turn = ((r[1] - q[1]).atan2(r[0] - q[0]) - (q[1] - p[1]).atan2(q[0] - p[0])).rem_euclid(std::f64::consts::TAU);
        if turn > 25f64.to_radians() && ears.iter().all(|e| (e[0] - q[0]).hypot(e[1] - q[1]) > d * 0.6) {
            ears.push(q);
        }
    }
    if ears.is_empty() {
        // A round footprint has no corners: six around it.
        ears = (0..6).filter_map(|k| hull.get(k * n / 6).copied()).collect();
        ears.dedup();
    }
    if ears.is_empty() {
        return Err(a.fail("the body does not stand on the bed"));
    }
    if ears.len() > 40 {
        return Err(a.fail("the outline has too many corners for ears"));
    }
    let up = Vec3::new(0.0, 0.0, 1.0);
    let mut own: Option<String> = None;
    let mut created_bodies = Vec::new();
    for e in &ears {
        let at = Vec3::new(e[0], e[1], bed);
        if a.flag("separate") {
            let made = cylinder(s, at, up, d, a.num("thickness"), Do::New("Brim ear"))?;
            own.get_or_insert_with(|| made.clone());
            created_bodies.push(made);
        } else {
            cylinder(s, at, up, d, a.num("thickness"), Do::Join(&body))?;
        }
    }
    Ok(json!({"summary": format!("{} brim ears on `{body}`", ears.len()), "ears": ears.len(), "created_bodies": created_bodies}))
}

pub(super) fn split_to_fit(s: &mut Session, a: &Args) -> Result<Value> {
    let body = a.body(s)?;
    let p = setup();
    let margin = a.num("margin");
    let start = mesh_of(s, std::slice::from_ref(&body))?.bounds();
    let mut pieces = vec![body.clone()];
    let mut pins = 0usize;
    let (d, depth, clear) = (a.num("pin_diameter"), a.num("pin_depth"), a.num("clearance"));
    for axis in 0..3 {
        let unit = [Vec3::new(1., 0., 0.), Vec3::new(0., 1., 0.), Vec3::new(0., 0., 1.)][axis];
        let room = p.bed[axis] - 2.0 * margin;
        let size = start.size().dot(unit);
        if room <= 1.0 {
            return Err(a.fail("the margin leaves no room on the bed"));
        }
        let n = (size / room - 1e-9).ceil().max(1.0) as usize;
        if n > 12 {
            return Err(a.fail("it would take more than 12 pieces along one axis: scale it down first"));
        }
        for k in 1..n {
            let at = start.min.dot(unit) + size * k as f64 / n as f64;
            let origin = unit * at;
            for piece in pieces.clone() {
                let m = mesh_of(s, std::slice::from_ref(&piece))?;
                let b = m.bounds();
                if b.min.dot(unit) > at - 1e-3 || b.max.dot(unit) < at + 1e-3 {
                    continue;
                }
                let spots = pin_spots(&m, axis, at, d, depth, a.int("pins"));
                let before: Vec<String> = s.world_state()?.bodies.iter().map(|b| b.name.clone()).collect();
                s.execute("solid.split_body", &json!({"body": piece, "plane": {"origin": xyz(origin), "normal": xyz(unit)}}))?;
                let after = s.world_state()?;
                let new: Vec<String> = after.bodies.iter().map(|b| b.name.clone()).filter(|n| !before.contains(n)).collect();
                let mut halves = new.clone();
                if after.body(&piece).is_some() {
                    halves.push(piece.clone());
                } else {
                    pieces.retain(|x| *x != piece);
                }
                for q in &spots {
                    s.execute(
                        "solid.cylinder",
                        &json!({"diameter": mm(d + clear), "height": mm(2.0 * depth), "base": xyz(*q - unit * depth), "axis": xyz(unit), "operation": "cut", "targets": halves}),
                    )?;
                    pins += 1;
                }
                pieces.extend(new);
            }
        }
    }
    if pieces.len() < 2 {
        return Ok(json!({"summary": "It already fits: nothing was cut", "created_bodies": []}));
    }
    // The pins stand beside the part, a little shorter than their holes.
    let mut made = Vec::new();
    for i in 0..pins {
        let at = Vec3::new(start.max.x + 10.0 + d + (i % 8) as f64 * (d + 4.0), start.min.y + (i / 8) as f64 * (d + 4.0), start.min.z);
        made.push(cylinder(s, at, Vec3::new(0., 0., 1.), d, 2.0 * depth - 0.6, Do::New("Pin"))?);
    }
    Ok(json!({"summary": format!("{} pieces and {pins} alignment pins", pieces.len()), "pieces": pieces, "pins": pins, "created_bodies": made}))
}

/// Where pins fit across the plane `axis = at`: as far apart as can be.
fn pin_spots(m: &PrintMesh, axis: usize, at: f64, d: f64, depth: f64, want: usize) -> Vec<Vec3> {
    if want == 0 {
        return Vec::new();
    }
    let b = m.bounds();
    let solid = rybocraft_geom::Mesh { positions: m.positions.clone(), triangles: m.triangles.clone(), ..Default::default() };
    let inside = solid.inside_index();
    let place = |u: f64, v: f64, w: f64| match axis {
        0 => Vec3::new(w, u, v),
        1 => Vec3::new(v, w, u),
        _ => Vec3::new(u, v, w),
    };
    let (lo, hi) = match axis {
        0 => ([b.min.y, b.min.z], [b.max.y, b.max.z]),
        1 => ([b.min.z, b.min.x], [b.max.z, b.max.x]),
        _ => ([b.min.x, b.min.y], [b.max.x, b.max.y]),
    };
    // Room for the pin and a wall of 1.2 mm all round it, through both sides.
    let r = d * 0.5 + 1.2;
    let fits = |u: f64, v: f64| {
        [-depth - 1.0, -depth * 0.5, 0.0, depth * 0.5, depth + 1.0].iter().all(|w| {
            inside.contains(place(u, v, at + w))
                && (0..12).all(|k| {
                    let (sn, cs) = (f64::from(k) * std::f64::consts::TAU / 12.0).sin_cos();
                    inside.contains(place(u + r * cs, v + r * sn, at + w))
                })
        })
    };
    const N: usize = 24;
    let mut spots: Vec<[f64; 2]> = Vec::new();
    for i in 0..N {
        for j in 0..N {
            let u = lo[0] + (hi[0] - lo[0]) * (i as f64 + 0.5) / N as f64;
            let v = lo[1] + (hi[1] - lo[1]) * (j as f64 + 0.5) / N as f64;
            if fits(u, v) {
                spots.push([u, v]);
            }
        }
    }
    let mid = [(lo[0] + hi[0]) * 0.5, (lo[1] + hi[1]) * 0.5];
    let mut chosen: Vec<[f64; 2]> = Vec::new();
    while chosen.len() < want {
        let apart = |p: &[f64; 2]| {
            if chosen.is_empty() {
                (p[0] - mid[0]).hypot(p[1] - mid[1])
            } else {
                chosen.iter().map(|c| (p[0] - c[0]).hypot(p[1] - c[1])).fold(f64::INFINITY, f64::min)
            }
        };
        match spots.iter().max_by(|a, b| apart(a).total_cmp(&apart(b))) {
            Some(p) if chosen.is_empty() || apart(p) > d + 3.0 => {
                let p = *p;
                chosen.push(p);
            }
            _ => break,
        }
    }
    chosen.into_iter().map(|[u, v]| place(u, v, at)).collect()
}

/// The flattest large face the body could lie on; used by tests.
#[cfg(test)]
pub(super) fn bed_area(s: &Session, body: &str) -> f64 {
    let m = mesh_of(s, &[body.to_string()]).unwrap();
    let z = m.bounds().min.z;
    m.tris().filter(|t| t.iter().all(|p| p.z < z + 0.01)).map(|t| rybocraft_print3d::normal_area(t).1).sum()
}
