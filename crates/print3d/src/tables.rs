//! Printers, filaments and fastener fits.

use serde::Serialize;

/// A printer's build volume, in mm.
#[derive(Clone, Copy, Debug, Serialize)]
pub struct Printer {
    pub name: &'static str,
    pub bed: [f64; 3],
    pub nozzle: f64,
    /// mm/s for walls and infill.
    pub speed: [f64; 2],
    /// mm³/s.
    pub max_flow: f64,
}

const fn printer(name: &'static str, x: f64, y: f64, z: f64, wall: f64, infill: f64, max_flow: f64) -> Printer {
    Printer { name, bed: [x, y, z], nozzle: 0.4, speed: [wall, infill], max_flow }
}

/// Nominal build volumes. Speeds are working values for the estimate, not maker's limits.
pub const PRINTERS: &[Printer] = &[
    printer("Generic 220", 220., 220., 250., 50., 70., 10.),
    printer("Generic 300", 300., 300., 400., 50., 70., 10.),
    printer("Bambu Lab A1 mini", 180., 180., 180., 180., 250., 20.),
    printer("Bambu Lab A1", 256., 256., 256., 180., 250., 22.),
    printer("Bambu Lab P1S / X1C", 256., 256., 256., 200., 270., 24.),
    printer("Bambu Lab H2D", 325., 320., 325., 200., 300., 28.),
    printer("Prusa MK4S", 250., 210., 220., 150., 200., 22.),
    printer("Prusa CORE One", 250., 220., 270., 170., 220., 24.),
    printer("Prusa MINI+", 180., 180., 180., 60., 80., 10.),
    printer("Prusa XL", 360., 360., 360., 130., 180., 22.),
    printer("Creality Ender 3", 220., 220., 250., 50., 60., 9.),
    printer("Creality K1", 220., 220., 250., 200., 280., 24.),
    printer("Creality K2 Plus", 350., 350., 350., 200., 280., 26.),
    printer("Elegoo Neptune 4 Pro", 225., 225., 265., 150., 220., 18.),
    printer("Elegoo Centauri Carbon", 256., 256., 256., 200., 270., 24.),
    printer("Anycubic Kobra 3", 250., 250., 260., 150., 220., 18.),
    printer("Sovol SV08", 350., 350., 345., 200., 280., 26.),
    printer("Voron 2.4 350", 350., 350., 340., 200., 280., 26.),
    printer("Qidi Plus 4", 305., 305., 280., 180., 250., 24.),
];

#[derive(Clone, Copy, Debug, Serialize)]
pub struct Filament {
    pub name: &'static str,
    /// g/cm³.
    pub density: f64,
    /// Percent it shrinks as it cools.
    pub shrinkage: f64,
    /// Overhangs flatter than this (degrees from horizontal) want support.
    pub overhang: f64,
    /// Typical price per kilogram.
    pub price: f64,
}

/// Typical values; a maker's data sheet beats them.
pub const FILAMENTS: &[Filament] = &[
    Filament { name: "PLA", density: 1.24, shrinkage: 0.2, overhang: 40., price: 20. },
    Filament { name: "PLA-CF", density: 1.29, shrinkage: 0.15, overhang: 40., price: 34. },
    Filament { name: "PETG", density: 1.27, shrinkage: 0.4, overhang: 45., price: 22. },
    Filament { name: "ABS", density: 1.04, shrinkage: 0.7, overhang: 45., price: 22. },
    Filament { name: "ASA", density: 1.07, shrinkage: 0.6, overhang: 45., price: 27. },
    Filament { name: "TPU", density: 1.21, shrinkage: 0.5, overhang: 50., price: 32. },
    Filament { name: "PA (Nylon)", density: 1.12, shrinkage: 1.2, overhang: 50., price: 45. },
    Filament { name: "PA-CF", density: 1.17, shrinkage: 0.4, overhang: 45., price: 70. },
    Filament { name: "PC", density: 1.20, shrinkage: 0.6, overhang: 45., price: 38. },
    Filament { name: "HIPS", density: 1.04, shrinkage: 0.6, overhang: 45., price: 24. },
    Filament { name: "PVA", density: 1.23, shrinkage: 0.3, overhang: 45., price: 70. },
    Filament { name: "Standard resin", density: 1.12, shrinkage: 0.6, overhang: 30., price: 32. },
];

pub fn printer_named(name: &str) -> Option<&'static Printer> {
    PRINTERS.iter().find(|p| p.name.eq_ignore_ascii_case(name.trim()))
}

pub fn filament_named(name: &str) -> Option<&'static Filament> {
    FILAMENTS.iter().find(|p| p.name.eq_ignore_ascii_case(name.trim()))
}

/// Metric fastener dimensions, in mm.
#[derive(Clone, Copy, Debug, Serialize)]
pub struct Fastener {
    pub size: &'static str,
    pub nominal: f64,
    /// Through hole, normal fit (ISO 273 medium).
    pub clearance: f64,
    /// Hole a screw cuts its own thread in.
    pub self_tap: f64,
    /// Socket cap head diameter and height (ISO 4762).
    pub head: [f64; 2],
    /// Hex nut across flats and thickness (ISO 4032).
    pub nut: [f64; 2],
    /// Heat-set insert: hole diameter and depth for common brass inserts.
    pub insert: [f64; 2],
}

const fn fastener(
    size: &'static str,
    nominal: f64,
    clearance: f64,
    self_tap: f64,
    head: [f64; 2],
    nut: [f64; 2],
    insert: [f64; 2],
) -> Fastener {
    Fastener { size, nominal, clearance, self_tap, head, nut, insert }
}

pub const FASTENERS: &[Fastener] = &[
    fastener("M2", 2.0, 2.4, 1.7, [3.8, 2.0], [4.0, 1.6], [3.2, 4.0]),
    fastener("M2.5", 2.5, 2.9, 2.1, [4.5, 2.5], [5.0, 2.0], [3.8, 5.0]),
    fastener("M3", 3.0, 3.4, 2.6, [5.5, 3.0], [5.5, 2.4], [4.0, 6.5]),
    fastener("M4", 4.0, 4.5, 3.5, [7.0, 4.0], [7.0, 3.2], [5.6, 9.0]),
    fastener("M5", 5.0, 5.5, 4.4, [8.5, 5.0], [8.0, 4.7], [6.4, 10.5]),
    fastener("M6", 6.0, 6.6, 5.2, [10.0, 6.0], [10.0, 5.2], [8.0, 13.5]),
    fastener("M8", 8.0, 9.0, 7.0, [13.0, 8.0], [13.0, 6.8], [9.7, 13.5]),
];

pub fn fastener_named(size: &str) -> Option<&'static Fastener> {
    FASTENERS.iter().find(|f| f.size.eq_ignore_ascii_case(size.trim()))
}
