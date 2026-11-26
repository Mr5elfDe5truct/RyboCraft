# RyboCraft

Parametric CAD for 3D printing, by R.G. Studios (Ryan B. Gyles). Sketch, constrain, extrude, and
change a parameter to watch the timeline rebuild, then prepare the part for printing in the same
app. RyboCraft has a desktop app for Windows, a command-line tool and an MCP server, so AI agents
can model with it too.

## Maker

Storage trays, drawer cabinets, swing towers, hinged boxes, pegboard holders, cups; 3D text, keychains,
nameplates, cake toppers, balloon letters, ornaments, snowflakes, coasters, QR codes; lithophanes, reliefs,
cut-outs, cookie cutters and SVGs from pictures; bolts, brackets, vases, dice, marble track, stands; and edits
for any model, STL or solid: text, texture, split, flexi joints, cavities, moulds, hollow, puzzle, flat base,
resize, loop, fuse and cut (`make.*`).

## 3D Print

- **Printer:** printers, filaments, slicing defaults, send to slicer.
- **Analyze:** printability score, overhangs, wall thickness, islands, layer preview, print estimate, filament comparison.
- **Prepare:** auto orient, lay face down, drop to bed, arrange, scale to bed, shrinkage, supports, brim ears, split to fit with pins.
- **Features:** heat-set inserts, nut traps, screw holes, magnet pockets, teardrop holes, lattices, hollow and drain, vents, keyholes.
- **Make:** enclosure, print-in-place hinge, grid bins and base plates, gear, knob, hook, spacer.
- **Calibrate:** cube, tolerance, overhang, bridging, temperature, retraction, first layer, thin wall, hole and fit tests.

All of them are commands (`make.*`, `print.*`), so `rybocraft-cli` and its MCP server have them too.

## Build

    cargo run --profile profiling -p rybocraft
    powershell -ExecutionPolicy Bypass -File packaging/windows/build-installer.ps1

The installer, `dist/RyboCraft-Setup-<version>.exe`, installs to
`C:\Program Files\RG Studios\RyboCraft`. `/S` installs silently.

## Use it from an AI agent

    claude mcp add rybocraft -- rybocraft-cli mcp

## Licence

MIT OR Apache-2.0, at your option ([LICENSE-MIT](LICENSE-MIT), [LICENSE-APACHE](LICENSE-APACHE)).
The required notices for the open-source code and assets RyboCraft includes are in
[NOTICE](NOTICE), [ATTRIBUTION.md](ATTRIBUTION.md) and
[THIRD-PARTY-LICENSES.txt](THIRD-PARTY-LICENSES.txt). See also [DISCLAIMER.md](DISCLAIMER.md).
