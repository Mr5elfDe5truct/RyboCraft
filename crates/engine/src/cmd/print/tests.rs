use super::*;
use crate::schema::ParamCheck;

fn run(s: &mut Session, id: &str, p: Value) -> Value {
    s.execute(id, &p).unwrap_or_else(|e| panic!("{id}: {e}"))
}

fn block() -> Session {
    let mut s = Session::default();
    run(&mut s, "solid.box", json!({"length": 40, "width": 30, "height": 20, "corner": [0, 0, 0], "body_name": "Block"}));
    s
}

fn volume(s: &Session, body: &str) -> f64 {
    let m = mesh_of(s, &[body.to_string()]).unwrap();
    let t = rybocraft_print3d::analysis::topology(&m);
    assert!(t.watertight(), "{body}: {t:?}");
    t.volume
}

fn near(a: f64, b: f64, tol: f64) {
    assert!((a - b).abs() <= tol, "{a} is not {b}");
}

#[test]
fn every_tool_is_a_command_with_defaults_its_own_schema_takes() {
    for t in TOOLS {
        let spec = crate::cmd::find_command(t.id).unwrap_or_else(|| panic!("{}", t.id));
        assert!(PANELS.contains(&t.panel) && spec.tab == "3D PRINT", "{}", t.id);
        for f in t.fields {
            let d = match f.kind {
                FieldKind::Length { default, .. } | FieldKind::Number { default, .. } => json!(default),
                FieldKind::Int { default, .. } => json!(default),
                FieldKind::Bool(b) => json!(b),
                FieldKind::Choice { default, .. } => json!(default),
                _ => continue,
            };
            shape(f, &d).unwrap_or_else(|e| panic!("{} {}: {e}", t.id, f.key));
        }
    }
}

#[test]
fn bad_parameters_are_refused_and_change_nothing() {
    let mut s = block();
    let undo = s.undo.len();
    for p in [json!({"wall": -1}), json!({"pattern": "foam"}), json!({"wall": "nonsense"}), json!({"body": "Nope"})] {
        assert!(s.execute("print.lattice", &p).is_err(), "{p}");
    }
    s.param_check = ParamCheck::Strict;
    assert!(s.execute("print.estimate", &json!({"no_such": 1})).is_err());
    assert_eq!(s.undo.len(), undo);
}

#[test]
fn a_block_is_printable_and_a_table_is_not_quite() {
    let mut s = block();
    let r = run(&mut s, "print.check", json!({}));
    assert_eq!(r["score"], 100.0, "{r}");
    run(&mut s, "solid.box", json!({"length": 100, "width": 90, "height": 3, "corner": [-30, -30, 20], "operation": "join", "targets": ["Block"]}));
    let r = run(&mut s, "print.check", json!({}));
    assert!(r["score"].as_f64().unwrap() < 90.0, "{r}");
    assert!(s.print_overlay.as_ref().is_some_and(|o| !o.triangles.is_empty()));
    let r = run(&mut s, "print.overhangs", json!({}));
    near(r["overhang_area_mm2"].as_f64().unwrap(), 9000.0 - 1200.0, 0.1);
    let r = run(&mut s, "print.auto_orient", json!({}));
    assert_eq!(r["bodies"][0]["turned"], true, "{r}");
    near(r["bodies"][0]["overhang_after_mm2"].as_f64().unwrap(), 0.0, 1e-6);
    let b = mesh_of(&s, &["Block".into()]).unwrap().bounds();
    near(b.min.z, 0.0, 1e-6);
    near(prepare::bed_area(&s, "Block"), 9000.0, 1.0);
}

#[test]
fn analyses_do_not_touch_the_design() {
    let mut s = block();
    let (doc, undo) = (s.doc.clone(), s.undo.len());
    for t in TOOLS.iter().filter(|t| t.reads_only && t.id != "print.setup") {
        run(&mut s, t.id, json!({}));
    }
    assert!(Arc::ptr_eq(&doc, &s.doc) && s.undo.len() == undo);
}

#[test]
fn holes_take_out_what_they_say_and_undo_in_one_step() {
    let top = json!([20, 15, 20]);
    let pi = std::f64::consts::PI;
    let mut s = block();
    let undo = s.undo.len();
    run(&mut s, "print.magnet_pocket", json!({"body": "Block", "point": top, "diameter": 6, "thickness": 2, "clearance": 0}));
    near(volume(&s, "Block"), 24000.0 - pi * 9.0 * 2.1, 0.5);
    assert_eq!(s.undo.len(), undo + 1);
    run(&mut s, "edit.undo", json!({}));
    near(volume(&s, "Block"), 24000.0, 1e-6);
    run(&mut s, "print.nut_trap", json!({"point": top, "size": "M3", "clearance": 0, "depth": 3, "bolt_depth": 0}));
    near(volume(&s, "Block"), 24000.0 - 5.5 * 5.5 * 3f64.sqrt() / 2.0 * 3.0, 0.2);
    run(&mut s, "edit.undo", json!({}));
    for (id, p) in [
        ("print.insert_hole", json!({"point": top, "size": "M4"})),
        ("print.insert_hole", json!({"point": top, "size": "M3", "boss": 6})),
        ("print.nut_trap", json!({"point": top, "bridge": 0.2})),
        ("print.screw_hole", json!({"point": top, "head": "counterbore", "bridge": 0.2})),
        ("print.screw_hole", json!({"point": top, "head": "countersink"})),
        ("print.screw_hole", json!({"point": top, "fit": "self_tap"})),
        ("print.magnet_pocket", json!({"point": top, "shape": "block", "cover": 0.6})),
        ("print.teardrop_hole", json!({"point": [40, 15, 10], "diameter": 6})),
        ("print.teardrop_hole", json!({"point": [20, 0, 10], "diameter": 6, "flat": false, "depth": 40})),
        ("print.vent", json!({"point": [20, 30, 10], "width": 25, "height": 15})),
        ("print.keyhole", json!({"point": [20, 0, 6], "travel": 6})),
        ("print.hollow", json!({"wall": 2})),
    ] {
        run(&mut s, id, p.clone());
        let v = volume(&s, "Block");
        assert!((v - 24000.0).abs() > 5.0, "{id} {p}: {v}");
        assert_eq!(s.undo.len(), undo + 1, "{id}");
        run(&mut s, "edit.undo", json!({}));
        near(volume(&s, "Block"), 24000.0, 1e-6);
    }
    assert!(s.execute("print.teardrop_hole", &json!({"point": top})).is_err());
    assert!(s.execute("print.screw_hole", &json!({"point": [100, 100, 100]})).is_err());
}

#[test]
fn a_lattice_is_a_lighter_watertight_body() {
    let mut s = block();
    let r = run(&mut s, "print.lattice", json!({"resolution": 1.0, "wall": 1.5}));
    let name = r["created_bodies"][0].as_str().unwrap().to_string();
    assert!(volume(&s, &name) < 24000.0 * 0.75);
    assert!(!s.body_visible("Block"));
}

#[test]
fn supports_stand_under_a_table() {
    let mut s = block();
    run(&mut s, "solid.box", json!({"length": 100, "width": 90, "height": 3, "corner": [-30, -30, 20], "operation": "join", "targets": ["Block"]}));
    let r = run(&mut s, "print.supports", json!({"spacing": 8}));
    assert!(r["posts"].as_u64().unwrap() > 50, "{r}");
    volume(&s, r["created_bodies"][0].as_str().unwrap());
}

#[test]
fn a_long_part_is_split_with_pins() {
    let mut s = Session::default();
    run(&mut s, "solid.box", json!({"length": 500, "width": 40, "height": 40, "corner": [0, 0, 0], "body_name": "Beam"}));
    let r = run(&mut s, "print.split_to_fit", json!({"pins": 2}));
    assert_eq!(r["pieces"].as_array().unwrap().len(), 3, "{r}");
    assert_eq!(r["pins"], 4, "{r}");
    for p in r["pieces"].as_array().unwrap() {
        let m = mesh_of(&s, &[p.as_str().unwrap().to_string()]).unwrap();
        assert!(m.bounds().size().x < 210.0);
        volume(&s, p.as_str().unwrap());
    }
}

#[test]
fn bodies_are_arranged_dropped_scaled_and_compensated() {
    let mut s = block();
    run(&mut s, "solid.box", json!({"length": 10, "width": 10, "height": 10, "corner": [5, 5, 50], "body_name": "Small"}));
    run(&mut s, "print.arrange", json!({}));
    let a = mesh_of(&s, &["Block".into()]).unwrap().bounds();
    let b = mesh_of(&s, &["Small".into()]).unwrap().bounds();
    assert!(a.min.z.abs() < 1e-6 && b.min.z.abs() < 1e-6);
    assert!(b.min.x >= a.max.x + 4.99 || b.min.y >= a.max.y + 4.99);
    run(&mut s, "print.shrinkage", json!({"bodies": ["Small"], "percent": 1, "percent_z": 0}));
    near(volume(&s, "Small"), 1000.0 / 0.99 / 0.99, 1e-3);
    run(&mut s, "solid.box", json!({"length": 600, "width": 10, "height": 10, "corner": [0, 200, 0], "body_name": "Long"}));
    run(&mut s, "print.fit_to_bed", json!({"bodies": ["Long"]}));
    near(mesh_of(&s, &["Long".into()]).unwrap().bounds().size().x, 210.0, 1e-3);
    let end = mesh_of(&s, &["Long".into()]).unwrap().bounds();
    run(&mut s, "print.lay_flat", json!({"body": "Long", "point": [end.max.x, end.center().y, end.center().z]}));
    near(mesh_of(&s, &["Long".into()]).unwrap().bounds().size().z, 210.0, 1e-3);
    run(&mut s, "print.mouse_ears", json!({"body": "Block"}));
    assert!(volume(&s, "Block") > 24000.0);
}

#[test]
fn every_ready_made_part_is_watertight() {
    for t in TOOLS.iter().filter(|t| t.panel == "MAKE" || t.id == "print.fit_test") {
        let mut s = Session::default();
        let r = run(&mut s, t.id, json!({"at": [10, 20, 0]}));
        let names = created(&r);
        assert!(!names.is_empty(), "{}: {r}", t.id);
        for n in &names {
            assert!(volume(&s, n) > 1.0, "{} {n}", t.id);
        }
        assert_eq!(s.undo.len(), 1, "{}", t.id);
        assert_eq!(s.model.state().bodies.len(), names.len(), "{}: loose pieces", t.id);
    }
    let Some(FieldKind::Choice { options, .. }) = tool("print.calibration").and_then(|t| t.fields.iter().find(|f| f.key == "kind")).map(|f| f.kind) else {
        panic!("kinds")
    };
    for kind in options {
        let mut s = Session::default();
        let r = run(&mut s, "print.calibration", json!({"kind": kind}));
        for n in created(&r) {
            assert!(volume(&s, &n) > 1.0, "{kind} {n}");
        }
        assert_eq!(s.model.state().bodies.len(), created(&r).len(), "{kind}: loose pieces");
    }
}

#[test]
fn a_hinge_is_two_parts_that_never_touch() {
    let mut s = Session::default();
    let r = run(&mut s, "print.hinge", json!({}));
    assert_eq!(created(&r).len(), 2);
    let r = run(&mut s, "inspect.interference", json!({"bodies": created(&r)}));
    let hits = r.get("interferences").or_else(|| r.get("pairs")).and_then(Value::as_array).map_or(0, Vec::len);
    assert_eq!(hits, 0, "{r}");
}

#[test]
fn lengths_take_expressions_and_survive_an_inch_design() {
    let mut s = block();
    run(&mut s, "parameters.change", json!({"name": "mag", "expression": "8 mm"}));
    run(&mut s, "print.magnet_pocket", json!({"point": [20, 15, 20], "diameter": "mag", "thickness": 2, "clearance": 0}));
    near(volume(&s, "Block"), 24000.0 - std::f64::consts::PI * 16.0 * 2.1, 1.5);
    let mut s = block();
    run(&mut s, "document.units", json!({"units": "in"}));
    run(&mut s, "print.magnet_pocket", json!({"point": [20, 15, 20], "diameter": 8, "thickness": 2, "clearance": 0}));
    near(volume(&s, "Block"), 24000.0 - std::f64::consts::PI * 16.0 * 2.1, 1.5);
}

#[test]
fn a_gear_outline_is_simple_and_the_right_size() {
    for z in [8, 20, 60] {
        let o = make::gear_outline(z, 1.5, 20.0, 0.2);
        let area: f64 = o.iter().zip(o.iter().cycle().skip(1)).map(|(a, b)| a[0] * b[1] - b[0] * a[1]).sum::<f64>() * 0.5;
        let pitch = 1.5 * z as f64 * 0.5;
        assert!(area > 0.0 && area < std::f64::consts::PI * (pitch + 1.5).powi(2), "{z}: {area}");
        near(o.iter().map(|p| p[0].hypot(p[1])).fold(0.0, f64::max), pitch + 1.5, 1e-9);
    }
}
