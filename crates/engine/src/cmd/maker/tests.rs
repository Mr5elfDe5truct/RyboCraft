use super::*;

fn run(s: &mut Session, id: &str, p: Value) -> Value {
    s.execute(id, &p).unwrap_or_else(|e| panic!("{id}: {e}"))
}

#[test]
fn every_blank_tool_makes_watertight_bodies_in_one_undo_step() {
    for t in TOOLS.iter().filter(|t| t.panel != "EDIT MODEL" && !t.fields.iter().any(|f| f.key == "image")) {
        let mut s = Session::default();
        let r = run(&mut s, t.id, json!({}));
        let bodies = created(&r);
        assert!(!bodies.is_empty(), "{}: {r}", t.id);
        for b in &bodies {
            let m = mesh_of(&s, std::slice::from_ref(b)).unwrap();
            let k = rybocraft_print3d::analysis::topology(&m);
            assert!(k.watertight() && k.volume > 0.5, "{} {b}: {k:?}", t.id);
        }
        assert_eq!(s.undo.len(), 1, "{}", t.id);
    }
}

#[test]
fn models_are_edited_as_solids_and_as_meshes() {
    for mesh in [false, true] {
        for t in TOOLS.iter().filter(|t| t.panel == "EDIT MODEL") {
            let mut s = Session::default();
            run(&mut s, "solid.box", json!({"length": 60, "width": 24, "height": 24, "corner": [0, 0, 0], "body_name": "Part"}));
            run(&mut s, "solid.box", json!({"length": 10, "width": 10, "height": 40, "corner": [20, 5, -5], "operation": "new", "body_name": "Other"}));
            if mesh {
                run(&mut s, "make.fuse", json!({"bodies": ["Part", "Other"], "detail": 1}));
                run(&mut s, "solid.box", json!({"length": 10, "width": 10, "height": 60, "corner": [40, 5, -5], "operation": "new", "body_name": "Other"}));
            }
            let part = if mesh { "Fused" } else { "Part" };
            let mut p = json!({"detail": 0.8});
            let keys: Vec<&str> = t.fields.iter().map(|f| f.key).collect();
            if !keys.contains(&"detail") {
                p = json!({});
            }
            if keys.contains(&"bodies") {
                p["bodies"] = json!([part, "Other"]);
            } else {
                p["body"] = json!(part);
            }
            if keys.contains(&"point") {
                p["point"] = json!([10, 12, 24]);
            }
            run(&mut s, t.id, p);
        }
    }
}
