//! Features only printed parts want: insert holes, nut traps, teardrops, lattices.

use rybocraft_print3d::lattice::{LatticeSettings, Pattern};
use serde_json::{Value, json};
use rybocore_record::Vec3;
use rybocraft_geom::prelude::*;

use super::*;

/// Cuts start this far outside the face, so no skin is left.
const OVER: f64 = 0.05;

fn done(summary: String, body: &str) -> Result<Value> {
    Ok(json!({"summary": summary, "body": body}))
}

pub(super) fn insert_hole(s: &mut Session, a: &Args) -> Result<Value> {
    let (body, mut p, n) = a.spot(s)?;
    let f = a.fastener()?;
    let d = if a.num("diameter") > 0.0 { a.num("diameter") } else { f.insert[0] };
    let depth = if a.num("depth") > 0.0 { a.num("depth") } else { f.insert[1] };
    let boss = a.num("boss");
    if boss > 0.0 {
        cylinder(s, p - n * 0.2, n, d + 2.0 * (d * 0.35).max(1.6), boss + 0.2, Do::Join(&body))?;
        p = p + n * boss;
    }
    cylinder(s, p + n * OVER, n * -1.0, d, depth + OVER, Do::Cut(&body))?;
    // A lead-in that also takes the plastic the insert pushes up.
    let lead = 0.6;
    cone(s, p + n * OVER, n * -1.0, d + 2.0 * (lead + OVER) * 0.8, d, lead + OVER, Do::Cut(&body))?;
    done(format!("{} insert hole: {d} mm, {depth} mm deep", f.size), &body)
}

pub(super) fn nut_trap(s: &mut Session, a: &Args) -> Result<Value> {
    let (body, p, n) = a.spot(s)?;
    let f = a.fastener()?;
    let flats = f.nut[0] + a.num("clearance");
    let depth = if a.num("depth") > 0.0 { a.num("depth") } else { f.nut[1] + 0.3 };
    let into = n * -1.0;
    prism(s, Flat::into_face(p + n * OVER, into), &hexagon(flats, [0.0, 0.0]), depth + OVER, Do::Cut(&body))?;
    let bolt = a.num("bolt_depth");
    if bolt > 0.0 {
        let bridge = a.num("bridge");
        let from = if bridge > 0.0 { depth + bridge } else { depth - OVER };
        cylinder(s, p + into * from, into, f.clearance, bolt, Do::Cut(&body))?;
    }
    done(format!("{} nut trap: {flats:.2} mm across flats, {depth:.2} mm deep", f.size), &body)
}

pub(super) fn screw_hole(s: &mut Session, a: &Args) -> Result<Value> {
    let (body, p, n) = a.spot(s)?;
    let f = a.fastener()?;
    let d = if a.text("fit") == "self_tap" { f.self_tap } else { f.clearance } + a.num("extra");
    let (into, depth) = (n * -1.0, a.num("depth"));
    if d <= 0.1 {
        return Err(a.fail("the hole would have no diameter"));
    }
    match a.text("head") {
        "counterbore" => {
            let (hd, hh) = (f.head[0] + 0.6, f.head[1] + 0.3);
            cylinder(s, p + n * OVER, into, hd, hh + OVER, Do::Cut(&body))?;
            let bridge = a.num("bridge");
            let from = if bridge > 0.0 { hh + bridge } else { hh - OVER };
            if depth > from {
                cylinder(s, p + into * from, into, d, depth - from, Do::Cut(&body))?;
            }
        }
        "countersink" => {
            cylinder(s, p + n * OVER, into, d, depth + OVER, Do::Cut(&body))?;
            let top = f.nominal * 2.0 + 0.4;
            cone(s, p + n * OVER, into, top + 2.0 * OVER, d, (top - d) * 0.5 + OVER, Do::Cut(&body))?;
        }
        _ => {
            cylinder(s, p + n * OVER, into, d, depth + OVER, Do::Cut(&body))?;
        }
    }
    done(format!("{} {} hole: {d:.2} mm", f.size, a.text("fit").replace('_', "-")), &body)
}

pub(super) fn magnet_pocket(s: &mut Session, a: &Args) -> Result<Value> {
    let (body, p, n) = a.spot(s)?;
    let (c, t, cover) = (a.num("clearance"), a.num("thickness"), a.num("cover"));
    let into = n * -1.0;
    // Buried, the pocket starts below the cover; open, it runs out through the face.
    let (from, h) = if cover > 0.0 { (p + into * cover, t + 0.2) } else { (p + n * OVER, t + 0.1 + OVER) };
    if a.text("shape") == "block" {
        let (w, l) = ((a.num("diameter") + c) * 0.5, (a.num("width") + c) * 0.5);
        prism(s, Flat::into_face(from, into), &[[-w, -l], [w, -l], [w, l], [-w, l]], h, Do::Cut(&body))?;
    } else {
        cylinder(s, from, into, a.num("diameter") + c, h, Do::Cut(&body))?;
    }
    let note = if cover > 0.0 { format!(", buried under {cover} mm: pause the print to drop it in") } else { String::new() };
    done(format!("Magnet pocket for {} × {t} mm{note}", a.num("diameter")), &body)
}

pub(super) fn teardrop(s: &mut Session, a: &Args) -> Result<Value> {
    let (body, p, n) = a.spot(s)?;
    if n.z.abs() > 0.8 {
        return Err(a.fail("a teardrop is for sideways holes: pick an upright face (an upright hole prints round)"));
    }
    let (d, depth) = (a.num("diameter"), a.num("depth"));
    let (r, into) = (d * 0.5, n * -1.0);
    let k = r * std::f64::consts::FRAC_1_SQRT_2;
    let peak = r * std::f64::consts::SQRT_2;
    let points = if a.flag("flat") {
        let top = r + (peak - r) * 0.4;
        let half = peak - top;
        vec![[-k, k], [0.0, 0.0], [k, k], [half, top], [-half, top]]
    } else {
        vec![[-k, k], [0.0, 0.0], [k, k], [0.0, peak]]
    };
    let from = p + n * OVER;
    cylinder(s, from, into, d, depth + OVER, Do::Cut(&body))?;
    prism(s, Flat::into_face(from, into), &points, depth + OVER, Do::Cut(&body))?;
    done(format!("Teardrop hole, {d} mm"), &body)
}

pub(super) fn lattice(s: &mut Session, a: &Args) -> Result<Value> {
    let body = a.body(s)?;
    let m = mesh_of(s, std::slice::from_ref(&body))?;
    let set = LatticeSettings {
        pattern: match a.text("pattern") {
            "diamond" => Pattern::Diamond,
            "schwarz" => Pattern::Schwarz,
            "cubic" => Pattern::Cubic,
            _ => Pattern::Gyroid,
        },
        cell: a.num("cell"),
        wall: a.num("wall"),
        skin: a.num("skin"),
        resolution: a.num("resolution"),
    };
    let mut warnings = Vec::new();
    if set.resolution > set.wall * 0.8 {
        warnings.push("the resolution is coarse for this wall: lower it for a cleaner lattice");
    }
    let filled = rybocraft_print3d::lattice::lattice(&m, &set).map_err(|e| a.fail(e))?;
    if filled.triangles.is_empty() {
        return Err(a.fail("nothing is left: use a thicker wall or a smaller cell"));
    }
    let made = add_mesh_body(s, &format!("{body} lattice"), &filled)?;
    let saved = (1.0 - filled.volume() / m.volume().max(1e-9)) * 100.0;
    // The original stays in the design, out of sight.
    if s.execute("browser.visibility", &json!({"items": [body], "visible": false})).is_err() {
        warnings.push("the original is still shown: hide it before exporting");
    }
    Ok(json!({"summary": format!("Lattice of `{body}`: {saved:.0}% lighter, {} triangles", filled.triangles.len()), "saved_percent": saved, "created_bodies": made, "warnings": warnings}))
}

pub(super) fn hollow(s: &mut Session, a: &Args) -> Result<Value> {
    let body = a.body(s)?;
    let m = mesh_of(s, std::slice::from_ref(&body))?;
    let before = m.volume();
    let b = m.bounds();
    let wall = a.num("wall");
    if s.execute("solid.shell", &json!({"faces": [], "thickness": mm(wall), "direction": "inside", "body": body})).is_err() {
        // Shapes the kernel cannot shell are hollowed as a mesh.
        let open = if a.int("drains") > 0 { "bottom" } else { "none" };
        return super::super::maker::hollow_mesh(s, a, wall, open, a.num("diameter"), 0.5);
    }
    let n = a.int("drains");
    let hollowed = mesh_of(s, std::slice::from_ref(&body))?;
    let mut made = 0;
    if n > 0 {
        // Holes go up where there is floor below and hollow above.
        let solid = rybocraft_geom::Mesh { positions: hollowed.positions.clone(), triangles: hollowed.triangles.clone(), ..Default::default() };
        let inside = solid.inside_index();
        let mut spots: Vec<[f64; 2]> = Vec::new();
        for i in 0..24 {
            for j in 0..24 {
                let x = b.min.x + b.size().x * (f64::from(i) + 0.5) / 24.0;
                let y = b.min.y + b.size().y * (f64::from(j) + 0.5) / 24.0;
                let up = |z: f64| Vec3::new(x, y, b.min.z + z);
                if inside.contains(up(wall * 0.5)) && !inside.contains(up(wall * 1.3)) {
                    spots.push([x, y]);
                }
            }
        }
        let c = b.center();
        let mut chosen: Vec<[f64; 2]> = Vec::new();
        while chosen.len() < n {
            let apart = |p: &[f64; 2]| {
                if chosen.is_empty() {
                    (p[0] - c.x).hypot(p[1] - c.y)
                } else {
                    chosen.iter().map(|q| (p[0] - q[0]).hypot(p[1] - q[1])).fold(f64::INFINITY, f64::min)
                }
            };
            match spots.iter().max_by(|p, q| apart(p).total_cmp(&apart(q))) {
                Some(p) if chosen.is_empty() || apart(p) > a.num("diameter") + 1.0 => {
                    let p = *p;
                    chosen.push(p);
                }
                _ => break,
            }
        }
        for [x, y] in &chosen {
            cylinder(s, Vec3::new(*x, *y, b.min.z - OVER), Vec3::new(0., 0., 1.), a.num("diameter"), wall * 1.3 + OVER, Do::Cut(&body))?;
            made += 1;
        }
    }
    let after = mesh_of(s, std::slice::from_ref(&body))?.volume();
    let saved = (1.0 - after / before.max(1e-9)) * 100.0;
    let warnings = if made < n { vec![format!("only {made} drain hole(s) fit the bottom")] } else { Vec::new() };
    Ok(json!({"summary": format!("`{body}` hollowed: {saved:.0}% less material, {made} drain hole(s)"), "saved_percent": saved, "drains": made, "warnings": warnings}))
}

pub(super) fn vent(s: &mut Session, a: &Args) -> Result<Value> {
    let (body, p, n) = a.spot(s)?;
    let (w, h, hex, rib) = (a.num("width"), a.num("height"), a.num("hex"), a.num("rib"));
    let pitch = hex + rib;
    let row = pitch * 3f64.sqrt() * 0.5;
    let r = hex / 3f64.sqrt();
    let mut cells = Vec::new();
    let rows = ((h * 0.5 - r) / row).floor();
    let cols = ((w * 0.5 - hex * 0.5) / pitch).floor() + 1.0;
    if rows < 0.0 || cols < 1.0 {
        return Err(a.fail("the grille is too small for one hexagon"));
    }
    for j in -(rows as i64)..=rows as i64 {
        let off = if j.rem_euclid(2) == 1 { pitch * 0.5 } else { 0.0 };
        for i in -(cols as i64)..=cols as i64 {
            let x = i as f64 * pitch + off;
            if x.abs() + hex * 0.5 <= w * 0.5 + 1e-9 {
                cells.push(hexagon(hex, [x, j as f64 * row]));
            }
        }
    }
    if cells.len() > 150 {
        return Err(a.fail(format!("{} hexagons is too many: use larger ones", cells.len())));
    }
    let into = n * -1.0;
    sketched(s, Flat::into_face(p + n * OVER, into), a.num("depth") + OVER, 0.0, Do::Cut(&body), |s| {
        cells.iter().try_for_each(|c| polygon(s, c))
    })?;
    let open = cells.len() as f64 * hex * hex * 3f64.sqrt() * 0.5;
    Ok(json!({"summary": format!("{} hexagons, {open:.0} mm² open", cells.len()), "cells": cells.len(), "open_area_mm2": open, "body": body}))
}

pub(super) fn keyhole(s: &mut Session, a: &Args) -> Result<Value> {
    let (body, p, n) = a.spot(s)?;
    let (head, shank, travel, room, lip) = (a.num("head") + 0.6, a.num("shank") + 0.4, a.num("travel"), a.num("head_depth"), a.num("lip"));
    if shank >= head {
        return Err(a.fail("the shank must be thinner than the head"));
    }
    let into = n * -1.0;
    let flat = Flat::into_face(p + n * OVER, into);
    let up = flat.y * travel;
    // In through the round hole, then slide: the slot runs up from it.
    cylinder(s, p + n * OVER, into, head, lip + room + OVER, Do::Cut(&body))?;
    cylinder(s, p + up + n * OVER, into, shank, lip + OVER + 0.1, Do::Cut(&body))?;
    let half = shank * 0.5;
    prism(s, flat, &[[-half, 0.0], [half, 0.0], [half, travel], [-half, travel]], lip + OVER + 0.1, Do::Cut(&body))?;
    cylinder(s, p + up + into * lip, into, head, room, Do::Cut(&body))?;
    let half = head * 0.5;
    prism(s, Flat::into_face(p + into * lip, into), &[[-half, 0.0], [half, 0.0], [half, travel], [-half, travel]], room, Do::Cut(&body))?;
    done("Keyhole hanger".to_string(), &body)
}
