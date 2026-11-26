//! The 3D printing commands (`print.*`). Each is described once, by its [`Tool`]: the table gives
//! the parameter schema agents see, the defaults and limits the handler reads, and the form the
//! app shows. The geometry itself is `rybocraft-print3d`.

mod design;
mod fit;
mod make;
mod prepare;

use std::collections::BTreeSet;
use std::sync::Arc;

use rybocraft_print3d::{PrintMesh, tables};
use serde_json::{Map, Value, json};
use rybocore_record::Vec3;
use rybocraft_doc::expr::Kind;
use rybocraft_geom::prelude::*;

use crate::params::bad;
use crate::schema::{Contract, Issue};
use crate::{EngineError, Result, Sel, Session};

/// What a tool's parameter is, with its default and limits.
#[derive(Clone, Copy, Debug)]
pub enum FieldKind {
    /// Millimetres, or a length expression over the design's parameters.
    Length { default: f64, min: f64, max: f64 },
    /// A plain number with the unit shown after it.
    Number { default: f64, min: f64, max: f64, unit: &'static str },
    Int { default: i64, min: i64, max: i64 },
    Bool(bool),
    Choice { default: &'static str, options: &'static [&'static str] },
    /// Body names; none means the selected bodies, else every visible one.
    Bodies,
    /// One body; none means the selected one, else the only visible one.
    Body,
    /// A point on a face; none means the selected face.
    Point,
    /// A position in the model.
    Position([f64; 3]),
    Text(&'static str),
    /// An installed font.
    Font,
    /// A file of one of these types.
    File(&'static [&'static str]),
}

#[derive(Clone, Copy, Debug)]
pub struct Field {
    pub key: &'static str,
    pub label: &'static str,
    pub help: &'static str,
    pub kind: FieldKind,
}

pub struct Tool {
    pub id: &'static str,
    pub label: &'static str,
    /// The ribbon panel on the 3D PRINT tab.
    pub panel: &'static str,
    pub icon: &'static str,
    pub help: &'static str,
    pub fields: &'static [Field],
    /// Changes nothing in the design.
    pub reads_only: bool,
}

pub(in crate::cmd) const fn field(key: &'static str, label: &'static str, kind: FieldKind, help: &'static str) -> Field {
    Field { key, label, help, kind }
}
pub(in crate::cmd) const fn len(default: f64, min: f64, max: f64) -> FieldKind {
    FieldKind::Length { default, min, max }
}
pub(in crate::cmd) const fn num(default: f64, min: f64, max: f64, unit: &'static str) -> FieldKind {
    FieldKind::Number { default, min, max, unit }
}
pub(in crate::cmd) const fn int(default: i64, min: i64, max: i64) -> FieldKind {
    FieldKind::Int { default, min, max }
}
pub(in crate::cmd) const fn choice(default: &'static str, options: &'static [&'static str]) -> FieldKind {
    FieldKind::Choice { default, options }
}

pub(in crate::cmd) const BODIES: Field = field("bodies", "Bodies", FieldKind::Bodies, "Bodies to use. Default: the selected bodies, else all visible ones");
pub(in crate::cmd) const BODY: Field = field("body", "Body", FieldKind::Body, "The body. Default: the selected one, else the only visible one");
pub(in crate::cmd) const POINT: Field =
    field("point", "Face point", FieldKind::Point, "A point on a flat face of the body, in mm. Default: where the selected face was picked");
pub(in crate::cmd) const AT: Field = field("at", "Position", FieldKind::Position([0.0; 3]), "Where the new part's centre goes on the bed, in mm");

/// The panels of the 3D PRINT tab.
pub const PANELS: [&str; 6] = ["PRINTER", "ANALYZE", "PREPARE", "FEATURES", "MAKE", "CALIBRATE"];

macro_rules! tools {
    ($tab:literal; $($id:literal, $label:literal, $panel:literal, $icon:literal, $reads:literal, $run:path, $help:literal, [$($field:expr),* $(,)?];)*) => {
        pub static TOOLS: &[$crate::cmd::print::Tool] = &[$($crate::cmd::print::Tool {
            id: $id, label: $label, panel: $panel, icon: $icon, help: $help, fields: &[$($field),*], reads_only: $reads,
        }),*];
        pub static COMMANDS: &[$crate::cmd::CommandSpec] = &[$({
            fn handler(s: &mut $crate::Session, p: &serde_json::Value) -> $crate::Result<serde_json::Value> {
                let args = $crate::cmd::print::Args::read(s, $id, p)?;
                $run(s, &args)
            }
            let spec = $crate::cmd::CommandSpec::new($id, $label, |s, p| crate::schema::run_checked_as(s, p, $id, handler))
                .at($tab, $panel)
                .icon($icon)
                .contract(&$crate::cmd::print::ToolContract($id))
                .output($crate::cmd::print::print_output)
                .enabled($crate::cmd::not_in_sketch)
                .params($help);
            if $reads { spec.noundo() } else { spec }
        }),*];
    };
}

const SIZES: &[&str] = &["M2", "M2.5", "M3", "M4", "M5", "M6", "M8"];

tools! { "3D PRINT";
    "print.setup", "Print Setup", "PRINTER", "p3_printer", true, setup_cmd,
    "Choose the printer, filament and slicing defaults the other print tools use. They are remembered on this computer. Returns the setup and the printers and filaments known",
    [
        field("printer", "Printer", FieldKind::Text(""), "A printer by name (see the returned list), or `custom` with bed_x, bed_y and bed_z"),
        field("bed_x", "Bed X", len(0.0, 0.0, 5000.0), "Build volume along X; 0 keeps the printer's"),
        field("bed_y", "Bed Y", len(0.0, 0.0, 5000.0), "Build volume along Y; 0 keeps the printer's"),
        field("bed_z", "Bed Z", len(0.0, 0.0, 5000.0), "Build height; 0 keeps the printer's"),
        field("material", "Filament", FieldKind::Text(""), "A filament by name (see the returned list)"),
        field("layer", "Layer height", len(0.0, 0.0, 2.0), "0 keeps the current value"),
        field("nozzle", "Nozzle", len(0.0, 0.0, 3.0), "Nozzle diameter; 0 keeps the current value"),
        field("walls", "Walls", int(0, 0, 50), "Wall loops; 0 keeps the current value"),
        field("infill", "Infill", num(-1.0, -1.0, 100.0, "%"), "-1 keeps the current value"),
        field("price", "Filament price", num(-1.0, -1.0, 100000.0, "/kg"), "Price of a kilogram, in your currency; -1 keeps the current value"),
    ];
    "print.clear_overlay", "Clear Highlights", "PRINTER", "delete", true, clear_overlay,
    "Remove the highlights the print analyses leave in the view", [];

    "print.check", "Printability Check", "ANALYZE", "p3_check", true, prepare::check,
    "Check bodies for everything that commonly spoils a print: open or non-manifold meshes, overhangs, thin walls, islands that start in mid-air, bed size, bed contact and stability. Returns a score out of 100 and an issue list, and highlights the problems",
    [BODIES, field("min_wall", "Thinnest wall", len(0.8, 0.05, 20.0), "Walls thinner than this are reported"),
     field("angle", "Overhang angle", num(0.0, 0.0, 89.0, "°"), "Faces flatter than this (from horizontal) need support; 0 uses the filament's")];
    "print.overhangs", "Overhangs", "ANALYZE", "p3_overhang", true, prepare::overhangs,
    "Highlight the faces that need support and report their area and the support they want",
    [BODIES, field("angle", "Overhang angle", num(0.0, 0.0, 89.0, "°"), "Faces flatter than this (from horizontal); 0 uses the filament's")];
    "print.thin_walls", "Wall Thickness", "ANALYZE", "p3_thickness", true, prepare::thin_walls,
    "Measure wall thickness all over and highlight walls thinner than the limit",
    [BODIES, field("min_wall", "Thinnest wall", len(0.8, 0.05, 50.0), "Walls thinner than this are highlighted")];
    "print.islands", "Islands", "ANALYZE", "p3_islands", true, prepare::islands,
    "Find regions that would start printing in mid-air, layer by layer",
    [BODIES, field("layer", "Layer height", len(0.0, 0.0, 2.0), "0 uses the print setup's"),
     field("min_area", "Smallest island", num(0.5, 0.0, 10000.0, "mm²"), "Smaller ones are ignored")];
    "print.layer", "Layer Preview", "ANALYZE", "p3_layers", true, prepare::layer,
    "Show one layer's outline in the view, with its area and perimeter",
    [BODIES, field("layer", "Layer number", int(1, 1, 100000), "Counting from 1 at the bed"),
     field("height", "Layer height", len(0.0, 0.0, 2.0), "0 uses the print setup's")];
    "print.estimate", "Print Estimate", "ANALYZE", "p3_estimate", true, prepare::estimate,
    "Estimate filament, weight, cost and time from the real layers of the bodies",
    [BODIES, field("layer", "Layer height", len(0.0, 0.0, 2.0), "0 uses the print setup's"),
     field("infill", "Infill", num(-1.0, -1.0, 100.0, "%"), "-1 uses the print setup's"),
     field("walls", "Walls", int(0, 0, 50), "0 uses the print setup's"),
     field("supports", "Include supports", FieldKind::Bool(true), "Add a rough amount for supports"),
     field("copies", "Copies", int(1, 1, 1000), "How many to print")];
    "print.compare_materials", "Compare Filaments", "ANALYZE", "library", true, prepare::compare_materials,
    "The same print in every known filament: weight, length and cost side by side", [BODIES];

    "print.auto_orient", "Auto Orient", "PREPARE", "p3_orient", false, prepare::auto_orient,
    "Turn each body the way up that prints best (least support, good bed contact, low height) and set it on the bed",
    [BODIES, field("priority", "Priority", choice("balanced", &["balanced", "least_support", "fastest", "best_adhesion"]), "What matters most"),
     field("angle", "Overhang angle", num(0.0, 0.0, 89.0, "°"), "0 uses the filament's")];
    "print.lay_flat", "Lay Face Down", "PREPARE", "print_face", false, prepare::lay_flat,
    "Turn a body so the picked flat face lies on the bed", [BODY, POINT];
    "print.drop_to_bed", "Drop to Bed", "PREPARE", "p3_drop", false, prepare::drop_to_bed,
    "Move bodies straight down (or up) so they rest on Z = 0",
    [BODIES, field("center", "Center on bed", FieldKind::Bool(false), "Also centre them, together, on the origin")];
    "print.arrange", "Arrange on Bed", "PREPARE", "p3_arrange", false, prepare::arrange,
    "Lay bodies out on the printer's bed without overlaps, starting more plates when one is full",
    [BODIES, field("gap", "Gap", len(5.0, 0.0, 200.0), "Between bodies"), field("margin", "Bed margin", len(5.0, 0.0, 200.0), "Kept free at the bed's edge")];
    "print.fit_to_bed", "Scale to Bed", "PREPARE", "scale", false, prepare::fit_to_bed,
    "Scale bodies evenly so they fit the build volume",
    [BODIES, field("margin", "Margin", len(5.0, 0.0, 200.0), "Kept free on every side"),
     field("enlarge", "Enlarge too", FieldKind::Bool(false), "Also scale small parts up to fill the volume")];
    "print.shrinkage", "Shrinkage Compensation", "PREPARE", "p3_shrink", false, prepare::shrinkage,
    "Scale bodies up by what the filament shrinks",
    [BODIES, field("percent", "Shrinkage XY", num(-1.0, -1.0, 10.0, "%"), "-1 uses the filament's"),
     field("percent_z", "Shrinkage Z", num(-1.0, -1.0, 10.0, "%"), "-1 uses the XY value")];
    "print.supports", "Custom Supports", "PREPARE", "p3_supports", false, prepare::supports,
    "Make breakaway support posts under every overhang, as a mesh body of their own",
    [BODIES, field("angle", "Overhang angle", num(0.0, 0.0, 89.0, "°"), "0 uses the filament's"),
     field("spacing", "Spacing", len(4.0, 1.0, 50.0), "Between posts"), field("diameter", "Post diameter", len(1.6, 0.4, 20.0), "Of each post"),
     field("tip", "Tip diameter", len(0.6, 0.1, 10.0), "Where it touches the part"), field("gap", "Gap", len(0.2, 0.0, 2.0), "Left between post and part"),
     field("foot", "Foot", len(1.0, 0.0, 10.0), "Extra radius on the bed"), field("bed_only", "From bed only", FieldKind::Bool(false), "No posts standing on the part")];
    "print.mouse_ears", "Brim Ears", "PREPARE", "p3_ears", false, prepare::mouse_ears,
    "Add thin discs at a part's outer corners on the bed, to hold them down against warping",
    [BODY, field("diameter", "Diameter", len(12.0, 2.0, 100.0), "Of each ear"), field("thickness", "Thickness", len(0.2, 0.05, 5.0), "One layer is usual"),
     field("separate", "Separate body", FieldKind::Bool(false), "Keep the ears as their own body instead of joining them")];
    "print.split_to_fit", "Split to Fit Bed", "PREPARE", "split_body", false, prepare::split_to_fit,
    "Cut a body too big for the printer into pieces that fit, with holes for alignment pins and the pins to go in them",
    [BODY, field("margin", "Margin", len(5.0, 0.0, 200.0), "Kept free on every side"), field("pins", "Pins per cut", int(2, 0, 8), "0: plain cuts"),
     field("pin_diameter", "Pin diameter", len(5.0, 1.0, 50.0), "Of the pins"), field("pin_depth", "Pin depth", len(8.0, 1.0, 100.0), "Into each side"),
     field("clearance", "Clearance", len(0.2, 0.0, 2.0), "Extra hole diameter")];

    "print.insert_hole", "Heat-Set Insert", "FEATURES", "p3_insert", false, design::insert_hole,
    "Cut a hole sized for a brass heat-set insert, with a lead-in, optionally on a new boss",
    [BODY, POINT, field("size", "Thread", choice("M3", SIZES), "Insert thread"), field("diameter", "Hole diameter", len(0.0, 0.0, 50.0), "0 uses the common size"),
     field("depth", "Hole depth", len(0.0, 0.0, 100.0), "0 uses the common depth"), field("boss", "Boss height", len(0.0, 0.0, 200.0), "0: straight into the face")];
    "print.nut_trap", "Nut Trap", "FEATURES", "p3_nut", false, design::nut_trap,
    "Cut a hexagonal pocket for a captive nut and the bolt hole behind it",
    [BODY, POINT, field("size", "Thread", choice("M3", SIZES), "Nut size"), field("clearance", "Clearance", len(0.2, 0.0, 2.0), "Across the flats"),
     field("depth", "Pocket depth", len(0.0, 0.0, 100.0), "0: the nut's thickness and a little"), field("bolt_depth", "Bolt hole depth", len(20.0, 0.0, 500.0), "0: none"),
     field("bridge", "Bridge layer", len(0.0, 0.0, 2.0), "Leave this much between pocket and hole so it prints upside down without support")];
    "print.screw_hole", "Screw Hole", "FEATURES", "hole", false, design::screw_hole,
    "Cut a screw hole sized for printing, with or without a head recess",
    [BODY, POINT, field("size", "Thread", choice("M3", SIZES), "Screw size"), field("fit", "Fit", choice("clearance", &["clearance", "self_tap"]), "Clearance, or a hole the screw cuts its thread in"),
     field("depth", "Depth", len(20.0, 0.5, 500.0), "From the face"), field("head", "Head recess", choice("none", &["none", "counterbore", "countersink"]), "For the screw's head"),
     field("extra", "Extra diameter", len(0.1, -1.0, 2.0), "Printed holes come out small"),
     field("bridge", "Bridge layer", len(0.0, 0.0, 2.0), "Leave this much under a counterbore so it bridges")];
    "print.magnet_pocket", "Magnet Pocket", "FEATURES", "p3_magnet", false, design::magnet_pocket,
    "Cut a press-fit pocket for a disc or block magnet",
    [BODY, POINT, field("shape", "Shape", choice("disc", &["disc", "block"]), "Magnet shape"), field("diameter", "Diameter / length", len(6.0, 1.0, 200.0), "Of the magnet"),
     field("width", "Block width", len(6.0, 1.0, 200.0), "Block magnets only"), field("thickness", "Thickness", len(2.0, 0.5, 100.0), "Of the magnet"),
     field("clearance", "Clearance", len(0.15, -0.5, 2.0), "Extra size all round (diameter)"), field("cover", "Cover", len(0.0, 0.0, 10.0), "Buries the magnet this deep, to be dropped in at a print pause")];
    "print.teardrop_hole", "Teardrop Hole", "FEATURES", "p3_teardrop", false, design::teardrop,
    "Cut a sideways hole that prints without support: a circle with a 45° peak",
    [BODY, POINT, field("diameter", "Diameter", len(5.0, 0.5, 200.0), "Of the hole"), field("depth", "Depth", len(20.0, 0.5, 1000.0), "From the face"),
     field("flat", "Flatten peak", FieldKind::Bool(true), "Cut the peak off just above the circle, to bridge")];
    "print.lattice", "Lattice Fill", "FEATURES", "p3_lattice", false, design::lattice,
    "Replace a body's inside with a gyroid or other lattice, as a watertight mesh body; the original is hidden",
    [BODY, field("pattern", "Pattern", choice("gyroid", &["gyroid", "diamond", "schwarz", "cubic"]), "The surface"), field("cell", "Cell size", len(10.0, 1.0, 200.0), "Period of the pattern"),
     field("wall", "Wall", len(1.2, 0.2, 20.0), "Thickness of the pattern"), field("skin", "Skin", len(1.2, 0.0, 50.0), "Solid outside; 0 for an open lattice"),
     field("resolution", "Resolution", len(0.8, 0.1, 10.0), "Smaller is smoother and slower")];
    "print.hollow", "Hollow & Drain", "FEATURES", "shell", false, design::hollow,
    "Hollow a body to a wall thickness and add drain holes through the bottom",
    [BODY, field("wall", "Wall", len(2.0, 0.2, 100.0), "Left all round"), field("drains", "Drain holes", int(2, 0, 4), "Through the bottom"),
     field("diameter", "Hole diameter", len(4.0, 0.5, 50.0), "Of the drains")];
    "print.vent", "Vent Grille", "FEATURES", "perforate", false, design::vent,
    "Cut a honeycomb vent through a wall: hexagons print without support standing up",
    [BODY, POINT, field("width", "Width", len(30.0, 2.0, 1000.0), "Of the grille"), field("height", "Height", len(20.0, 2.0, 1000.0), "Of the grille"),
     field("hex", "Hexagon size", len(5.0, 1.0, 100.0), "Across the flats"), field("rib", "Rib", len(1.2, 0.4, 20.0), "Between hexagons"), field("depth", "Depth", len(5.0, 0.2, 500.0), "Through the wall")];
    "print.keyhole", "Keyhole Hanger", "FEATURES", "slot", false, design::keyhole,
    "Cut a keyhole slot to hang the part on a screw",
    [BODY, POINT, field("head", "Screw head", len(8.0, 2.0, 50.0), "Head diameter"), field("shank", "Screw shank", len(4.0, 1.0, 30.0), "Shank diameter"),
     field("travel", "Travel", len(10.0, 2.0, 100.0), "Slot length"), field("head_depth", "Head room", len(3.5, 1.0, 50.0), "Behind the lip"), field("lip", "Lip", len(2.0, 0.6, 20.0), "Thickness holding the head")];

    "print.hinge", "Print-in-Place Hinge", "MAKE", "p3_hinge", false, make::hinge,
    "A hinge that prints assembled: two leaves turning on captured cones",
    [AT, field("length", "Length", len(50.0, 15.0, 500.0), "Along the axis"), field("leaf", "Leaf width", len(20.0, 5.0, 200.0), "Each side"),
     field("thickness", "Leaf thickness", len(3.0, 1.0, 20.0), "Of the leaves"), field("barrel", "Barrel diameter", len(8.0, 4.0, 60.0), "Of the knuckles"),
     field("knuckles", "Knuckles", int(5, 3, 21), "An odd number"), field("clearance", "Clearance", len(0.35, 0.1, 1.5), "Between moving parts")];
    "print.enclosure", "Enclosure", "MAKE", "p3_enclosure", false, make::enclosure,
    "A box with a lid for electronics: rounded corners, screw posts for heat-set inserts and a locating lip",
    [AT, field("length", "Inside length", len(80.0, 10.0, 2000.0), "X"), field("width", "Inside width", len(50.0, 10.0, 2000.0), "Y"), field("height", "Inside height", len(30.0, 5.0, 2000.0), "Z"),
     field("wall", "Wall", len(2.4, 0.8, 20.0), "Sides, floor and lid"), field("radius", "Corner radius", len(6.0, 0.5, 100.0), "Outside corners"),
     field("size", "Screws", choice("M3", SIZES), "Corner screws"), field("posts", "Screw posts", FieldKind::Bool(true), "Posts with insert holes in the corners"),
     field("clearance", "Lid clearance", len(0.2, 0.0, 1.0), "Around the lip")];
    "print.grid_bin", "Grid Bin", "MAKE", "p3_bin", false, make::grid_bin,
    "A storage bin for 42 mm modular grid base plates: stepped feet, dividers, optional magnet holes",
    [AT, field("units_x", "Units X", int(2, 1, 12), "Of 42 mm"), field("units_y", "Units Y", int(1, 1, 12), "Of 42 mm"), field("units_z", "Height units", int(6, 2, 30), "Of 7 mm"),
     field("wall", "Wall", len(1.2, 0.8, 5.0), "Thickness"), field("dividers", "Dividers", int(0, 0, 20), "Across X"), field("magnets", "Magnet holes", FieldKind::Bool(false), "6 × 2 mm, in every foot")];
    "print.grid_base", "Grid Base Plate", "MAKE", "p3_base", false, make::grid_base,
    "The base plate 42 mm grid bins sit in",
    [AT, field("units_x", "Units X", int(3, 1, 12), "Of 42 mm"), field("units_y", "Units Y", int(2, 1, 12), "Of 42 mm")];
    "print.gear", "Printable Gear", "MAKE", "gear", false, make::gear,
    "An involute spur gear with backlash for printing, as a solid with a bore",
    [AT, field("teeth", "Teeth", int(20, 8, 120), "Number of teeth"), field("module", "Module", num(1.5, 0.5, 20.0, "mm"), "Pitch diameter over teeth"),
     field("thickness", "Thickness", len(8.0, 1.0, 200.0), "Along the axis"), field("bore", "Bore", len(5.0, 0.0, 200.0), "0: none"),
     field("pressure", "Pressure angle", num(20.0, 14.0, 30.0, "°"), "20° is usual"), field("backlash", "Backlash", len(0.2, 0.0, 2.0), "Thinner teeth by this much")];
    "print.knob", "Knob", "MAKE", "p3_knob", false, make::knob,
    "A grippy knob with a nut pocket, for a bolt head or captive nut",
    [AT, field("diameter", "Diameter", len(35.0, 12.0, 200.0), "Overall"), field("height", "Height", len(14.0, 5.0, 100.0), "Overall"),
     field("lobes", "Grips", int(7, 3, 24), "Scallops around the edge"), field("size", "Thread", choice("M5", SIZES), "Bolt")];
    "print.hook", "Wall Hook", "MAKE", "p3_hook", false, make::hook,
    "A screw-on hook that prints flat on its side, so its layers carry the load",
    [AT, field("width", "Width", len(16.0, 5.0, 100.0), "Printed height"), field("height", "Plate height", len(50.0, 20.0, 300.0), "Along the wall"),
     field("reach", "Reach", len(30.0, 10.0, 200.0), "Out from the wall"), field("thickness", "Thickness", len(6.0, 2.0, 40.0), "Of the arm"), field("size", "Screws", choice("M4", SIZES), "Mounting screws")];
    "print.spacer", "Spacer", "MAKE", "p3_spacer", false, make::spacer,
    "A round spacer or standoff with a clearance hole",
    [AT, field("size", "Thread", choice("M3", SIZES), "Screw"), field("height", "Height", len(10.0, 0.4, 500.0), "Of the spacer"), field("outer", "Outer diameter", len(0.0, 0.0, 200.0), "0: twice the screw's and a bit")];

    "print.calibration", "Calibration Part", "CALIBRATE", "p3_calibrate", false, make::calibration,
    "Make a calibration print for your printer and filament",
    [AT, field("kind", "Test", choice("cube", &["cube", "tolerance", "overhang", "bridging", "temperature_tower", "retraction", "first_layer", "thin_walls", "hole_sizes", "flow_cube"]), "Which test")];
    "print.fit_test", "Fit Test", "CALIBRATE", "p3_fit", false, fit::fit_test,
    "A strip of holes from tight to loose and a pin to try in them, to find your printer's clearance",
    [AT, field("diameter", "Pin diameter", len(8.0, 2.0, 50.0), "Nominal"), field("from", "Tightest", len(0.0, -0.5, 2.0), "Clearance of the first hole"),
     field("step", "Step", len(0.1, 0.02, 1.0), "Between holes"), field("count", "Holes", int(6, 2, 12), "How many")];
}

/// The printers known, by name.
pub fn printers() -> Vec<&'static str> {
    tables::PRINTERS.iter().map(|p| p.name).collect()
}

/// The filaments known, by name.
pub fn filaments() -> Vec<&'static str> {
    tables::FILAMENTS.iter().map(|p| p.name).collect()
}

pub fn tool(id: &str) -> Option<&'static Tool> {
    TOOLS.iter().chain(super::maker::TOOLS).find(|t| t.id == id)
}

/// The contract a tool's fields describe.
pub(crate) struct ToolContract(pub &'static str);

impl ToolContract {
    fn fields(&self) -> &'static [Field] {
        tool(self.0).map_or(&[], |t| t.fields)
    }
}

fn length_schema() -> Value {
    json!({"anyOf": [{"type": "number"}, {"type": "string", "minLength": 1, "maxLength": 4096}]})
}

impl Contract for ToolContract {
    fn input_schema(&self) -> schemars::Schema {
        let mut properties = Map::new();
        for f in self.fields() {
            let point = json!({"type": "array", "items": {"type": "number"}, "minItems": 3, "maxItems": 3});
            let (mut schema, note) = match f.kind {
                FieldKind::Length { default, min, max } => (length_schema(), format!(". Length in mm, {min} to {max}. Default: {default}")),
                FieldKind::Number { default, min, max, unit } => {
                    (json!({"type": "number", "minimum": min, "maximum": max}), format!(". In {unit}. Default: {default}"))
                }
                FieldKind::Int { default, min, max } => {
                    (json!({"type": "integer", "minimum": min, "maximum": max}), format!(". Default: {default}"))
                }
                FieldKind::Bool(d) => (json!({"type": "boolean"}), format!(". Default: {d}")),
                FieldKind::Choice { default, options } => (json!({"type": "string", "enum": options}), format!(". Default: {default}")),
                FieldKind::Bodies => (json!({"type": "array", "items": {"type": "string"}}), String::new()),
                FieldKind::Body | FieldKind::Text(_) | FieldKind::Font | FieldKind::File(_) => (json!({"type": "string"}), String::new()),
                FieldKind::Point => (point, String::new()),
                FieldKind::Position(d) => (point, format!(". Default: {d:?}")),
            };
            if let Some(o) = schema.as_object_mut() {
                o.insert("description".into(), json!(format!("{}{note}", f.help)));
            }
            properties.insert(f.key.into(), schema);
        }
        let help = tool(self.0).map_or("", |t| t.help);
        schemars::Schema::try_from(json!({
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "title": self.0, "description": help, "type": "object", "properties": properties, "additionalProperties": false,
        }))
        .unwrap_or_default()
    }

    fn allowed_keys(&self) -> Arc<BTreeSet<String>> {
        Arc::new(self.fields().iter().map(|f| f.key.to_string()).collect())
    }

    fn check(&self, params: &Value) -> Vec<Issue> {
        let object = match params {
            Value::Null => return Vec::new(),
            Value::Object(o) => o,
            _ => return vec![Issue { path: String::new(), message: "parameters must be an object".into(), suggestion: None }],
        };
        let mut out = Vec::new();
        for key in object.keys().take(64) {
            if !self.fields().iter().any(|f| f.key == key) {
                out.push(Issue { path: key.clone(), message: format!("unknown key `{key}`"), suggestion: None });
            }
        }
        for f in self.fields() {
            if let Some(v) = object.get(f.key)
                && let Err(message) = shape(f, v)
            {
                out.push(Issue { path: f.key.into(), message, suggestion: None });
            }
        }
        out
    }
}

/// Whether `v` has the right shape for `f`; expressions are evaluated later.
fn shape(f: &Field, v: &Value) -> std::result::Result<(), String> {
    let key = f.key;
    let range = |x: f64, min: f64, max: f64| {
        if x.is_finite() && x >= min && x <= max { Ok(()) } else { Err(format!("`{key}` must be between {min} and {max}")) }
    };
    match (f.kind, v) {
        (_, Value::Null) => Ok(()),
        (FieldKind::Length { .. }, Value::String(t)) if !t.trim().is_empty() && t.len() <= 4096 => Ok(()),
        (FieldKind::Length { min, max, .. } | FieldKind::Number { min, max, .. }, Value::Number(n)) => range(n.as_f64().unwrap_or(f64::NAN), min, max),
        (FieldKind::Int { min, max, .. }, Value::Number(n)) => match n.as_f64() {
            Some(x) if x.fract() == 0.0 => range(x, min as f64, max as f64),
            _ => Err(format!("`{key}` must be a whole number")),
        },
        (FieldKind::Bool(_), Value::Bool(_)) => Ok(()),
        (FieldKind::Choice { options, .. }, Value::String(t)) if options.contains(&t.as_str()) => Ok(()),
        (FieldKind::Choice { options, .. }, _) => Err(format!("`{key}` must be one of {}", options.join(", "))),
        (FieldKind::Bodies, Value::Array(a)) if a.iter().all(Value::is_string) => Ok(()),
        (FieldKind::Bodies | FieldKind::Body | FieldKind::Text(_) | FieldKind::Font | FieldKind::File(_), Value::String(_)) => Ok(()),
        (FieldKind::Point | FieldKind::Position(_), v) if crate::params::vec3(v).is_some() => Ok(()),
        (FieldKind::Point | FieldKind::Position(_), _) => Err(format!("`{key}` must be [x, y, z] of numbers")),
        _ => Err(format!("`{key}` has the wrong type")),
    }
}

pub(crate) fn print_output() -> schemars::Schema {
    schemars::Schema::try_from(json!({
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "title": "PrintResult", "type": "object",
        "description": "What the print tool found or made",
        "properties": {
            "summary": {"type": "string", "description": "The result in a sentence"},
            "created_bodies": {"type": "array", "items": {"type": "string"}},
            "warnings": {"type": "array", "items": {"type": "string"}},
        },
        "required": ["summary"], "additionalProperties": true,
    }))
    .unwrap_or_default()
}

/// A call's parameters, checked against the tool's fields.
pub(crate) struct Args {
    cmd: &'static str,
    fields: &'static [Field],
    values: Map<String, Value>,
}

impl Args {
    pub(in crate::cmd) fn read(s: &Session, cmd: &'static str, p: &Value) -> Result<Args> {
        let fields = tool(cmd).map_or(&[][..], |t| t.fields);
        let mut values = Map::new();
        for f in fields {
            let Some(v) = p.get(f.key).filter(|v| !v.is_null()) else { continue };
            shape(f, v).map_err(|e| bad(cmd, e))?;
            let v = match (f.kind, v) {
                (FieldKind::Length { min, max, .. }, Value::String(text)) => {
                    let x = s.doc.eval(text, Kind::Length).map_err(|e| bad(cmd, format!("{}: {e}", f.key)))?;
                    if !(x >= min && x <= max) {
                        return Err(bad(cmd, format!("`{}` must be between {min} and {max} mm", f.key)));
                    }
                    json!(x)
                }
                _ => v.clone(),
            };
            values.insert(f.key.into(), v);
        }
        Ok(Args { cmd, fields, values })
    }

    fn kind(&self, key: &str) -> Option<FieldKind> {
        self.fields.iter().find(|f| f.key == key).map(|f| f.kind)
    }

    pub(in crate::cmd) fn num(&self, key: &str) -> f64 {
        let given = self.values.get(key).and_then(Value::as_f64);
        given.unwrap_or(match self.kind(key) {
            Some(FieldKind::Length { default, .. } | FieldKind::Number { default, .. }) => default,
            Some(FieldKind::Int { default, .. }) => default as f64,
            _ => 0.0,
        })
    }

    pub(in crate::cmd) fn int(&self, key: &str) -> usize {
        self.num(key).max(0.0) as usize
    }

    pub(in crate::cmd) fn flag(&self, key: &str) -> bool {
        self.values.get(key).and_then(Value::as_bool).unwrap_or(matches!(self.kind(key), Some(FieldKind::Bool(true))))
    }

    pub(in crate::cmd) fn text(&self, key: &str) -> &str {
        self.values.get(key).and_then(Value::as_str).unwrap_or(match self.kind(key) {
            Some(FieldKind::Choice { default, .. } | FieldKind::Text(default)) => default,
            _ => "",
        })
    }

    /// `[x, y, z]` of the part to make.
    pub(in crate::cmd) fn at(&self) -> Vec3 {
        self.values.get("at").and_then(crate::params::vec3).unwrap_or(Vec3::new(0.0, 0.0, 0.0))
    }

    pub(in crate::cmd) fn fail(&self, msg: impl Into<String>) -> EngineError {
        bad(self.cmd, msg)
    }

    pub(in crate::cmd) fn fastener(&self) -> Result<&'static tables::Fastener> {
        tables::fastener_named(self.text("size")).ok_or_else(|| self.fail("unknown thread size"))
    }

    /// The bodies a call is about.
    pub(in crate::cmd) fn bodies(&self, s: &Session) -> Result<Vec<String>> {
        let world = s.world_state()?;
        let given: Vec<String> = match self.values.get("bodies") {
            Some(Value::Array(a)) => a.iter().filter_map(Value::as_str).map(str::to_string).collect(),
            Some(Value::String(n)) => vec![n.clone()],
            _ => Vec::new(),
        };
        for n in &given {
            if world.body(n).is_none() {
                return Err(self.fail(super::no_body(n, world.bodies.iter().map(|b| b.name.as_str()))));
            }
        }
        if !given.is_empty() {
            return Ok(given);
        }
        let mut selected: Vec<String> = Vec::new();
        for x in &s.selection {
            if let Sel::Body { name } | Sel::Face { body: name, .. } = x
                && world.body(name).is_some()
                && !selected.contains(name)
            {
                selected.push(name.clone());
            }
        }
        if !selected.is_empty() {
            return Ok(selected);
        }
        let visible: Vec<String> = world.bodies.iter().filter(|b| s.body_visible(&b.name)).map(|b| b.name.clone()).collect();
        if visible.is_empty() { Err(self.fail("there is no visible body to print")) } else { Ok(visible) }
    }

    /// The one body a call is about.
    pub(in crate::cmd) fn body(&self, s: &Session) -> Result<String> {
        let world = s.world_state()?;
        if let Some(n) = self.values.get("body").and_then(Value::as_str).filter(|n| !n.trim().is_empty()) {
            return match world.body(n) {
                Some(_) => Ok(n.to_string()),
                None => Err(self.fail(super::no_body(n, world.bodies.iter().map(|b| b.name.as_str())))),
            };
        }
        let mut all = self.bodies(s)?;
        match (all.pop(), all.is_empty()) {
            (Some(one), true) => Ok(one),
            _ => Err(self.fail("select one body, or give `body`")),
        }
    }

    /// The body, the point on it and the face's outward normal there.
    pub(in crate::cmd) fn spot(&self, s: &Session) -> Result<(String, Vec3, Vec3)> {
        let given = self.values.get("point").and_then(crate::params::vec3);
        let picked = s.selection.iter().find_map(|x| match x {
            Sel::Face { body, point, .. } => Some((body.clone(), *point)),
            _ => None,
        });
        let (body, point) = match (given, picked, self.values.get("body").and_then(Value::as_str)) {
            (Some(p), _, _) => (self.body(s)?, p),
            (None, Some((b, p)), None) => (b, p),
            (None, Some((b, p)), Some(named)) if b == named => (b, p),
            _ => return Err(self.fail("select a flat face of the body, or give `point`")),
        };
        let mesh = mesh_of(s, std::slice::from_ref(&body))?;
        // The nearest triangle to the point says which way the face looks.
        let near = mesh
            .tris()
            .map(|t| (tri_distance(t, point), rybocraft_print3d::normal_area(t)))
            .filter(|(_, (_, a))| *a > 0.0)
            .min_by(|a, b| a.0.total_cmp(&b.0));
        match near {
            Some((d, (n, _))) if d <= 0.5 => Ok((body, point, n)),
            _ => Err(self.fail("`point` is not on the body")),
        }
    }
}

fn tri_distance(t: [Vec3; 3], p: Vec3) -> f64 {
    let set = rybocraft_geom::mesh_distance::TriangleSet::new(&rybocraft_geom::Mesh {
        positions: t.to_vec(),
        triangles: vec![[0, 1, 2]],
        ..rybocraft_geom::Mesh::default()
    });
    set.distance_to(p, &[0])
}

/// The bodies' meshes together, in the world.
pub(in crate::cmd) fn mesh_of(s: &Session, names: &[String]) -> Result<PrintMesh> {
    let world = s.world_state()?;
    let mut out = PrintMesh::default();
    for n in names {
        let b = world.body(n).ok_or_else(|| EngineError::Other(format!("no body `{n}`")))?;
        let m = b.mesh();
        out.append(&m.positions, &m.triangles);
    }
    if out.triangles.is_empty() {
        return Err(EngineError::Other("the bodies have no surface to print".into()));
    }
    Ok(out)
}

/// What an analysis shows in the view, until the design changes.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PrintOverlay {
    pub revision: u64,
    /// Filled triangles and their colour role.
    pub triangles: Vec<(OverlayRole, [Vec3; 3])>,
    pub lines: Vec<(OverlayRole, [Vec3; 2])>,
    pub points: Vec<(OverlayRole, Vec3)>,
    pub label: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OverlayRole {
    /// Needs support.
    Overhang,
    /// Too thin.
    Thin,
    /// Starts in mid-air.
    Island,
    /// A layer's outline.
    Layer,
}

fn show(s: &mut Session, mut o: PrintOverlay) {
    o.revision = s.revision;
    s.print_overlay = Some(Arc::new(o));
}

fn clear_overlay(s: &mut Session, _: &Args) -> Result<Value> {
    s.print_overlay = None;
    Ok(json!({"summary": "Highlights cleared"}))
}

/// The printer and filament the tools work with.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct PrintSetup {
    pub printer: String,
    pub bed: [f64; 3],
    pub material: String,
    pub layer: f64,
    pub nozzle: f64,
    pub walls: usize,
    pub skins: usize,
    /// Percent.
    pub infill: f64,
    pub price: f64,
}

impl Default for PrintSetup {
    fn default() -> Self {
        PrintSetup {
            printer: "Generic 220".into(),
            bed: [220.0, 220.0, 250.0],
            material: "PLA".into(),
            layer: 0.2,
            nozzle: 0.4,
            walls: 3,
            skins: 4,
            infill: 15.0,
            price: 20.0,
        }
    }
}

impl PrintSetup {
    pub fn filament(&self) -> tables::Filament {
        tables::filament_named(&self.material).copied().unwrap_or(tables::Filament {
            name: "PLA",
            density: 1.24,
            shrinkage: 0.2,
            overhang: 40.0,
            price: 20.0,
        })
    }
    /// Walls and infill speed, and flow.
    fn speeds(&self) -> ([f64; 2], f64) {
        tables::printer_named(&self.printer).map_or(([60.0, 80.0], 10.0), |p| (p.speed, p.max_flow))
    }
    fn sane(self) -> PrintSetup {
        let d = PrintSetup::default();
        let ok = |x: f64, lo: f64, hi: f64| x.is_finite() && x >= lo && x <= hi;
        PrintSetup {
            bed: if self.bed.iter().all(|x| ok(*x, 10.0, 5000.0)) { self.bed } else { d.bed },
            layer: if ok(self.layer, 0.02, 2.0) { self.layer } else { d.layer },
            nozzle: if ok(self.nozzle, 0.1, 3.0) { self.nozzle } else { d.nozzle },
            walls: self.walls.clamp(1, 50),
            skins: self.skins.min(50),
            infill: if ok(self.infill, 0.0, 100.0) { self.infill } else { d.infill },
            price: if ok(self.price, 0.0, 1e5) { self.price } else { d.price },
            ..self
        }
    }
}

static SETUP: std::sync::Mutex<Option<PrintSetup>> = std::sync::Mutex::new(None);

/// Where the setup is remembered; nowhere in tests and in the browser.
fn setup_file() -> Option<std::path::PathBuf> {
    if cfg!(any(test, target_arch = "wasm32")) { None } else { Some(crate::recovery::data_dir()?.join("print-setup.json")) }
}

pub fn setup() -> PrintSetup {
    let mut held = SETUP.lock().unwrap_or_else(|e| e.into_inner());
    held.get_or_insert_with(|| {
        let saved = setup_file().and_then(|p| std::fs::read_to_string(p).ok()).filter(|t| t.len() < 1 << 16);
        saved.and_then(|t| serde_json::from_str::<PrintSetup>(&t).ok()).unwrap_or_default().sane()
    })
    .clone()
}

fn setup_cmd(_: &mut Session, a: &Args) -> Result<Value> {
    let mut p = setup();
    let name = a.text("printer").trim().to_string();
    if !name.is_empty() {
        if let Some(found) = tables::printer_named(&name) {
            p.printer = found.name.into();
            p.bed = found.bed;
        } else if name.eq_ignore_ascii_case("custom") {
            p.printer = "Custom".into();
        } else {
            let known: Vec<&str> = tables::PRINTERS.iter().map(|p| p.name).collect();
            return Err(a.fail(format!("no printer `{name}`; known: {}, or custom", known.join(", "))));
        }
    }
    for (slot, key) in p.bed.iter_mut().zip(["bed_x", "bed_y", "bed_z"]) {
        if a.num(key) > 0.0 {
            *slot = a.num(key);
        }
    }
    let material = a.text("material").trim().to_string();
    if !material.is_empty() {
        let found = tables::filament_named(&material).ok_or_else(|| {
            let known: Vec<&str> = tables::FILAMENTS.iter().map(|p| p.name).collect();
            a.fail(format!("no filament `{material}`; known: {}", known.join(", ")))
        })?;
        p.material = found.name.into();
        p.price = found.price;
    }
    if a.num("layer") > 0.0 {
        p.layer = a.num("layer");
    }
    if a.num("nozzle") > 0.0 {
        p.nozzle = a.num("nozzle");
    }
    if a.int("walls") > 0 {
        p.walls = a.int("walls");
    }
    if a.num("infill") >= 0.0 {
        p.infill = a.num("infill");
    }
    if a.num("price") >= 0.0 {
        p.price = a.num("price");
    }
    let p = p.sane();
    let mut warnings = Vec::new();
    if p.layer > p.nozzle * 0.8 + 1e-9 {
        warnings.push(format!("a layer of {} mm is too thick for a {} mm nozzle", p.layer, p.nozzle));
    }
    if let Some(file) = setup_file()
        && let Ok(text) = serde_json::to_string_pretty(&p)
    {
        let saved = file.parent().map_or(Ok(()), std::fs::create_dir_all).and_then(|()| std::fs::write(&file, text));
        if let Err(e) = saved {
            warnings.push(format!("the setup could not be saved: {e}"));
        }
    }
    let summary = format!(
        "{} ({} × {} × {} mm), {}, {} mm layers, {} walls, {}% infill",
        p.printer, p.bed[0], p.bed[1], p.bed[2], p.material, p.layer, p.walls, p.infill
    );
    *SETUP.lock().unwrap_or_else(|e| e.into_inner()) = Some(p.clone());
    Ok(json!({"summary": summary, "setup": p, "printers": tables::PRINTERS, "filaments": tables::FILAMENTS, "warnings": warnings}))
}

// ---- running other commands --------------------------------------------------------------------

/// The bodies a command's reply says it made.
pub(in crate::cmd) fn created(v: &Value) -> Vec<String> {
    v.get("created_bodies").and_then(Value::as_array).map_or(Vec::new(), |a| a.iter().filter_map(Value::as_str).map(str::to_string).collect())
}

pub(in crate::cmd) fn xyz(v: Vec3) -> Value {
    json!([v.x, v.y, v.z])
}

/// A length for another command: it says mm, whatever units the design uses.
pub(in crate::cmd) fn mm(x: f64) -> Value {
    json!(format!("{x} mm"))
}

/// What a solid does to the model.
#[derive(Clone, Copy)]
pub(in crate::cmd) enum Do<'a> {
    New(&'a str),
    Join(&'a str),
    Cut(&'a str),
}

impl Do<'_> {
    fn put(self, p: &mut Value) {
        let Some(o) = p.as_object_mut() else { return };
        match self {
            Do::New(name) => {
                o.insert("operation".into(), json!("new"));
                o.insert("body_name".into(), json!(name));
            }
            Do::Join(body) => {
                o.insert("operation".into(), json!("join"));
                o.insert("targets".into(), json!([body]));
            }
            Do::Cut(body) => {
                o.insert("operation".into(), json!("cut"));
                o.insert("targets".into(), json!([body]));
            }
        }
    }
    /// The body the result is on.
    fn body(self, reply: &Value) -> String {
        match self {
            Do::New(name) => created(reply).into_iter().next().unwrap_or_else(|| name.to_string()),
            Do::Join(b) | Do::Cut(b) => b.to_string(),
        }
    }
}

pub(in crate::cmd) fn solid(s: &mut Session, id: &str, mut p: Value, what: Do) -> Result<String> {
    what.put(&mut p);
    let r = s.execute(id, &p)?;
    Ok(what.body(&r))
}

pub(in crate::cmd) fn cylinder(s: &mut Session, base: Vec3, axis: Vec3, diameter: f64, height: f64, what: Do) -> Result<String> {
    solid(s, "solid.cylinder", json!({"diameter": mm(diameter), "height": mm(height), "base": xyz(base), "axis": xyz(axis)}), what)
}

pub(in crate::cmd) fn cone(s: &mut Session, base: Vec3, axis: Vec3, from: f64, to: f64, height: f64, what: Do) -> Result<String> {
    solid(s, "solid.cone", json!({"diameter": mm(from), "top_diameter": mm(to), "height": mm(height), "base": xyz(base), "axis": xyz(axis)}), what)
}

/// A box from its lowest corner.
pub(in crate::cmd) fn cuboid(s: &mut Session, corner: Vec3, size: [f64; 3], what: Do) -> Result<String> {
    solid(s, "solid.box", json!({"length": mm(size[0]), "width": mm(size[1]), "height": mm(size[2]), "corner": xyz(corner)}), what)
}

/// A flat frame to draw a profile in: the prism grows along `x` × `y`.
#[derive(Clone, Copy)]
pub(in crate::cmd) struct Flat {
    pub origin: Vec3,
    pub x: Vec3,
    pub y: Vec3,
}

impl Flat {
    pub fn level(origin: Vec3) -> Flat {
        Flat { origin, x: Vec3::new(1.0, 0.0, 0.0), y: Vec3::new(0.0, 1.0, 0.0) }
    }
    /// On a face looking along `-into`, growing into it, with `y` as near to world up as it goes.
    pub fn into_face(origin: Vec3, into: Vec3) -> Flat {
        let up = if into.z.abs() > 0.99 { Vec3::new(0.0, 1.0, 0.0) } else { Vec3::new(0.0, 0.0, 1.0) };
        let y = (up - into * up.dot(into)).normalized().unwrap_or(Vec3::new(0.0, 1.0, 0.0));
        Flat { origin, x: y.cross(into), y }
    }
    fn plane(&self) -> Value {
        json!({"origin": xyz(self.origin), "x_dir": xyz(self.x), "y_dir": xyz(self.y)})
    }
}

/// Run `draw` in a new sketch on `flat`, then extrude everything drawn.
pub(in crate::cmd) fn sketched(
    s: &mut Session,
    flat: Flat,
    height: f64,
    taper: f64,
    what: Do,
    draw: impl FnOnce(&mut Session) -> Result<()>,
) -> Result<String> {
    let open = s.execute("sketch.create", &json!({"plane": flat.plane()}))?;
    draw(s)?;
    s.execute("sketch.finish", &json!({}))?;
    let mut p = json!({"distance": mm(height)});
    if let Some(o) = p.as_object_mut() {
        if let Some(id) = open.get("feature").or_else(|| open.get("sketch")).filter(|v| v.is_u64()) {
            o.insert("sketch".into(), id.clone());
        }
        if taper != 0.0 {
            o.insert("taper".into(), json!(format!("{taper} deg")));
        }
    }
    solid(s, "solid.extrude", p, what)
}

pub(in crate::cmd) fn polygon(s: &mut Session, points: &[[f64; 2]]) -> Result<()> {
    s.execute("sketch.line", &json!({"points": points, "closed": true})).map(|_| ())
}

/// A prism over a polygon.
pub(in crate::cmd) fn prism(s: &mut Session, flat: Flat, points: &[[f64; 2]], height: f64, what: Do) -> Result<String> {
    sketched(s, flat, height, 0.0, what, |s| polygon(s, points))
}

/// A prism over a rounded rectangle centred on the frame, its sides leaning by `taper` degrees.
pub(in crate::cmd) fn rounded(s: &mut Session, flat: Flat, size: [f64; 2], radius: f64, height: f64, taper: f64, what: Do) -> Result<String> {
    let (w, h) = (size[0] * 0.5, size[1] * 0.5);
    sketched(s, flat, height, taper, what, |s| {
        if radius > 0.01 {
            s.execute("sketch.rectangle.rounded", &json!({"p0": [-w, -h], "p1": [w, h], "radius": mm(radius.min(w.min(h) - 0.01))})).map(|_| ())
        } else {
            polygon(s, &[[-w, -h], [w, -h], [w, h], [-w, h]])
        }
    })
}

/// A regular hexagon by its width across the flats, two corners on the Y axis.
pub(in crate::cmd) fn hexagon(flats: f64, at: [f64; 2]) -> Vec<[f64; 2]> {
    let r = flats / 3f64.sqrt();
    (0..6)
        .map(|k| {
            let a = (f64::from(k) * 60.0 + 30.0).to_radians();
            [at[0] + r * a.cos(), at[1] + r * a.sin()]
        })
        .collect()
}

/// Add a closed mesh to the design as a mesh body.
pub(in crate::cmd) fn add_mesh_body(s: &mut Session, name: &str, m: &PrintMesh) -> Result<Vec<String>> {
    let positions: Vec<f64> = m.positions.iter().flat_map(|p| [p.x, p.y, p.z]).collect();
    let triangles: Vec<u32> = m.triangles.iter().flatten().copied().collect();
    let mesh = rybocraft_doc::MeshData {
        name: name.to_string(),
        positions: positions.into(),
        triangles: triangles.into(),
        corner_normals: Default::default(),
        color: None,
    };
    let kind = rybocraft_doc::FeatureKind::MeshImport { file: name.to_string(), meshes: vec![mesh] };
    let r = super::features::add_feature(s, &json!({"name": name, "body_name": name}), kind)?;
    Ok(created(&r))
}

#[cfg(test)]
mod tests;
