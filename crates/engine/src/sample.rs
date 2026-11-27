//! A sample design (used by `rybocraft --sample` and demos): a parametric plate with rounded
//! corners, two holes and a boss.

use serde_json::{Value, json};

pub fn script() -> Value {
    json!({"commands": [
        {"command": "parameters.change", "params": {"name": "width", "expression": "80 mm", "comment": "plate width"}},
        {"command": "parameters.change", "params": {"name": "depth", "expression": "50 mm"}},
        {"command": "parameters.change", "params": {"name": "thickness", "expression": "8 mm"}},
        {"command": "sketch.create", "params": {"plane": "XY", "name": "Base"}},
        {"command": "sketch.rectangle.two_point", "params": {"p0": [0, 0], "p1": [80, 50]}},
        {"command": "sketch.constraint.coincident", "params": {"a": "p1", "b": "origin"}},
        {"command": "sketch.dimension", "params": {"entities": ["l1"], "value": "width"}},
        {"command": "sketch.dimension", "params": {"entities": ["l2"], "value": "depth"}},
        {"command": "sketch.finish", "params": {}},
        {"command": "solid.extrude", "params": {"distance": "thickness", "body_name": "Plate"}},
        {"command": "solid.fillet", "params": {"edges": [[0, 0, 4], [80, 0, 4], [80, 50, 4], [0, 50, 4]], "radius": "6 mm"}},
        {"command": "sketch.create", "params": {"plane": "XY", "name": "Holes"}},
        {"command": "sketch.circle.center", "params": {"center": [12, 25], "diameter": 8}},
        {"command": "sketch.circle.center", "params": {"center": [68, 25], "diameter": 8}},
        {"command": "sketch.finish", "params": {}},
        {"command": "solid.extrude", "params": {"distance": "thickness", "operation": "cut", "name": "Holes"}},
        {"command": "sketch.create", "params": {"plane": "XY", "offset": "thickness", "name": "Boss"}},
        {"command": "sketch.circle.center", "params": {"center": [40, 25], "diameter": 24}},
        {"command": "sketch.finish", "params": {}},
        {"command": "solid.extrude", "params": {"distance": 14, "operation": "join", "name": "Boss"}},
        {"command": "sketch.create", "params": {"plane": "XY", "offset": "thickness + 14 mm", "name": "Bore"}},
        {"command": "sketch.circle.center", "params": {"center": [40, 25], "diameter": 12}},
        {"command": "sketch.finish", "params": {}},
        {"command": "solid.extrude", "params": {"distance": "thickness + 14 mm", "direction": "negative", "operation": "cut", "name": "Bore"}},
    ]})
}

/// A built-in sample design: its name, a line about what it shows, the group the start page lists
/// it under, and the script that builds it.
pub struct Sample {
    pub name: &'static str,
    pub about: &'static str,
    pub group: &'static str,
    pub script: fn() -> Value,
}

/// The start page's sample groups, in the order it shows them.
pub const GROUPS: [&str; 6] = ["Engineering", "Basic Shapes", "Complex Shapes", "Electronics", "Gaming", "Workshop & Home"];

/// A sample whose script is a file in `assets/samples/scripts/`: the commands it runs, as
/// `rybocraft-cli run` takes them. A script that doesn't parse builds nothing (the tests check
/// every one builds).
macro_rules! file_sample {
    ($name:ident, $file:literal) => {
        fn $name() -> Value {
            serde_json::from_str(include_str!(concat!("../../../assets/samples/scripts/", $file, ".json")))
                .unwrap_or_else(|_| json!({"commands": []}))
        }
    };
}

file_sample!(basic_cube, "basic-cube");
file_sample!(basic_tube, "basic-tube");
file_sample!(basic_sphere, "basic-sphere");
file_sample!(basic_cone, "basic-cone");
file_sample!(basic_torus, "basic-torus");
file_sample!(basic_pyramid, "basic-pyramid");
file_sample!(basic_hex_prism, "basic-hex-prism");
file_sample!(complex_twisted_vase, "complex-twisted-vase");
file_sample!(complex_coil_spring, "complex-coil-spring");
file_sample!(complex_bottle, "complex-bottle");
file_sample!(complex_knurled_knob, "complex-knurled-knob");
file_sample!(complex_swept_handle, "complex-swept-handle");
file_sample!(complex_wing, "complex-wing");
file_sample!(pi3b_plus_case, "electronics-pi3b-plus-case");
file_sample!(pi4_case, "electronics-pi4-case");
file_sample!(pi5_case, "electronics-pi5-case");
file_sample!(arduino_uno_base, "electronics-arduino-uno-base");
file_sample!(xbox_faceplate, "gaming-xbox-series-faceplate");
file_sample!(dualsense_faceplate, "gaming-ps5-dualsense-faceplate");
file_sample!(phone_stand, "maker-phone-stand");
file_sample!(cable_clip, "maker-cable-clip");
file_sample!(hex_bit_holder, "maker-hex-bit-holder");
file_sample!(sd_card_holder, "maker-sd-card-holder");
file_sample!(calibration_cube, "maker-calibration-cube");
file_sample!(wall_hook, "maker-wall-hook");
file_sample!(grid_bin, "maker-grid-bin");

/// The built-in samples (the start page lists them by group, in this order), each built through
/// commands. Their pictures are `assets/samples/exports/<index>.png`, so add new ones at the end.
pub fn samples() -> Vec<Sample> {
    vec![
        Sample { name: "Sample Plate", about: "Parametric plate with fillets, holes and a bored boss", group: GROUPS[0], script: script },
        Sample { name: "Sheet Metal Bracket", about: "Base flange with edge flanges and a hem on a 2 mm rule", group: GROUPS[0], script: bracket },
        Sample { name: "Hinge Assembly", about: "Two components and a revolute joint, driven to 60°", group: GROUPS[0], script: hinge },
        Sample { name: "Plastic Enclosure", about: "Shelled box with a lip, screw bosses, a rest and a snap fit", group: GROUPS[0], script: enclosure },
        Sample { name: "Bolted Flange", about: "Hub and flange with a circular pattern driven by a parameter", group: GROUPS[0], script: flange },
        Sample { name: "Vent Plate", about: "Slots in a rectangular pattern: rows, columns and pitch are parameters", group: GROUPS[0], script: vents },
        Sample { name: "Rounded Cube", about: "A box with every edge filleted: size and rounding are parameters", group: GROUPS[1], script: basic_cube },
        Sample { name: "Tube", about: "Two circles extruded into a tube: diameter, wall and height are parameters", group: GROUPS[1], script: basic_tube },
        Sample { name: "Sphere", about: "A sphere whose diameter is a parameter", group: GROUPS[1], script: basic_sphere },
        Sample { name: "Cone", about: "A cone, or a frustum when the top diameter is above zero", group: GROUPS[1], script: basic_cone },
        Sample { name: "Torus", about: "A ring: its radius and the tube's are parameters", group: GROUPS[1], script: basic_torus },
        Sample { name: "Square Pyramid", about: "A loft from a square to a single apex point", group: GROUPS[1], script: basic_pyramid },
        Sample { name: "Hexagonal Prism", about: "A six-sided polygon extruded to a height parameter", group: GROUPS[1], script: basic_hex_prism },
        Sample { name: "Twisted Vase", about: "A loft through three turned octagons, shelled open at the top", group: GROUPS[2], script: complex_twisted_vase },
        Sample { name: "Coil Spring", about: "A helical coil: turns and pitch are parameters", group: GROUPS[2], script: complex_coil_spring },
        Sample { name: "Threaded Bottle", about: "A revolved spline profile, shelled, with an M27 thread on the neck", group: GROUPS[2], script: complex_bottle },
        Sample { name: "Knurled Knob", about: "24 grooves in a circular pattern, with a nut pocket and a bolt hole", group: GROUPS[2], script: complex_knurled_knob },
        Sample { name: "Swept Handle", about: "An elliptical grip swept along a spline, with screw feet", group: GROUPS[2], script: complex_swept_handle },
        Sample { name: "Wing Section", about: "A tapered loft between two NACA 2412 airfoils", group: GROUPS[2], script: complex_wing },
        Sample { name: "Raspberry Pi 3 Model B+ Case", about: "Case and vented lid with standoffs and cut-outs for the Pi 3 B+ ports", group: GROUPS[3], script: pi3b_plus_case },
        Sample { name: "Raspberry Pi 4 Model B Case", about: "Case and vented lid with standoffs and cut-outs for the Pi 4 ports", group: GROUPS[3], script: pi4_case },
        Sample { name: "Raspberry Pi 5 Case", about: "Case and extra-vented lid with standoffs and cut-outs for the Pi 5 ports", group: GROUPS[3], script: pi5_case },
        Sample { name: "Arduino Uno Base", about: "A base plate with standoffs on the Uno's mounting holes", group: GROUPS[3], script: arduino_uno_base },
        Sample { name: "Xbox Series X|S Controller Faceplate", about: "Starting shape for a replacement faceplate: check it against your controller before printing", group: GROUPS[4], script: xbox_faceplate },
        Sample { name: "PS5 DualSense Controller Faceplate", about: "Starting shape for a replacement faceplate: check it against your controller before printing", group: GROUPS[4], script: dualsense_faceplate },
        Sample { name: "Phone Stand", about: "A leaning stand with a lip and a cable notch; width is a parameter", group: GROUPS[5], script: phone_stand },
        Sample { name: "Cable Clip", about: "A screw-on clip sized from the cable diameter parameter", group: GROUPS[5], script: cable_clip },
        Sample { name: "Hex Bit Holder", about: "Pockets for 1/4 inch bits in a pattern of rows and columns", group: GROUPS[5], script: hex_bit_holder },
        Sample { name: "SD Card Holder", about: "Slots for five SD cards and sixteen microSD cards", group: GROUPS[5], script: sd_card_holder },
        Sample { name: "Calibration Cube", about: "A 20 mm cube with X, Y and Z engraved on its faces", group: GROUPS[5], script: calibration_cube },
        Sample { name: "Wall Hook", about: "A screw-on hook that prints on its side, made by Wall Hook", group: GROUPS[5], script: wall_hook },
        Sample { name: "Grid Bin", about: "A storage bin for 42 mm grid base plates, made by Grid Bin", group: GROUPS[5], script: grid_bin },
    ]
}

fn bracket() -> Value {
    json!({"commands": [
        {"command": "sheet.manage_rules", "params": {"name": "Steel 2mm", "thickness": "2 mm", "bend_radius": "Thickness", "active": true}},
        {"command": "sketch.create", "params": {"plane": "XY", "name": "Base"}},
        {"command": "sketch.rectangle.two_point", "params": {"p0": [0, 0], "p1": [120, 60]}},
        {"command": "sketch.finish", "params": {}},
        {"command": "sheet.flange", "params": {"sketch": "Base"}},
        {"command": "sheet.flange", "params": {"edges": [[60, 0, 2], [60, 60, 2]], "height": 30}},
        {"command": "sheet.flange", "params": {"edges": [[120, 30, 2]], "height": 20, "angle": "60 deg"}},
        {"command": "sheet.hem", "params": {"edges": [[0, 30, 2]], "length": 8}},
    ]})
}

fn hinge() -> Value {
    let mut cmds = Vec::new();
    for name in ["Leaf A", "Leaf B"] {
        cmds.extend([
            json!({"command": "component.activate", "params": {"component": "root"}}),
            json!({"command": "component.create", "params": {"name": name}}),
            json!({"command": "solid.box", "params": {"length": 40, "width": 30, "height": 3, "corner": [0, 0, 0]}}),
            json!({"command": "solid.cylinder", "params": {"base": [0, 33, 4], "axis": [1, 0, 0], "radius": 4, "height": 40, "operation": "new"}}),
        ]);
    }
    cmds.extend([
        json!({"command": "component.activate", "params": {"component": "root"}}),
        json!({"command": "occurrence.ground", "params": {"occurrence": "Leaf A:1", "grounded": true}}),
        json!({"command": "joint.create", "params": {"type": "revolute", "name": "Hinge",
            "a": {"occurrence": "Leaf A:1", "circle": [1, 37, 4]}, "b": {"occurrence": "Leaf B:1", "circle": [1, 37, 4]},
            "limits": [["0 deg", "180 deg"]]}}),
        json!({"command": "joint.drive", "params": {"joint": "Hinge", "value": 60}}),
    ]);
    json!({ "commands": cmds })
}

fn enclosure() -> Value {
    json!({"commands": [
        {"command": "solid.box", "params": {"length": 80, "width": 50, "height": 30, "body_name": "Enclosure"}},
        {"command": "solid.shell", "params": {"faces": [[40, 25, 30]], "thickness": 2}},
        {"command": "plastic.lip", "params": {"face": [1, 25, 30], "width": "1 mm", "height": "2 mm"}},
        {"command": "plastic.snap_fit", "params": {"position": [40, 44, 2], "hook": [0, -1, 0], "length": "14 mm", "thickness": "1.5 mm", "width": "6 mm", "catch_depth": "1 mm", "catch_length": "2 mm"}},
        {"command": "plastic.boss", "params": {"position": [10, 10, 2], "diameter": "7 mm", "height": "12 mm", "hole_diameter": "2.5 mm"}},
        {"command": "plastic.boss", "params": {"position": [70, 40, 2], "diameter": "7 mm", "height": "12 mm", "hole_diameter": "2.5 mm"}},
        {"command": "plastic.rest", "params": {"position": [40, 25, 2], "width": "16 mm", "length": "24 mm", "height": "3 mm"}},
    ]})
}

fn flange() -> Value {
    json!({"commands": [
        {"command": "parameters.change", "params": {"name": "bolts", "expression": "6", "unit": "", "comment": "number of bolt holes"}},
        {"command": "parameters.change", "params": {"name": "pcd", "expression": "70 mm", "comment": "bolt circle diameter"}},
        {"command": "sketch.create", "params": {"plane": "XY", "name": "Disc"}},
        {"command": "sketch.circle.center", "params": {"center": [0, 0], "diameter": 100}},
        {"command": "sketch.finish", "params": {}},
        {"command": "solid.extrude", "params": {"distance": 10, "body_name": "Flange", "name": "Disc"}},
        {"command": "sketch.create", "params": {"plane": {"face": [0, 0, 10]}, "name": "Hub"}},
        {"command": "sketch.circle.center", "params": {"center": [0, 0], "diameter": 44}},
        {"command": "sketch.finish", "params": {}},
        {"command": "solid.extrude", "params": {"distance": 16, "operation": "join", "name": "Hub"}},
        {"command": "sketch.create", "params": {"plane": "XY", "name": "Bore"}},
        {"command": "sketch.circle.center", "params": {"center": [0, 0], "diameter": 24}},
        {"command": "sketch.finish", "params": {}},
        {"command": "solid.extrude", "params": {"distance": 26, "operation": "cut", "name": "Bore"}},
        {"command": "sketch.create", "params": {"plane": "XY", "name": "Bolt"}},
        {"command": "sketch.circle.center", "params": {"center": [35, 0], "diameter": 9}},
        {"command": "sketch.finish", "params": {}},
        {"command": "solid.extrude", "params": {"distance": 10, "operation": "cut", "name": "BoltHole"}},
        {"command": "solid.pattern.circular", "params": {"features": ["BoltHole"], "axis": "Z", "count": "bolts"}},
    ]})
}

fn vents() -> Value {
    json!({"commands": [
        {"command": "parameters.change", "params": {"name": "cols", "expression": "6", "unit": ""}},
        {"command": "parameters.change", "params": {"name": "rows", "expression": "3", "unit": ""}},
        {"command": "parameters.change", "params": {"name": "pitch", "expression": "16 mm", "comment": "slot spacing"}},
        {"command": "solid.box", "params": {"length": 120, "width": 80, "height": 4, "body_name": "Plate"}},
        {"command": "solid.fillet", "params": {"edges": [[0, 0, 2], [120, 0, 2], [120, 80, 2], [0, 80, 2]], "radius": "8 mm"}},
        {"command": "sketch.create", "params": {"plane": "XY", "name": "Slot"}},
        {"command": "sketch.rectangle.two_point", "params": {"p0": [18, 12], "p1": [24, 26]}},
        {"command": "sketch.finish", "params": {}},
        {"command": "solid.extrude", "params": {"distance": 4, "operation": "cut", "name": "Vent"}},
        {"command": "solid.pattern.rectangular", "params": {"features": ["Vent"], "dir1": [1, 0, 0], "count1": "cols", "spacing1": "pitch",
            "dir2": [0, 1, 0], "count2": "rows", "spacing2": "22 mm"}},
    ]})
}

#[cfg(test)]
mod tests {
    use rybocraft_geom::MeshGeom;

    #[test]
    fn plate_boss_and_bore_planes_follow_thickness_without_weak_face_references() {
        let mut session = crate::Session::default();
        session.run_script(&super::script()).unwrap();
        let boss = session
            .doc
            .features
            .iter()
            .find(|feature| feature.name == "Boss" && matches!(feature.kind, rybocraft_doc::FeatureKind::Sketch { .. }))
            .unwrap()
            .id;
        let bore = session
            .doc
            .features
            .iter()
            .find(|feature| feature.name == "Bore" && matches!(feature.kind, rybocraft_doc::FeatureKind::Sketch { .. }))
            .unwrap()
            .id;
        session.execute("parameters.change", &serde_json::json!({"name":"thickness","expression":"12 mm"})).unwrap();
        assert!(session.model.results.iter().all(|result| result.error.is_none()));
        assert!((session.model.state().sketch(boss).unwrap().plane.origin.z - 12.).abs() < 1e-6);
        assert!((session.model.state().sketch(bore).unwrap().plane.origin.z - 26.).abs() < 1e-6);
        let world = session.world_state().unwrap();
        let plate = world.body("Plate").unwrap().mesh();
        assert!((plate.bounds().max.z - 26.).abs() < 1e-6);
        assert!(plate.contains(rybocore_record::Vec3::new(5., 25., 10.)));
        assert!(plate.contains(rybocore_record::Vec3::new(40., 35., 24.)));
        assert!(!plate.contains(rybocore_record::Vec3::new(40., 25., 24.)));
        assert!(!plate.contains(rybocore_record::Vec3::new(40., 25., 2.)));
        assert!(plate.contains(rybocore_record::Vec3::new(40., 35., 2.)));
        let reopened = crate::Session::new(rybocraft_doc::Document::from_json(&session.doc.to_json()).unwrap());
        assert!((reopened.model.state().sketch(bore).unwrap().plane.origin.z - 26.).abs() < 1e-6);
        let reopened_world = reopened.world_state().unwrap();
        let reopened_plate = reopened_world.body("Plate").unwrap().mesh();
        assert!(!reopened_plate.contains(rybocore_record::Vec3::new(40., 25., 2.)));
        assert!(reopened_plate.contains(rybocore_record::Vec3::new(40., 35., 2.)));
    }

    /// Every sample has commands (a script file that didn't parse would build nothing), a group
    /// the start page knows, and a name of its own.
    #[test]
    fn samples_have_commands_groups_and_unique_names() {
        let samples = super::samples();
        let mut names = std::collections::BTreeSet::new();
        for s in &samples {
            let script = (s.script)();
            assert!(script["commands"].as_array().is_some_and(|c| !c.is_empty()), "{} has no commands", s.name);
            assert!(super::GROUPS.contains(&s.group), "{}: unknown group {}", s.name, s.group);
            assert!(names.insert(s.name), "{} twice", s.name);
        }
        // The start page shows the groups in order, each once: keep a group's samples together.
        let order: Vec<_> = samples.iter().map(|s| s.group).collect::<Vec<_>>().chunk_by(|a, b| a == b).map(|run| run[0]).collect();
        assert_eq!(order, super::GROUPS);
    }

    /// Every built-in sample builds without a failing feature or joint, and without a panic on
    /// the way: panics inside the kernel are caught natively, but abort the web build (wasm
    /// can't unwind; the Plastic Enclosure's bosses used to panic 8 times).
    #[test]
    fn samples_build_cleanly_without_panics() {
        use std::sync::Arc;
        use std::sync::atomic::{AtomicUsize, Ordering};
        let me = std::thread::current().id();
        let count = Arc::new(AtomicUsize::new(0));
        let seen = count.clone();
        let prev = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |_| {
            if std::thread::current().id() == me {
                seen.fetch_add(1, Ordering::SeqCst);
            }
        }));
        let mut failed = Vec::new();
        for s in super::samples() {
            let mut session = crate::Session::default();
            let built = session.run_script(&(s.script)());
            let panics = count.swap(0, Ordering::SeqCst);
            if panics > 0 {
                failed.push(format!("{}: {panics} panic(s)", s.name));
            }
            if let Err(e) = built {
                failed.push(format!("{}: {e}", s.name));
                continue;
            }
            failed.extend(session.model.results.iter().filter_map(|r| r.error.as_ref().map(|e| format!("{}: {}: {e}", s.name, r.name))));
            if session.model.state().bodies.is_empty() {
                failed.push(format!("{}: no bodies", s.name));
            }
        }
        std::panic::set_hook(prev);
        assert!(failed.is_empty(), "{failed:?}");
    }
}
