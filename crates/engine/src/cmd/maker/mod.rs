//! The Maker tools (`make.*`): ready models to shape by a few numbers, and edits for models that
//! came as a mesh. They share the print tools' field tables, so the app's form, the command's
//! schema and its defaults all come from one line each.

mod edit;
mod gifts;
mod hardware;
mod picture;
mod storage;

use rybocraft_print3d::PrintMesh;
use rybocraft_print3d::sdf::{self, Source};
use serde_json::{Value, json};
use rybocore_record::Vec3;
use rybocraft_geom::prelude::*;

pub(in crate::cmd) use edit::hollow_mesh;

use super::print::*;
use crate::{Result, Session};

const FONT: Field = field("font", "Font", FieldKind::Font, "An installed font by name, or a font file; empty for the built-in one");
const DETAIL: Field = field("detail", "Detail", len(0.5, 0.1, 5.0), "Size of the smallest feature: smaller is finer and slower");
const IMAGE: Field = field("image", "Image", FieldKind::File(&["png", "jpg", "jpeg"]), "A PNG or JPEG picture");

pub const PANELS: [&str; 5] = ["STORAGE", "NAMES & GIFTS", "PICTURES", "HARDWARE & HOME", "EDIT MODEL"];

tools! { "MAKER";
    "make.tray", "Storage Tray", "STORAGE", "p3_bin", false, storage::tray,
    "A tray with compartments: columns, each split into its own number of spaces",
    [AT, field("length", "Length", len(180.0, 20.0, 1000.0), "Outside, X"), field("width", "Width", len(120.0, 20.0, 1000.0), "Outside, Y"), field("height", "Height", len(35.0, 5.0, 500.0), "Outside, Z"),
     field("columns", "Spaces per column", FieldKind::Text("2,2,2"), "One number per column, such as 2,1,3"),
     field("widths", "Column sizes", FieldKind::Text(""), "Relative sizes such as 1,2,1; empty for equal"),
     field("wall", "Wall", len(2.0, 0.8, 20.0), "Outside walls and floor"), field("divider", "Divider", len(1.6, 0.8, 20.0), "Dividers"),
     field("drop", "Divider drop", len(0.0, 0.0, 200.0), "How far dividers stay below the rim"), field("radius", "Corner radius", len(8.0, 0.0, 100.0), "Outside corners"),
     field("lid", "Lid", FieldKind::Bool(false), "A lift-off lid beside it")];
    "make.cabinet", "Drawer Cabinet", "STORAGE", "p3_enclosure", false, storage::cabinet,
    "A cabinet of drawers, trays and open cubbies, with the drawers made to slide",
    [AT, field("columns", "Columns", int(3, 1, 8), "Across"), field("rows", "Rows", int(2, 1, 10), "Up"),
     field("cell_w", "Cell width", len(80.0, 20.0, 300.0), "Inside"), field("cell_h", "Cell height", len(50.0, 12.0, 300.0), "Inside"), field("depth", "Depth", len(120.0, 20.0, 400.0), "Outside"),
     field("cells", "Cells", FieldKind::Text("drawer"), "drawer, tray or cubby for each cell from the lower left, separated by commas; the last repeats"),
     field("wall", "Wall", len(2.0, 0.8, 10.0), "Housing and drawers"), field("clearance", "Clearance", len(0.4, 0.1, 2.0), "Around drawers"),
     field("dividers", "Drawer dividers", int(0, 0, 8), "In every drawer"), field("handle", "Handle", choice("slot", &["slot", "knob", "scoop"]), "Drawer pull")];
    "make.swing_tower", "Swing Tower", "STORAGE", "p3_hinge", false, storage::swing_tower,
    "A tower of drawers that swing out around a corner post",
    [AT, field("drawers", "Drawers", int(4, 1, 12), "How many"), field("width", "Width", len(110.0, 40.0, 300.0), "Outside"), field("depth", "Depth", len(80.0, 40.0, 300.0), "Outside"),
     field("drawer_h", "Drawer height", len(24.0, 10.0, 150.0), "Each"), field("wall", "Wall", len(2.4, 1.2, 10.0), "Housing and drawers"), field("clearance", "Clearance", len(0.4, 0.2, 2.0), "Between moving parts"),
     field("labels", "Labels", FieldKind::Text("BITS,SCREWS,TAPE,SPARES"), "Raised on the fronts, from the bottom up"), FONT, field("open", "Show open", num(0.0, 0.0, 110.0, "°"), "Turns the top drawer for a look")];
    "make.hinged_box", "Hinged Box", "STORAGE", "p3_hinge", false, storage::hinged_box,
    "A box that prints open and flat with its hinge already assembled, with compartments and a snap",
    [AT, field("length", "Length", len(90.0, 30.0, 300.0), "Along the hinge"), field("width", "Width", len(60.0, 20.0, 300.0), "Across"), field("height", "Half height", len(16.0, 8.0, 100.0), "Of each half"),
     field("wall", "Wall", len(2.0, 1.2, 8.0), "All walls"), field("radius", "Corner radius", len(5.0, 0.0, 40.0), "Upright corners"),
     field("cols", "Columns", int(3, 1, 12), "Compartments along"), field("rows", "Rows", int(2, 1, 12), "Compartments across"), field("clearance", "Clearance", len(0.35, 0.15, 1.0), "In the hinge")];
    "make.pegboard", "Pegboard Holder", "STORAGE", "p3_hook", false, storage::pegboard,
    "A hook, bin or shelf for round-hole pegboard (25.4 mm pitch, 6.35 mm holes)",
    [AT, field("kind", "Kind", choice("hook", &["hook", "bin", "shelf"]), "What it holds"), field("holes", "Pegs across", int(1, 1, 8), "25.4 mm apart"),
     field("reach", "Reach", len(40.0, 10.0, 200.0), "Out from the board"), field("height", "Height", len(40.0, 15.0, 200.0), "Bins and the back plate"), field("peg", "Peg diameter", len(5.8, 3.0, 8.0), "A little under the hole")];
    "make.cup", "Desk Cup", "STORAGE", "cylinder", false, storage::cup,
    "A pen cup or caddy with any number of sides and a name on the front",
    [AT, field("sides", "Sides", int(6, 3, 64), "64 is round"), field("diameter", "Diameter", len(80.0, 20.0, 400.0), "Across the corners"), field("height", "Height", len(100.0, 10.0, 400.0), "Overall"),
     field("wall", "Wall", len(2.0, 0.8, 20.0), "Sides and floor"), field("text", "Text", FieldKind::Text(""), "Raised on one side"), FONT, field("size", "Text size", len(14.0, 3.0, 100.0), "Capital height")];

    "make.text", "3D Text", "NAMES & GIFTS", "text", false, gifts::text,
    "Solid lettering in any font, alone or on a backing plate",
    [AT, field("text", "Text", FieldKind::Text("RyboCraft"), "What it says; \\n starts a new line"), FONT, field("size", "Size", len(20.0, 2.0, 500.0), "Capital height"), field("depth", "Depth", len(4.0, 0.4, 200.0), "Of the letters"),
     field("base", "Backing", choice("none", &["none", "plate", "rounded"]), "Behind the letters"), field("margin", "Margin", len(5.0, 0.0, 100.0), "Of the backing"), field("base_depth", "Backing depth", len(2.0, 0.4, 100.0), "Thickness"), field("standing", "Standing", FieldKind::Bool(false), "Upright instead of lying flat")];
    "make.keychain", "Name Keychain", "NAMES & GIFTS", "p3_ears", false, gifts::keychain,
    "A name on a tag with a ring hole; the letters are their own body for a second colour",
    [AT, field("text", "Name", FieldKind::Text("RYAN"), "What it says"), FONT, field("size", "Size", len(12.0, 4.0, 80.0), "Capital height"), field("base", "Tag depth", len(2.4, 0.8, 20.0), "Thickness"),
     field("raised", "Letter depth", len(1.2, 0.2, 10.0), "Above the tag"), field("margin", "Margin", len(4.0, 1.0, 40.0), "Around the letters"), field("hole", "Ring hole", len(4.5, 1.0, 20.0), "Diameter")];
    "make.nameplate", "Nameplate", "NAMES & GIFTS", "text", false, gifts::nameplate,
    "A desk nameplate on a slope, or a flat plate with screw holes",
    [AT, field("text", "Text", FieldKind::Text("R.G. Studios"), "What it says"), FONT, field("size", "Size", len(18.0, 4.0, 200.0), "Capital height"), field("margin", "Margin", len(10.0, 2.0, 100.0), "Around the letters"),
     field("raised", "Letter depth", len(1.6, 0.2, 20.0), "Height of the letters"), field("style", "Style", choice("desk", &["desk", "wall"]), "Standing or screwed on"), field("thickness", "Thickness", len(4.0, 1.0, 40.0), "Of a wall plate")];
    "make.cake_topper", "Cake Topper", "NAMES & GIFTS", "p3_drop", false, gifts::cake_topper,
    "Joined lettering on a bar with stakes",
    [AT, field("text", "Text", FieldKind::Text("Happy Day"), "What it says"), FONT, field("size", "Size", len(30.0, 8.0, 200.0), "Capital height"), field("depth", "Depth", len(3.0, 1.0, 20.0), "Thickness"), field("stakes", "Stakes", int(2, 1, 5), "How many"), field("stake", "Stake length", len(60.0, 10.0, 200.0), "Below the bar")];
    "make.balloon", "Balloon Letters", "NAMES & GIFTS", "sphere", false, gifts::balloon,
    "Puffy inflated letters and numbers",
    [AT, field("text", "Text", FieldKind::Text("PARTY"), "What it says"), FONT, field("size", "Size", len(40.0, 8.0, 300.0), "Capital height"), field("puff", "Puff", len(8.0, 1.0, 80.0), "Half the thickness"), field("flat", "Flat back", FieldKind::Bool(true), "Cut the back off flat"), DETAIL];
    "make.ornament", "Lattice Ornament", "NAMES & GIFTS", "p3_lattice", false, gifts::ornament,
    "A round woven ornament with a hanging loop",
    [AT, field("diameter", "Diameter", len(60.0, 20.0, 200.0), "Overall"), field("strands", "Strands", int(10, 3, 40), "Each way around"), field("twist", "Twist", num(1.0, 0.0, 4.0, "turns"), "Pole to pole; 0 for straight ribs"),
     field("thickness", "Strand", len(2.4, 0.8, 12.0), "Thickness"), field("squash", "Height", num(100.0, 40.0, 200.0, "%"), "Of the diameter"), field("loop", "Hanging loop", FieldKind::Bool(true), "On top"), DETAIL];
    "make.snowflake", "Snowflake", "NAMES & GIFTS", "pattern_circ", false, gifts::snowflake,
    "A snowflake grown from one branching arm; change the seed for another",
    [AT, field("diameter", "Diameter", len(80.0, 20.0, 300.0), "Overall"), field("arms", "Arms", int(6, 3, 12), "Around"), field("branches", "Branches", int(4, 0, 10), "Per arm"), field("seed", "Seed", int(7, 0, 9999), "Another snowflake"),
     field("line", "Line", len(2.4, 0.8, 20.0), "Width"), field("thickness", "Thickness", len(2.4, 0.4, 20.0), "Depth"), field("loop", "Hanging loop", FieldKind::Bool(false), "On one arm"), DETAIL];
    "make.coaster", "Coaster", "NAMES & GIFTS", "cylinder", false, gifts::coaster,
    "A coaster with a rim and raised text",
    [AT, field("shape", "Shape", choice("round", &["round", "square", "hexagon"]), "Outline"), field("diameter", "Size", len(95.0, 30.0, 300.0), "Across"), field("thickness", "Thickness", len(4.0, 1.0, 20.0), "Floor"), field("rim", "Rim", len(2.0, 0.0, 10.0), "Height above the floor"),
     field("text", "Text", FieldKind::Text("CHEERS"), "Raised in the middle"), FONT, field("size", "Text size", len(14.0, 3.0, 100.0), "Capital height")];
    "make.qr", "QR Code", "NAMES & GIFTS", "pattern_rect", false, gifts::qr,
    "A scannable QR code tag, with the code as its own body for a second colour",
    [AT, field("text", "Link or text", FieldKind::Text("https://example.com"), "What scanning gives"), field("size", "Size", len(60.0, 15.0, 300.0), "Across the plate"), field("base", "Plate", len(2.0, 0.6, 20.0), "Thickness"), field("raised", "Code height", len(0.8, 0.2, 5.0), "Above the plate"), field("hole", "Ring hole", len(0.0, 0.0, 20.0), "0: none")];

    "make.lithophane", "Lithophane", "PICTURES", "canvas", false, picture::lithophane,
    "A picture that shows when light shines through it, flat or curved",
    [AT, IMAGE, field("width", "Width", len(100.0, 20.0, 400.0), "Of the picture"), field("thin", "Thinnest", len(0.6, 0.2, 5.0), "Bright parts"), field("thick", "Thickest", len(3.0, 0.6, 20.0), "Dark parts"),
     field("border", "Border", len(3.0, 0.0, 30.0), "Full thickness, all round"), field("curve", "Curve", num(0.0, 0.0, 350.0, "°"), "How far around a cylinder; 0 is flat"), field("pitch", "Pixel", len(0.4, 0.1, 2.0), "Smaller is finer"), field("invert", "Invert", FieldKind::Bool(false), "Swap bright and dark")];
    "make.relief", "Picture Relief", "PICTURES", "emboss", false, picture::relief,
    "A picture as a raised relief on a plate",
    [AT, IMAGE, field("width", "Width", len(100.0, 10.0, 400.0), "Of the picture"), field("base", "Plate", len(2.0, 0.4, 20.0), "Thickness"), field("height", "Relief", len(3.0, 0.2, 40.0), "Height of the brightest part"), field("smooth", "Smoothing", int(1, 0, 10), "Softens noise"), field("pitch", "Pixel", len(0.4, 0.1, 2.0), "Smaller is finer"), field("invert", "Invert", FieldKind::Bool(false), "Raise dark parts instead")];
    "make.logo", "Picture Cut-Out", "PICTURES", "silhouette", false, picture::logo,
    "A logo or silhouette cut out by brightness, alone, on a plate, as a stamp, or with a keychain ring",
    [AT, IMAGE, field("width", "Width", len(60.0, 5.0, 400.0), "Of the picture"), field("threshold", "Threshold", num(50.0, 1.0, 99.0, "%"), "Darker than this is solid"), field("depth", "Depth", len(3.0, 0.4, 50.0), "Of the shape"),
     field("style", "Style", choice("shape", &["shape", "plate", "stamp", "keychain"]), "What to make of it"), field("invert", "Invert", FieldKind::Bool(false), "Keep bright parts instead"), DETAIL];
    "make.cookie_cutter", "Cookie Cutter", "PICTURES", "p3_teardrop", false, picture::cookie_cutter,
    "A cutter from a built-in shape or a picture's outline",
    [AT, field("shape", "Shape", choice("heart", &["heart", "star", "circle", "flower", "hexagon", "picture"]), "Outline"), IMAGE, field("width", "Width", len(70.0, 15.0, 250.0), "Across"), field("height", "Height", len(14.0, 5.0, 50.0), "Of the blade"),
     field("wall", "Blade", len(1.0, 0.5, 4.0), "Thickness"), field("rim", "Rim", len(4.0, 0.0, 20.0), "Width of the grip"), field("threshold", "Threshold", num(50.0, 1.0, 99.0, "%"), "For pictures"), DETAIL];
    "make.svg", "SVG to Solid", "PICTURES", "svg", false, picture::svg,
    "A vector drawing given depth, as an editable solid",
    [AT, field("image", "SVG file", FieldKind::File(&["svg"]), "The drawing"), field("depth", "Depth", len(3.0, 0.2, 100.0), "Thickness")];

    "make.bolt", "Bolt & Nut", "HARDWARE & HOME", "fastener", false, hardware::bolt,
    "A threaded bolt and its nut, with clearance for printing",
    [AT, field("size", "Size", choice("M8", &["M4", "M5", "M6", "M8"]), "Thread"), field("length", "Length", len(30.0, 5.0, 300.0), "Under the head"), field("head", "Head", choice("hex", &["hex", "knob", "none"]), "Shape"), field("nut", "Nut", FieldKind::Bool(true), "Make one too"), field("clearance", "Clearance", len(0.3, 0.0, 1.5), "Between the threads")];
    "make.bracket", "Bracket", "HARDWARE & HOME", "flange", false, hardware::bracket,
    "An angle bracket with screw holes and gussets",
    [AT, field("a", "Leg A", len(50.0, 10.0, 500.0), "Along the floor"), field("b", "Leg B", len(50.0, 10.0, 500.0), "Upright"), field("width", "Width", len(30.0, 6.0, 500.0), "Across"), field("thickness", "Thickness", len(5.0, 1.2, 50.0), "Of the legs"),
     field("size", "Screws", choice("M4", &["M2", "M2.5", "M3", "M4", "M5", "M6", "M8"]), "Holes"), field("holes", "Holes per leg", int(2, 0, 8), "Along each leg"), field("gussets", "Gussets", int(1, 0, 4), "Triangular braces"), field("gusset", "Gusset thickness", len(4.0, 1.0, 50.0), "Of each")];
    "make.vase", "Vase & Planter", "HARDWARE & HOME", "revolve", false, hardware::vase,
    "A vase or plant pot with ribs, twists or facets, and an optional drain and saucer",
    [AT, field("height", "Height", len(120.0, 20.0, 400.0), "Overall"), field("bottom", "Bottom diameter", len(60.0, 10.0, 400.0), "At the base"), field("middle", "Middle diameter", len(90.0, 10.0, 400.0), "Half-way"), field("top", "Top diameter", len(70.0, 10.0, 400.0), "At the rim"),
     field("wall", "Wall", len(2.0, 0.4, 20.0), "Sides and floor"), field("pattern", "Pattern", choice("ribs", &["smooth", "ribs", "facets", "waves", "petals"]), "Around it"), field("count", "Count", int(16, 3, 60), "Of the pattern"), field("depth", "Pattern depth", len(2.5, 0.0, 30.0), "In and out"), field("twist", "Twist", num(60.0, -360.0, 360.0, "°"), "Base to rim"), field("drain", "Drain hole", len(0.0, 0.0, 60.0), "0: none"), field("saucer", "Saucer", FieldKind::Bool(false), "One to stand it in")];
    "make.dice", "Dice", "HARDWARE & HOME", "box", false, hardware::dice,
    "Dice with 4, 6, 8, 12 or 20 sides: pips on a six, numbers on the rest",
    [AT, field("sides", "Sides", choice("6", &["4", "6", "8", "12", "20"]), "Which die"), field("size", "Size", len(18.0, 8.0, 100.0), "Face to face"), field("round", "Rounding", len(1.5, 0.0, 10.0), "Edges"), field("depth", "Marking depth", len(0.8, 0.2, 5.0), "Pips and numbers"), FONT, field("detail", "Detail", len(0.3, 0.1, 2.0), "Smaller is finer and slower")];
    "make.marble", "Marble Track", "HARDWARE & HOME", "pipe", false, hardware::marble,
    "Marble run pieces that join end to end: straight, curve and support",
    [AT, field("kind", "Piece", choice("straight", &["straight", "curve", "support"]), "Which"), field("marble", "Marble", len(16.0, 6.0, 60.0), "Diameter"), field("length", "Length", len(100.0, 20.0, 400.0), "Straight pieces and support height"), field("radius", "Curve radius", len(60.0, 25.0, 300.0), "To the middle"), field("angle", "Curve angle", num(90.0, 15.0, 180.0, "°"), "How far around"), field("wall", "Wall", len(2.4, 1.0, 10.0), "Thickness"), field("clearance", "Clearance", len(0.3, 0.0, 1.0), "In the joints")];
    "make.stand", "Device Stand", "HARDWARE & HOME", "rest", false, hardware::stand,
    "A phone or tablet stand with a cable notch",
    [AT, field("width", "Width", len(70.0, 20.0, 300.0), "Across"), field("device", "Device thickness", len(12.0, 4.0, 40.0), "With its case"), field("height", "Back height", len(80.0, 30.0, 250.0), "Support"), field("angle", "Lean", num(20.0, 5.0, 45.0, "°"), "From upright"), field("thickness", "Thickness", len(5.0, 2.0, 20.0), "Of the stand"), field("cable", "Cable notch", len(14.0, 0.0, 40.0), "0: none")];

    "make.add_text", "Text on Model", "EDIT MODEL", "emboss", false, edit::add_text,
    "Raise or engrave text on a face of any model, solid or STL",
    [BODY, POINT, field("text", "Text", FieldKind::Text("Text"), "What it says"), FONT, field("size", "Size", len(10.0, 1.0, 300.0), "Capital height"), field("depth", "Depth", len(1.0, 0.1, 50.0), "Up or in"), field("mode", "Mode", choice("raise", &["raise", "engrave"]), "Which way"), field("angle", "Turn", num(0.0, -180.0, 180.0, "°"), "On the face"),
     field("separate", "Separate body", FieldKind::Bool(false), "Keep raised letters as their own body, for another colour"), DETAIL];
    "make.texture", "Texture Model", "EDIT MODEL", "p3_lattice", false, edit::texture,
    "Give a model's whole surface a printable texture",
    [BODY, field("pattern", "Texture", choice("knurl", &["knurl", "waves", "bumps", "gyroid", "scales", "layers", "rock"]), "Which"), field("scale", "Scale", len(5.0, 0.5, 100.0), "Size of the pattern"), field("depth", "Depth", len(0.8, 0.05, 20.0), "In and out"), field("keep_bottom", "Smooth bottom", len(1.0, 0.0, 100.0), "Left plain, up from the lowest point"), DETAIL];
    "make.split", "Split Model", "EDIT MODEL", "split_body", false, edit::split,
    "Cut any model in two with matching pegs and holes",
    [BODY, field("axis", "Across", choice("Z", &["X", "Y", "Z"]), "Direction the cut faces"), field("position", "Position", num(50.0, 1.0, 99.0, "%"), "Along the model"), field("pegs", "Pegs", int(2, 0, 4), "0: a plain cut"), field("peg", "Peg diameter", len(5.0, 1.0, 50.0), "Of each"), field("peg_length", "Peg length", len(5.0, 1.0, 50.0), "Into the other half"), field("clearance", "Clearance", len(0.25, 0.0, 2.0), "Around pegs"), field("apart", "Move apart", FieldKind::Bool(true), "Show the halves side by side"), DETAIL];
    "make.flexi", "Flexi Joints", "EDIT MODEL", "chain", false, edit::flexi,
    "Cut a model into links joined by captured balls, so it prints in one piece and wiggles",
    [BODY, field("axis", "Along", choice("X", &["X", "Y", "Z"]), "The model's length"), field("joints", "Joints", int(4, 1, 20), "How many"), field("gap", "Gap", len(0.6, 0.2, 5.0), "Between links"), field("clearance", "Ball clearance", len(0.4, 0.2, 2.0), "In the sockets"), field("ball", "Ball", num(50.0, 20.0, 80.0, "%"), "Of the model's thickness there"), DETAIL];
    "make.cavity", "Holder Cavity", "EDIT MODEL", "hole", false, edit::cavity,
    "Sink a can, bottle, candle, pot or pen hole into the top of any model",
    [BODY, field("preset", "For", choice("can", &["can", "slim can", "tall can", "tealight", "pen", "pint tub", "custom"]), "Sizes with room to slide"), field("diameter", "Diameter", len(67.5, 1.0, 500.0), "For custom"), field("depth", "Depth", len(90.0, 1.0, 500.0), "For custom; the floor is kept"), field("floor", "Floor", len(3.0, 0.0, 50.0), "Left below"), DETAIL];
    "make.mold", "Mould", "EDIT MODEL", "boundary", false, edit::mold,
    "A two-part mould around a model with keys and a pouring hole",
    [BODY, field("wall", "Wall", len(8.0, 3.0, 50.0), "Around the model"), field("axis", "Parting", choice("Y", &["X", "Y", "Z"]), "Direction the halves open"), field("pour", "Pouring hole", len(10.0, 0.0, 60.0), "From above; 0: none"), field("key", "Keys", len(6.0, 0.0, 30.0), "Diameter; 0: none"), field("clearance", "Clearance", len(0.25, 0.0, 2.0), "Around keys"), DETAIL];
    "make.hollow", "Hollow Model", "EDIT MODEL", "shell", false, edit::hollow,
    "Hollow any model to a wall, with an opening on top or a drain below",
    [BODY, field("wall", "Wall", len(2.4, 0.4, 50.0), "Left all round"), field("open", "Opening", choice("none", &["none", "top", "bottom"]), "Where"), field("diameter", "Opening diameter", len(20.0, 1.0, 500.0), "Of the hole"), DETAIL];
    "make.puzzle", "Puzzle Pieces", "EDIT MODEL", "pattern_rect", false, edit::puzzle,
    "Cut a model into blocks that plug together",
    [BODY, field("nx", "Across X", int(2, 1, 5), "Pieces"), field("ny", "Across Y", int(2, 1, 5), "Pieces"), field("nz", "Up Z", int(2, 1, 5), "Pieces"), field("peg", "Peg diameter", len(4.0, 0.0, 30.0), "0: none"), field("clearance", "Clearance", len(0.2, 0.0, 2.0), "Between pieces"), DETAIL];
    "make.flat_base", "Flat Base", "EDIT MODEL", "plane", false, edit::flat_base,
    "Cut a model's underside flat, and set it on the bed",
    [BODY, field("height", "Cut height", len(1.0, 0.05, 500.0), "Above the lowest point"), DETAIL];
    "make.resize", "Resize to Fit", "EDIT MODEL", "scale", false, edit::resize,
    "Scale a model to an exact size along one axis",
    [BODY, field("axis", "Axis", choice("Z", &["X", "Y", "Z"]), "Measured along"), field("size", "Size", len(100.0, 0.1, 3000.0), "Wanted"), field("even", "Keep proportions", FieldKind::Bool(true), "Scale all axes alike")];
    "make.loop", "Hanging Loop", "EDIT MODEL", "torus", false, edit::ring,
    "Add a keyring or hanging loop to the top of a model",
    [BODY, field("diameter", "Inside diameter", len(5.0, 1.0, 100.0), "Of the loop"), field("thickness", "Thickness", len(2.0, 0.6, 30.0), "Of the ring"), field("facing", "Facing", choice("Y", &["X", "Y"]), "The way the hole looks")];
    "make.fuse", "Fuse Models", "EDIT MODEL", "combine", false, edit::fuse,
    "Fuse several models, solids or STLs, into one clean watertight mesh",
    [BODIES, field("blend", "Blend", len(0.0, 0.0, 20.0), "Round the joins by this much"), DETAIL];
    "make.cut_model", "Cut with Model", "EDIT MODEL", "cut", false, edit::cut_model,
    "Cut one model out of another: select the one to keep first, then the cutter",
    [BODIES, field("clearance", "Clearance", len(0.0, 0.0, 10.0), "Extra room around the cutter"), field("keep", "Keep cutter", FieldKind::Bool(true), "Leave the cutter in the design"), DETAIL];
}

// ---- shared ------------------------------------------------------------------------------------

const UP: Vec3 = Vec3::new(0.0, 0.0, 1.0);

fn v(x: f64, y: f64, z: f64) -> Vec3 {
    Vec3::new(x, y, z)
}

fn made(summary: String, bodies: &[String]) -> Result<Value> {
    Ok(json!({"summary": summary, "created_bodies": bodies}))
}

/// The words of a text field; `\n` breaks lines.
fn words(a: &Args, key: &str) -> Result<String> {
    let t = a.text(key).replace("\\n", "\n");
    if t.trim().is_empty() {
        return Err(a.fail(format!("give `{key}` some text")));
    }
    if t.chars().count() > 400 {
        return Err(a.fail("that is too much text"));
    }
    Ok(t)
}

/// Lettering drawn centred on `flat`, as a solid.
fn lettering(s: &mut Session, a: &Args, flat: Flat, text: &str, size: f64, depth: f64, what: Do) -> Result<String> {
    let lines = text.lines().count().max(1) as f64;
    let mut p = json!({"text": text, "at": [0.0, size * (0.75 * (lines - 1.0) - 0.5)], "height": size, "align": "center"});
    let font = a.text("font").trim();
    if !font.is_empty()
        && let Some(o) = p.as_object_mut()
    {
        o.insert("font".into(), json!(font));
    }
    sketched(s, flat, depth, 0.0, what, |s| s.execute("sketch.text", &p).map(|_| ()))
}

/// The bodies a command makes besides the first: `lettering` with `Do::New` can make one per letter.
fn new_since(s: &Session, before: &[String]) -> Result<Vec<String>> {
    Ok(s.world_state()?.bodies.iter().map(|b| b.name.clone()).filter(|n| !before.contains(n)).collect())
}

fn names(s: &Session) -> Result<Vec<String>> {
    Ok(s.world_state()?.bodies.iter().map(|b| b.name.clone()).collect())
}

/// Lettering as bodies of its own, and the box around it.
fn letters(s: &mut Session, a: &Args, flat: Flat, text: &str, size: f64, depth: f64, name: &str) -> Result<(Vec<String>, rybocraft_geom::Aabb3)> {
    let before = names(s)?;
    lettering(s, a, flat, text, size, depth, Do::New(name))?;
    let new = new_since(s, &before)?;
    let b = mesh_of(s, &new)?.bounds();
    Ok((new, b))
}

/// Lettering as a mesh, made in a copy of the design so nothing is left behind.
fn letter_mesh(s: &Session, a: &Args, text: &str, size: f64, depth: f64) -> Result<PrintMesh> {
    // Small letters are drawn large: their outlines are too fine for the kernel as they are.
    let k = (20.0 / size).max(1.0);
    let mut t = s.scratch();
    let (bodies, _) = letters(&mut t, a, Flat::level(v(0., 0., 0.)), text, size * k, depth * k, "Text")?;
    let mut m = mesh_of(&t, &bodies)?;
    for p in &mut m.positions {
        *p = *p * (1.0 / k);
    }
    Ok(m)
}

fn remove(s: &mut Session, bodies: &[String]) -> Result<()> {
    if bodies.is_empty() {
        return Ok(());
    }
    s.execute("solid.remove", &json!({"bodies": bodies})).map(|_| ())
}

fn is_mesh(s: &Session, body: &str) -> Result<bool> {
    Ok(s.world_state()?.body(body).is_some_and(|b| b.body.is_mesh()))
}

/// The detail to mesh at: what was asked, or as fine as the size allows.
fn detail(a: &Args, b: &rybocraft_geom::Aabb3, warnings: &mut Vec<String>) -> f64 {
    let want = a.num("detail");
    let s = b.size();
    let least = ((s.x + 4.0 * want) * (s.y + 4.0 * want) * (s.z + 4.0 * want) / 12_000_000.0).cbrt();
    if least > want {
        warnings.push(format!("detail raised to {least:.2} mm for a model this large"));
    }
    want.max(least)
}

fn mesh_made(s: &mut Session, name: &str, m: &PrintMesh, summary: String, warnings: Vec<String>) -> Result<Value> {
    let bodies = add_mesh_body(s, name, m)?;
    Ok(json!({"summary": summary, "created_bodies": bodies, "warnings": warnings}))
}

/// Numbers separated by commas.
fn list(text: &str) -> Vec<f64> {
    text.split([',', ';', ' ']).filter_map(|t| t.trim().parse::<f64>().ok()).filter(|x| x.is_finite() && *x > 0.0).collect()
}

#[cfg(test)]
mod tests;
