//! Finding a printer's clearance.

use serde_json::{Value, json};
use rybocore_record::Vec3;

use super::*;

pub(super) fn fit_test(s: &mut Session, a: &Args) -> Result<Value> {
    let o = a.at();
    let (d, n) = (a.num("diameter"), a.int("count"));
    let up = Vec3::new(0.0, 0.0, 1.0);
    let pitch = d + 6.0;
    let length = pitch * n as f64 + 4.0;
    let strip = cuboid(s, o, [length, d + 8.0, 6.0], Do::New("Fit strip"))?;
    let mut holes = Vec::new();
    for i in 0..n {
        let c = a.num("from") + a.num("step") * i as f64;
        let x = 2.0 + pitch * (i as f64 + 0.5);
        cylinder(s, o + Vec3::new(x, d * 0.5 + 4.0, -0.05), up, d + c, 6.1, Do::Cut(&strip))?;
        holes.push((c * 1000.0).round() / 1000.0);
    }
    // A notch marks the tight end.
    cuboid(s, o + Vec3::new(-0.1, -0.1, -0.1), [2.1, 2.1, 6.2], Do::Cut(&strip))?;
    let pin = cylinder(s, o + Vec3::new(length + d + 6.0, d * 0.5 + 4.0, 0.0), up, d, 16.0, Do::New("Fit pin"))?;
    Ok(json!({"summary": format!("{n} holes for a {d} mm pin; the notched end is the tightest"), "clearances_mm": holes, "created_bodies": [strip, pin]}))
}
