//! The window every `print.*` and `make.*` tool opens in. The form comes from the tool's own
//! field table (`rybocraft_engine::cmd::print`), so a new tool needs no UI code; the result is
//! shown below it, and analyses paint their findings on the model.

use std::collections::BTreeMap;

use egui::{Color32, RichText, Stroke};
use serde_json::{Map, Value, json};
use rybocraft_engine::cmd::maker;
use rybocraft_engine::cmd::print::{Field, FieldKind, OverlayRole, PANELS, TOOLS, Tool, setup, tool};

use std::cell::RefCell;
use std::rc::Rc;

/// The tools of the tab `t` is on, and its panels.
fn family(t: &Tool) -> (&'static [Tool], &'static [&'static str], &'static str) {
    if t.id.starts_with("make.") { (maker::TOOLS, &maker::PANELS, "Maker Studio") } else { (TOOLS, &PANELS, "3D Print Studio") }
}

use crate::SolveApp;
use crate::ui_kit;
use rybocraft_theme::Tokens;

/// The window's state.
pub struct PrintStudio {
    tool: &'static Tool,
    /// What each tool's fields hold, as typed, by tool and key.
    values: BTreeMap<(&'static str, &'static str), Held>,
    /// The last result, or what went wrong.
    result: Option<Result<Value, String>>,
    /// Remake the part as its values change.
    live: bool,
    /// The undo depth right after the part on show was made, and the values it was made with.
    shown: Option<(usize, u64, Value)>,
    /// The values seen last frame and when they last changed: a drag settles before remaking.
    seen: Option<(Value, f64)>,
    /// A file picked for a field, waiting to be taken.
    picked: Rc<RefCell<Option<(&'static str, String)>>>,
}

#[derive(Clone, Debug)]
enum Held {
    Text(String),
    Bool(bool),
    Choice(usize),
    Position([String; 3]),
}

/// The printer and filament lists, for the setup form.
fn choices(key: &str) -> Vec<&'static str> {
    use rybocraft_engine::cmd::print::{filaments, printers};
    match key {
        "printer" => printers(),
        "material" => filaments(),
        _ => Vec::new(),
    }
}

fn plain(x: f64) -> String {
    format!("{x}")
}

impl PrintStudio {
    fn held(&mut self, f: &'static Field) -> &mut Held {
        self.values.entry((self.tool.id, f.key)).or_insert_with(|| match f.kind {
            FieldKind::Length { default, .. } | FieldKind::Number { default, .. } => Held::Text(plain(default)),
            FieldKind::Int { default, .. } => Held::Text(default.to_string()),
            FieldKind::Bool(b) => Held::Bool(b),
            FieldKind::Choice { default, options } => Held::Choice(options.iter().position(|o| *o == default).unwrap_or(0)),
            FieldKind::Position(d) => Held::Position(d.map(plain)),
            FieldKind::Text(t) => {
                let p = setup();
                let current = match f.key {
                    "printer" => p.printer,
                    "material" => p.material,
                    _ => t.to_string(),
                };
                Held::Text(current)
            }
            FieldKind::Bodies | FieldKind::Body | FieldKind::Point | FieldKind::Font | FieldKind::File(_) => Held::Text(String::new()),
        })
    }

    /// The call the form makes.
    fn params(&mut self) -> Result<Value, String> {
        let mut out = Map::new();
        for f in self.tool.fields {
            let held = self.held(f).clone();
            let v = match (f.kind, held) {
                (FieldKind::Bodies | FieldKind::Body | FieldKind::Point, _) => continue,
                (_, Held::Bool(b)) => json!(b),
                (FieldKind::Choice { options, .. }, Held::Choice(i)) => json!(options.get(i)),
                (FieldKind::Length { .. }, Held::Text(t)) => match t.trim().parse::<f64>() {
                    Ok(x) => json!(x),
                    Err(_) => json!(t),
                },
                (FieldKind::Number { .. } | FieldKind::Int { .. }, Held::Text(t)) => {
                    let x: f64 = t.trim().parse().map_err(|_| format!("{}: `{t}` is not a number", f.label))?;
                    if x.fract() == 0.0 && x.abs() < 1e15 { json!(x as i64) } else { json!(x) }
                }
                (FieldKind::Position(_), Held::Position(p)) => {
                    let mut at = [0.0; 3];
                    for (slot, t) in at.iter_mut().zip(&p) {
                        *slot = t.trim().parse().map_err(|_| format!("{}: `{t}` is not a number", f.label))?;
                    }
                    json!(at)
                }
                (_, Held::Text(t)) => json!(t),
                _ => continue,
            };
            out.insert(f.key.into(), v);
        }
        Ok(Value::Object(out))
    }
}

/// Open the window on a tool; true when `id` is one.
pub(crate) fn open(app: &mut SolveApp, id: &str) -> bool {
    let Some(t) = tool(id) else { return false };
    if id == "print.clear_overlay" {
        app.session.print_overlay = None;
        return true;
    }
    match &mut app.print_studio {
        Some(s) => {
            s.tool = t;
            s.result = None;
            s.shown = None;
            s.seen = None;
        }
        None => {
            app.print_studio =
                Some(PrintStudio { tool: t, values: BTreeMap::new(), result: None, live: true, shown: None, seen: None, picked: Rc::default() });
        }
    }
    true
}

/// What the tool would work on now, from the selection.
fn target(app: &SolveApp, kind: FieldKind) -> String {
    use rybocraft_engine::Sel;
    let mut bodies: Vec<&str> = Vec::new();
    let mut face = None;
    for x in &app.session.selection {
        match x {
            Sel::Body { name } => bodies.push(name),
            Sel::Face { body, point, .. } => {
                bodies.push(body);
                face.get_or_insert((body.as_str(), *point));
            }
            _ => {}
        }
    }
    bodies.dedup();
    match kind {
        FieldKind::Point => face.map_or_else(
            || "Click a flat face in the model".to_string(),
            |(b, p)| format!("{b} at {:.1}, {:.1}, {:.1}", p.x, p.y, p.z),
        ),
        FieldKind::Body => match bodies.as_slice() {
            [] => "The only visible body".into(),
            [one] => (*one).to_string(),
            _ => "Select one body".into(),
        },
        _ if bodies.is_empty() => "All visible bodies".into(),
        _ => bodies.join(", "),
    }
}

fn form(app: &mut SolveApp, ui: &mut egui::Ui, d: &mut PrintStudio) -> bool {
    let mut enter = false;
    let t = Tokens::get();
    let (last, first): (Vec<&'static Field>, Vec<_>) = d.tool.fields.iter().partition(|f| matches!(f.kind, FieldKind::Position(_)));
    for f in first.into_iter().chain(last) {
        if matches!(f.kind, FieldKind::Body) && d.tool.fields.iter().any(|f| matches!(f.kind, FieldKind::Point)) {
            // The picked face says which body.
            continue;
        }
        let id = format!("print_{}_{}", d.tool.id, f.key);
        let held = d.held(f);
        match held {
            Held::Bool(b) => {
                ui_kit::checkbox(ui, f.label, b).on_hover_text(f.help);
            }
            Held::Choice(i) => {
                let FieldKind::Choice { options, .. } = f.kind else { continue };
                let shown: Vec<String> = options.iter().map(|o| crate::workspace::word_cased(&o.replace('_', " "))).collect();
                // Thread sizes and the like are names, not words.
                let names: Vec<&str> = if f.key == "size" { options.to_vec() } else { shown.iter().map(String::as_str).collect() };
                ui_kit::row(ui, f.label, |ui| ui_kit::dropdown(ui, &id, i, &names).on_hover_text(f.help));
            }
            Held::Position(p) => {
                for (axis, value) in ["X", "Y", "Z"].into_iter().zip(p.iter_mut()) {
                    enter |= ui_kit::text_row(ui, &format!("{} {axis}", f.label), value, "mm").1;
                }
            }
            Held::Text(value) => match f.kind {
                FieldKind::Bodies | FieldKind::Body | FieldKind::Point => {
                    let text = target(app, f.kind);
                    ui_kit::row(ui, f.label, |ui| ui.label(RichText::new(text).color(t.accent)).on_hover_text(f.help));
                }
                FieldKind::Font => {
                    ui_kit::row(ui, f.label, |ui| ui_kit::text_field(ui, value, "Built-in"));
                }
                FieldKind::File(types) => {
                    let mut browse = false;
                    enter |= ui_kit::text_row(ui, f.label, value, "Path").1;
                    ui_kit::row(ui, "", |ui| browse = ui_kit::button(ui, "Browse…", ui_kit::Button::Normal).clicked());
                    if browse {
                        let (slot, key) = (d.picked.clone(), f.key);
                        app.pick_open_for(types, move |_, path| *slot.borrow_mut() = path.map(|p| (key, p)));
                    }
                }
                FieldKind::Text(_) if choices(f.key).is_empty() => enter |= ui_kit::text_row(ui, f.label, value, "").1,
                FieldKind::Text(_) => {
                    let known = choices(f.key);
                    let mut i = known.iter().position(|k| k == value).unwrap_or(usize::MAX);
                    ui_kit::row(ui, f.label, |ui| ui_kit::dropdown(ui, &id, &mut i, &known).on_hover_text(f.help));
                    if let Some(k) = known.get(i) {
                        *value = (*k).to_string();
                    }
                }
                FieldKind::Number { default, min, max, unit } => enter |= slide(ui, f, value, unit, default, min, max, false),
                FieldKind::Length { default, min, max } => enter |= slide(ui, f, value, "mm", default, min, max, false),
                FieldKind::Int { default, min, max } => enter |= slide(ui, f, value, "", default as f64, min as f64, max as f64, true),
                _ => enter |= ui_kit::text_row(ui, f.label, value, "").1,
            },
        }
    }
    enter
}

/// A value typed or dragged. The slider covers the useful part of a wide range; typing reaches the rest.
fn slide(ui: &mut egui::Ui, f: &Field, value: &mut String, unit: &str, default: f64, min: f64, max: f64, whole: bool) -> bool {
    let enter = ui_kit::text_row(ui, f.label, value, unit).1;
    if min < 0.0 && default == min {
        // -1 means "as set up": nothing to slide.
        return enter;
    }
    let top = if default > 0.0 { max.min((default * 3.0).max(min + 10.0)) } else { max.min(min + 50.0) };
    if let Ok(mut x) = value.trim().parse::<f64>() {
        let was = x;
        let step = if whole { 1.0 } else if top - min > 20.0 { 0.5 } else { 0.05 };
        let slider = egui::Slider::new(&mut x, min..=top.max(was)).step_by(step).show_value(false).trailing_fill(true);
        ui.spacing_mut().slider_width = (ui.available_width() - 8.0).max(60.0);
        if ui.add(slider).on_hover_text(f.help).changed() && x != was {
            *value = plain((x * 1000.0).round() / 1000.0);
        }
    }
    enter
}

/// `mass_g` → "Mass g".
fn key_label(k: &str) -> String {
    let text = k.replace('_', " ");
    let mut c = text.chars();
    c.next().map(|f| f.to_uppercase().collect::<String>() + c.as_str()).unwrap_or_default()
}

fn value_text(v: &Value) -> String {
    match v {
        Value::String(t) => t.clone(),
        Value::Bool(b) => if *b { "yes" } else { "no" }.into(),
        Value::Number(n) => n.as_f64().map_or_else(|| n.to_string(), |x| plain((x * 1000.0).round() / 1000.0)),
        Value::Array(a) => a.iter().map(value_text).collect::<Vec<_>>().join(", "),
        Value::Object(o) => o.iter().map(|(k, v)| format!("{} {}", key_label(k), value_text(v))).collect::<Vec<_>>().join(" · "),
        Value::Null => "–".into(),
    }
}

fn results(ui: &mut egui::Ui, r: &Result<Value, String>) {
    let t = Tokens::get();
    let v = match r {
        Err(e) => {
            ui.label(RichText::new(e).color(t.error));
            return;
        }
        Ok(v) => v,
    };
    if let Some(summary) = v["summary"].as_str() {
        ui.label(RichText::new(summary).strong());
    }
    for w in v["warnings"].as_array().into_iter().flatten() {
        ui.label(RichText::new(format!("⚠ {}", value_text(w))).color(t.warning));
    }
    for i in v["issues"].as_array().into_iter().flatten() {
        let color = match i["level"].as_str() {
            Some("error") => t.error,
            Some("warning") => t.warning,
            _ => t.text_dim,
        };
        ui.label(RichText::new(format!("● {}", value_text(&i["issue"]))).color(color));
        if let Some(fix) = i["fix"].as_str().filter(|f| !f.is_empty()) {
            ui.indent("fix", |ui| ui.label(RichText::new(fix).small().color(t.text_dim)));
        }
    }
    const SKIP: [&str; 8] = ["summary", "warnings", "issues", "printers", "filaments", "param_warnings", "setup", "body"];
    let rows: Vec<_> = v.as_object().into_iter().flatten().filter(|(k, _)| !SKIP.contains(&k.as_str())).collect();
    if rows.is_empty() {
        return;
    }
    ui.add_space(4.0);
    for (k, v) in rows {
        match v {
            Value::Array(a) if a.iter().any(Value::is_object) => {
                ui.label(RichText::new(key_label(k)).color(t.text_dim));
                for item in a.iter().take(40) {
                    ui.label(value_text(item));
                }
            }
            _ => {
                ui.horizontal_wrapped(|ui| {
                    ui.label(RichText::new(key_label(k)).color(t.text_dim));
                    ui.label(value_text(v));
                });
            }
        }
    }
}

/// Whether the tool makes a new part, rather than changing or checking one.
fn makes(t: &Tool) -> bool {
    t.fields.iter().any(|f| matches!(f.kind, FieldKind::Position(_)))
}

/// Draw the window when it is open. Called once per frame.
pub(crate) fn show(app: &mut SolveApp, ctx: &egui::Context) {
    let Some(mut d) = app.print_studio.take() else { return };
    let t = Tokens::get();
    let (mut go, mut next) = (false, None);
    // At the right of the view, clear of the model's middle and of the view compass.
    let view = crate::shell::content_rect(app, app.viewport.rect.unwrap_or_else(|| ctx.content_rect()));
    let corner = view.right_top() + egui::vec2(-92.0, 12.0);
    let tall = (view.height() - 110.0).clamp(160.0, 620.0);
    let closed = ui_kit::Window::new(egui::Id::unique("rc_print_studio"), "print_studio", family(d.tool).2).placed(egui::Align2::RIGHT_TOP, corner).cards(1.3).show_body(ctx, |ui| {
        egui::ScrollArea::vertical().max_height(tall).show(ui, |ui| {
            let (tools_all, panels, _) = family(d.tool);
            let groups: Vec<&str> = panels.to_vec();
            let mut group = groups.iter().position(|p| *p == d.tool.panel).unwrap_or(0);
            let names: Vec<String> = groups.iter().map(|p| crate::workspace::word_cased(p)).collect();
            let shown: Vec<&str> = names.iter().map(String::as_str).collect();
            let before = group;
            ui_kit::row(ui, "Group", |ui| ui_kit::dropdown(ui, "print_group", &mut group, &shown));
            let panel = groups.get(group).copied().unwrap_or(d.tool.panel);
            let tools: Vec<&'static Tool> = tools_all.iter().filter(|t| t.panel == panel && t.id != "print.clear_overlay").collect();
            let mut at = tools.iter().position(|t| t.id == d.tool.id).unwrap_or(0);
            let labels: Vec<&str> = tools.iter().map(|t| t.label).collect();
            ui_kit::row(ui, "Tool", |ui| ui_kit::dropdown(ui, "print_tool", &mut at, &labels));
            if let Some(t) = tools.get(at)
                && (t.id != d.tool.id || before != group)
            {
                next = Some(*t);
            }
            ui.label(RichText::new(d.tool.help).small().color(t.text_dim));
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                let label = match d.tool.panel {
                    "ANALYZE" => "Analyze",
                    "PRINTER" => "Save",
                    _ if makes(d.tool) => "Make",
                    _ => "Apply",
                };
                go |= ui_kit::primary_button(ui, label).clicked();
                if makes(d.tool) {
                    ui_kit::checkbox(ui, "Live", &mut d.live).on_hover_text("Remake the part as you change it");
                }
                if app.session.print_overlay.is_some() && ui_kit::button(ui, "Clear highlights", ui_kit::Button::Normal).clicked() {
                    app.session.print_overlay = None;
                }
            });
            if makes(d.tool) {
                // One line, always there, so nothing moves under the pointer while a value is dragged.
                let (text, color) = match &d.result {
                    None => (String::new(), t.text_dim),
                    Some(Ok(v)) => (value_text(&v["summary"]), t.text_dim),
                    Some(Err(e)) => (e.clone(), t.error),
                };
                ui.add(egui::Label::new(RichText::new(if text.is_empty() { " " } else { &text }).color(color)).truncate()).on_hover_text(&text);
            } else if let Some(r) = &d.result {
                results(ui, r);
            }
            ui.separator();
            go |= form(app, ui, &mut d);
            if makes(d.tool)
                && let Some(Ok(v)) = &d.result
            {
                for w in v["warnings"].as_array().into_iter().flatten() {
                    ui.label(RichText::new(format!("⚠ {}", value_text(w))).color(t.warning));
                }
            }
        });
    });
    let taken = d.picked.borrow_mut().take();
    if let Some((key, path)) = taken {
        d.values.insert((d.tool.id, key), Held::Text(path));
    }
    let now = ctx.input(|i| i.time);
    let held_down = ctx.input(|i| i.pointer.any_down());
    if let Some(t) = next {
        d.tool = t;
        d.result = None;
        d.shown = None;
        d.seen = None;
    } else if !go
        && d.live
        && makes(d.tool)
        && let Ok(p) = d.params()
    {
        if d.seen.as_ref().is_none_or(|(q, _)| *q != p) {
            d.seen = Some((p.clone(), now));
            ctx.request_repaint_after(std::time::Duration::from_millis(350));
        }
        let settled = d.seen.as_ref().is_some_and(|(_, at)| now - at > 0.3) && !held_down;
        // Only the part this window made last, and only while nothing was done since.
        let ours = d.shown.as_ref().is_some_and(|(depth, revision, q)| {
            *q != p && app.session.undo.len() == *depth && app.session.revision == *revision
        });
        if ours && settled {
            let id = d.tool.id;
            if app.run("edit.undo", json!({})).is_ok() {
                let r = app.run(id, p.clone());
                if r.is_err() {
                    // Values that make nothing leave the last part standing.
                    let _ = app.run("edit.redo", json!({}));
                }
                d.shown = Some((app.session.undo.len(), app.session.revision, p));
                d.result = Some(r);
            }
        }
    }
    if go && next.is_none() {
        let id = d.tool.id;
        d.result = Some(match d.params() {
            Ok(p) => {
                let r = app.run(id, p.clone());
                if r.is_ok() && makes(d.tool) {
                    d.shown = Some((app.session.undo.len(), app.session.revision, p));
                    app.start("view.fit");
                }
                r
            }
            Err(e) => Err(e),
        });
    }
    if !closed && app.print_studio.is_none() {
        app.print_studio = Some(d);
    }
}

/// Paint what the last analysis found.
pub(crate) fn overlay(app: &SolveApp, painter: &egui::Painter, proj: &crate::viewport::Proj) {
    use rybocraft_engine::geom::prelude::*;
    let Some(o) = app.session.print_overlay.as_ref().filter(|o| o.revision == app.session.revision) else { return };
    let color = |r: OverlayRole| match r {
        OverlayRole::Overhang => Color32::from_rgb(235, 90, 40),
        OverlayRole::Thin => Color32::from_rgb(220, 40, 130),
        OverlayRole::Island => Color32::from_rgb(250, 200, 20),
        OverlayRole::Layer => Color32::from_rgb(30, 190, 120),
    };
    let mut mesh = egui::Mesh::default();
    for (role, [a, b, c]) in &o.triangles {
        let (Some(pa), Some(pb), Some(pc)) = (proj.to_screen(*a), proj.to_screen(*b), proj.to_screen(*c)) else { continue };
        // Faces looking away show through faintly, so an underside can be found from above.
        let mid = (*a + *b + *c) * (1.0 / 3.0);
        let front = (*b - *a).cross(*c - *a).dot(proj.cam.eye() - mid) > 0.0;
        let fill = color(*role).gamma_multiply(if front { 0.6 } else { 0.22 });
        let i = mesh.vertices.len() as u32;
        for p in [pa, pb, pc] {
            mesh.colored_vertex(p, fill);
        }
        mesh.add_triangle(i, i + 1, i + 2);
    }
    painter.add(egui::Shape::mesh(mesh));
    for (role, [a, b]) in &o.lines {
        if let (Some(a), Some(b)) = (proj.to_screen(*a), proj.to_screen(*b)) {
            painter.line_segment([a, b], Stroke::new(2.0, color(*role)));
        }
    }
    for (role, p) in &o.points {
        if let Some(p) = proj.to_screen(*p) {
            painter.circle(p, 6.0, color(*role).gamma_multiply(0.7), Stroke::new(1.5, Color32::BLACK));
        }
    }
    let at = proj.rect.center_bottom() - egui::vec2(0.0, 150.0);
    let t = Tokens::get();
    let text = painter.layout_no_wrap(o.label.clone(), egui::FontId::proportional(t.font_label), Color32::WHITE);
    let r = egui::Rect::from_center_size(at, text.size() + egui::vec2(18.0, 8.0));
    painter.rect_filled(r, 6.0, Color32::from_black_alpha(190));
    painter.galley(r.center() - text.size() * 0.5, text, Color32::WHITE);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_tool_opens_and_its_defaults_make_a_valid_call() {
        let mut app = SolveApp::new(rybocraft_engine::Session::default(), crate::Services::default());
        for t in TOOLS.iter().chain(maker::TOOLS).filter(|t| t.id != "print.clear_overlay") {
            app.start(t.id);
            let d = app.print_studio.as_mut().unwrap();
            assert_eq!(d.tool.id, t.id);
            let p = d.params().unwrap();
            let contract = rybocraft_engine::find_command(t.id).and_then(|c| c.contract).unwrap();
            assert!(contract.check(&p).is_empty(), "{}: {p} {:?}", t.id, contract.check(&p));
        }
    }

    #[test]
    fn the_ribbon_has_every_tool_on_the_3d_print_tab() {
        assert!(crate::workspace::TABS.contains(&"3D PRINT"));
        for t in TOOLS {
            let c = rybocraft_engine::find_command(t.id).unwrap();
            assert_eq!(crate::workspace::destination(c), Some(("3D PRINT", t.panel)), "{}", t.id);
        }
    }

    #[test]
    fn the_window_draws_and_runs_a_tool() {
        let mut h = crate::scenario::Harness::new();
        h.app.start("print.calibration");
        h.frames(2);
        h.step(&json!({"widget": "Make"})).unwrap();
        h.frames(2);
        assert_eq!(h.app.session.model.state().bodies.len(), 1);
        h.app.start("print.check");
        h.frames(2);
        h.step(&json!({"widget": "Analyze"})).unwrap();
        h.frames(2);
        assert!(matches!(&h.app.print_studio.as_ref().unwrap().result, Some(Ok(v)) if v["score"].as_f64() > Some(80.0)));
        assert!(h.app.session.print_overlay.is_some());
    }
}
