# Contributing to RyboCraft

Agents: start from the router in [AGENTS.md](AGENTS.md), which points to a short skill in [`skills/`](skills/README.md) for each kind of work. This page stays the full human guide.

RyboCraft is an open-source parametric 3D CAD app written in pure Rust.
You sketch on a plane, constrain the sketch, turn profiles into solids, and
change a parameter to watch the timeline rebuild. It runs as a desktop app (Windows, macOS, Linux), in
the browser (wasm + WebGPU/WebGL2), headless from a CLI, and as an MCP server that AI agents can
model with.

RyboCraft is developed by R.G. Studios (Ryan B. Gyles). This page covers how to
build it, how the code is laid out, the few rules that aren't negotiable, and where help is
needed most.

Before choosing a task, check the [command inventory](docs/features.md) and
[roadmap](docs/roadmap-detail.md) for implemented capabilities and current priorities.

## Kernel work goes in rybocore/

Kernel and geometry work is welcome: booleans, fillets, chamfers, shell, offsets, sweeps and lofts,
threads and holes, STEP/IGES import and export, tessellation. It goes in `rybocore/`, the vendored
rybocore kernel, following [`rybocore/docs/layers.md`](rybocore/docs/layers.md) and
`skills/rybocraft-layers/SKILL.md`. RyboCraft's own `crates/kernel`, `crates/smlib` and
`crates/geom` only shrink toward the seam (ops, tess, record); please don't grow them.

A kernel pull request targets `main` like any other; a change that touches the kernel and the app is one
pull request. A big feature in several steps goes as stacked pull requests into `main`, each one landing in order
on top of the one before.
Crashes and data loss are still taken first.

## Get it running (about 10 minutes on a fast machine)

You need stable Rust 1.95 or newer (required by egui/eframe 0.36). For the web
build you also need `rustup target add wasm32-unknown-unknown` and [`trunk`](https://github.com/trunk-rs/trunk)
([documentation](https://trunk-rs.github.io/trunk/)).

```sh
git clone https://github.com/Mr5elfDe5truct/RyboCraft && cd rybocraft
cargo run --release -p rybocraft -- --sample      # desktop app with a sample part
cargo xtask test                                   # unit, engine and UI-scenario tests
cargo xtask ci                                     # everything a PR must pass (see below)
```

RyboCraft is the product, repository, Cargo package and executable name.
Build the desktop app with `cargo build --release -p rybocraft`, or the CLI with
`cargo build --release -p rybocraft-cli`. Run the CLI with
`cargo run --release -p rybocraft-cli -- commands` (or `target/release/rybocraft-cli commands`
after building). The old name appears only in the compatibility shims listed in
[docs/compatibility.md](docs/compatibility.md); don't add new uses of it.

Web: `cd apps/rybocraft-web && trunk serve --release`, then open the printed URL (`?sample`
opens the sample part, `?webgl` forces WebGL2). Saving and opening files in the browser needs
HTTPS or localhost, because it uses the Origin Private File System.

Windows from Linux: `cargo xwin build --release --target x86_64-pc-windows-msvc -p rybocraft`.

### NixOS: `nix develop`

NixOS has no `/usr/lib`, so nothing is on `LD_LIBRARY_PATH` and a plain `cargo run` finds none of
the libraries the app loads at run time: the Wayland ones winit opens the window through, the EGL
loader wgpu renders through, and libdbus-1, which rfd's file dialogs reach xdg-desktop-portal
through (a dialog then never appears and the pick returns no path):

```
ERROR rybocraft: winit EventLoopError: ... The wayland library could not be loaded
ERROR eframe::native::run: Exiting because of error: WGPU error: Failed to create surface for any enabled backend: {}
ERROR rfd::backend::xdg_desktop_portal::portal::libdbus: Can't connect to a portal: libdbus-1.so not found
```

`nix develop` (the [`flake.nix`](flake.nix) in the repository root) supplies Wayland,
X11, EGL, Vulkan-loader and D-Bus runtime libraries. It also exposes the NixOS
driver manifests in `/run/opengl-driver/share`, so Vulkan can discover the system
drivers. Enable `hardware.graphics.enable` on NixOS; other Linux distributions
may need a matching [nixGL wrapper](https://github.com/nix-community/nixGL).
The default development shell retains the Rust/C toolchain already on `PATH`;
`nix develop .#rust` supplies the pinned toolchain and builds in `target/nix`.

To launch, use `nix run .#rybocraft -- --sample`. To install the app's terminal
commands, desktop entry and generated icons, use `nix profile install .#rybocraft`.
`nix build` alone creates a build result; it does not install a launcher in your
desktop menu. The installed `rybocraft` command runs the wrapped app; `rybocraft-cli` is included as
well. See
[Nix installation and troubleshooting](docs/nix.md) for remote-flake commands,
desktop integration and how to distinguish an old executable on `PATH`.

## How the code is laid out

The [architecture page](docs/architecture.md) has the layer table. The short version:

- `crates/geom`: vectors, planes, meshes, measures.
- `crates/sketch`: our own constraint solver, plus profile finding.
- `crates/dynamics`: multibody dynamics: rigid bodies, joint constraints, gravity and springs,
  integrated in time (RFC #9719). It depends only on `geom`.
- `crates/kernel`: the B-rep kernel boundary: solids, extrude/revolve, booleans, fillets,
  meshes, measures and STEP, behind our own API (implemented with `rybocore`). `crates/smlib` is
  the safe wrapper around `rybocore`, and `rybocore` is vendored in this repository at
  `rybocore/`; `.github/workflows/rybocore-mirror.yml` mirrors changes to it back to
  [Mr5elfDe5truct/RyboCraft](https://github.com/Mr5elfDe5truct/RyboCraft).
- `crates/doc`: the document. It holds parameters and expressions, the feature timeline and its
  incremental rebuild, components and occurrences, joints, appearances, and persistent naming.
- `crates/io`: design files and mesh formats (3MF, STL, OBJ).
- `crates/engine`: the session and the command registry. **Everything the user can do is a
  command**, and the toolbar, dialogs, scripts, CLI, control channel and MCP all run the same
  commands.
- `crates/ui-egui`: the desktop/web front end (egui + wgpu). It's swappable; nothing below it
  depends on egui.
- `crates/mcp`, `apps/rybocraft`, `apps/rybocraft-cli`, `apps/rybocraft-web`.

`cargo xtask layers` fails the build if a lower layer depends on a higher one.

Whether a change belongs in RyboCraft or in RyboCore is decided by
[`rybocore/docs/layers.md`](rybocore/docs/layers.md). Its tests: the **kernel** (`rybocore`)
computes geometry any client could use; **ops** (`rybocore-ops`) is a recipe of kernel calls for a
workflow, with no geometric quantity of its own; the **app** (RyboCraft) holds state and maps input
to commands, with no geometry math.

## Rules that aren't negotiable

1. **Independent.** RyboCraft is independent. Behaviour is designed from public
   documentation, general CAD conventions and user feedback. Reference tests use analytic values
   and open-source tools. We don't drive, script, automate or extract data from commercial CAD
   products, and we never copy another product's code, icons, artwork, help text, tooltips,
   templates or materials. Command ids are our own (`solid.fillet`).
2. **No GPL/LGPL/AGPL code, not even reading it.** That rules out OpenCASCADE, FreeCAD
   (including planegcs), SolveSpace, CadQuery/OCP, LibreCAD and QCAD. RyboCraft is
   MIT OR Apache-2.0, and its solver and blends are written from the maths. (Running an
   open-source application to get reference values for a test is fine; reading its source is
   not.) Permissive
   dependencies are fine. Note which one you're adding and why in the PR. THIRD-PARTY-LICENSES.txt
   lists it once the change lands: the Generated files workflow rewrites it on `main`.
   Nightly/manual CI also scans advisories, bans and sources with cargo-deny, and a PR that
   changes Cargo.lock or a Cargo.toml checks the advisories; see
   [the local command and coverage limits](docs/book/src/dev/testing.md#dependency-advisories).
3. **Every asset is original or openly licensed and has a row in
   [ATTRIBUTION.md](ATTRIBUTION.md).** Icons are drawn in code (`crates/ui-egui/src/icons.rs`).
   `cargo xtask assets` enforces this.
4. **Never crash.** People trust a CAD app with their work.
   - Non-test code never uses `unwrap`, `expect`, `panic!` or indexing on input-derived data.
     There's no `unsafe` outside `apps/rybocraft/src/open_documents.rs` (macOS file-open events),
     `apps/rybocraft/src/native_crash.rs` (Windows native crash capture) and
     `crates/threedx-client/src/macos.rs` (the 3Dconnexion driver's client API on macOS), and the
     compiler denies it.
   - An unsupported case returns an error ("not supported yet: …"); it never panics.
   - Every call into the vendored kernel goes through `rybocraft_kernel::guard`.
   - Every crash fix comes with a regression test.
5. **Everything is a command, with a test.** A new feature means a `CommandSpec` in
   `crates/engine/src/cmd/` with input and output schemas (see
   [Adding a command](docs/book/src/dev/commands.md#adding-a-command); a test warns about a
   command without them, and will fail it once required), engine tests that measure the result (volume, area,
   face/edge/vertex counts against a value you can derive), and for UI work a scenario (below).
   A command on the toolbar also ships with its help tooltip (step 6 of Adding a command).
6. **One set of camera controls.** Every page or canvas that shows a view (the modelling
   viewport, the Render preview, and any new page such as a flat pattern or drawing) calls
   `navigation::handle` (`crates/ui-egui/src/navigation.rs`) for its camera input, with
   `NavCaps` saying whether it can orbit (off for flat 2D pages) and pick. New canvases must use
   `navigation::handle`; never read drags, the wheel or pinch to move the camera yourself. The
   `hand_rolled_camera_input_is_rejected` test fails if a page does.

Labels, parameter docs, messages and docs use the terms in the
[glossary](https://github.com/Mr5elfDe5truct/RyboCraft): one term for each idea. A change that adds a new
idea adds its term there.

[AGENTS.md](AGENTS.md) has the long form of these rules. It's written for AI agents, but it's
the most complete description of the house style.

## Tests

| What | Where | Run |
|---|---|---|
| Unit and engine tests | next to the code, or `*_tests.rs` files | `cargo xtask test` (or `cargo test --workspace --profile ci`) |
| UI scenarios | `crates/ui-egui/tests/scenarios/*.json` | part of `cargo test` |
| Reference tests | `cargo xtask oracle` | `tests/oracle/` (see below) |

The reference for what the kernel builds is the kernel-cases ladder (`tests/kernel-cases/ladder/`,
`cargo xtask kernel-cases --ladder`, [docs/kernel-ladder.md](docs/kernel-ladder.md)).

**Kernel cases go to rybocore.** A case that checks a kernel operation (a boolean, a blend, a
shell, a sweep, an import's geometry) is a new entry in rybocore's corpus,
`rybocore/crates/rybocore/tests/kernel_cases/<area>/` in this repository, with an answer
in closed form or exact. Draft it as a rig case here, export it with
`cargo xtask kernel-cases --area <area> --export DIR`, add `DIR/<area>/<case>.json` and its
`.bodies.json.gz` under `rybocore`, and delete the draft; or write the entry by hand as
that folder's README describes. The mirror workflow pushes the change to
[Mr5elfDe5truct/RyboCraft](https://github.com/Mr5elfDe5truct/RyboCraft). `tests/kernel-cases/` takes
no new cases: its `CASES` list only shrinks (`cargo xtask lists`). App-level cases (what a command
does, the UI, product rules) stay in RyboCraft, as tests in their area's existing test file or UI
scenarios.

The naming corpus checks persistent names: every name a corpus design stores still finds a face or
an edge, and two builds name its faces and edges alike
(`cargo test --profile ci -p rybocraft-cli --test naming_corpus`).
[docs/mutation-testing.md](docs/mutation-testing.md) shows how to find kernel behaviour these
black-box suites miss.

**UI scenarios** drive the real app headless: start a tool, click a plane or a sketch point,
press keys, then assert on the result.

```json
[
  {"start": "sketch.create"},
  {"click": {"plane": "XY"}},
  {"start": "sketch.rectangle.two_point"},
  {"click": {"sketch": [0, 0]}},
  {"click": {"sketch": [40, 30]}},
  {"expect": {"errors": 0}}
]
```

A new `.json` file in that folder is picked up automatically. To record a bug before it's fixed,
put `{"pending": "why"}` in the scenario, and remove it in the PR that fixes the bug.

**A failing test without its fix can't merge as-is:** it would turn `main`'s CI red for everyone.
Either include the fix in the same PR, or mark the test `#[ignore = "fails until #123 is fixed"]`
(it can then merge right away), and remove that line in the PR that fixes the bug.

**Use the `ci` profile.** CI runs the tests with `--profile ci`: light optimisation, line tables,
no LTO, and debug assertions off. It
builds several times faster than `--release`. If a test fails only in a plain debug build, say so
in the PR. `cargo xtask test` builds the tests with it and then runs every test binary at once
(`cargo test` runs them one after another); arguments after `--` go to each binary, e.g.
`cargo xtask test -- shell`. Set `RYBOCRAFT_TEST_JOBS` to a positive integer to opt into a
shared bound on top-level test/side commands for `cargo xtask test` and `cargo xtask ci`.
For example, `RYBOCRAFT_TEST_JOBS=2` allows two commands at once; the sequential wasm/doc
worker consumes one of those slots. Every binary and side check still runs after failures.
Without the variable the existing concurrency is unchanged. With it set, each test binary
inherits `RUST_TEST_THREADS` and `ACTION_MATRIX_JOBS` of `max(1, cpus / jobs)` — the machine's
parallelism shared between the binaries in flight — unless you set either one explicitly;
explicit caller overrides always win. Cargo's build-job setting remains an independent caller
override; this limit does not change test filters, timeouts or baselines.
Tests run beside each other on a busy machine, so a test must not depend on wall-clock timing.

**Ready pull requests test what they touch.** CI maps changed files to workspace packages and
runs Clippy, the test build and tests for those and their dependents. The
[package selector](scripts/ci/affected.py) and [CI workflow](.github/workflows/ci.yml) define
which changes require native checks; documentation, packaging, other-workflow and bot-script-only
changes run the repository checks only, on pull requests and on `main`. Workspace-wide changes,
merge-queue groups, nightly runs and Run workflow select the full workspace. A push to `main`
reuses a successful merge-queue run for the same commit, and runs only the repository checks
when no code changed since the last push to `main` that passed; otherwise it runs the full
suite. Doc tests, reference tests and the Windows jobs run on `main` and nightly only.

**CI shards the tests.** One job builds them into a cargo-nextest archive; two shards run on
pull requests and four during full validation. Every test runs in its own process: don't rely
on state left by another test.
Pull requests skip a few heavy tests (one to twelve minutes each), which run in full validation
(the merge queue, nightly, manual runs and pushes to `main` not already tested by the queue).
Check that validation after merging. They are listed in
`.config/nextest.toml` and the [testing page](docs/book/src/dev/testing.md):

- `rybocraft-engine`: `tests::hostile_params_never_panic`, everything in `tests::robustness`,
  `pattern_suppress_tests::enclosure_arc_extinction_keeps_a_real_sharp_inner_lip_after_resize_and_reopen`,
  `pattern_suppress_tests::enclosure_datums_follow_wall_height_and_board_depth_with_material_proofs`,
  `cmd::plastic::tests::enclosure_snap_fit_follows_resized_front_wall`, the `atomic_commands` contract tests
- `rybocraft-kernel`: `tests::boolean_identity_on_tangent_primitives`, `tspline::tests::hostile_tsplines_never_panic`, and the `boolean_props` property tests
  (these run only in the nightly "Property tests" job: their inputs are random, so main skips them too)
- `rybocraft-ui-egui` scenarios: `acceptance_sensor_enclosure`

**The action matrix** tries every ordinary action on every face, edge, vertex and body of a
corpus of designs against a checked-in ratchet ([docs/action-matrix.md](docs/action-matrix.md)).
A pull request that makes an action pass should record it with `cargo xtask action-matrix --bless`;
one that makes a passing action fail fails the sample every CI run checks.

**The reference tests** live in [`tests/oracle/`](tests/oracle/), each a `recipe.json`
in our own format with an `expected.json` beside it. RyboCraft rebuilds the recipe and compares
body count, volume, area, centre of mass and bounding box (and, where there is a `reference.stl`,
the surface itself). Expected values are either worked out in closed form
(`tools/oracle-analytic/`) or computed by building the same recipe in FreeCAD, run as a separate
program (`tools/freecad-oracle/`); each file says which. The generated
[reference test report](docs/oracle.md) has the current case count and results. The reference-test
CI job runs `cargo xtask oracle` during full validation, and on pull requests that reach
`rybocraft-cli`. A case that
fails because of a known bug names its issue in `expected.json` (`"known_failure": "#123"`);
never change an expected value to match RyboCraft's output.

## Changing the design file format

A design file says which format it is in (`"format": "rybocraft/N"`). If your change saves
something new in a design (a field, a feature type, a new kind of name), or makes older designs
mean something else, it needs a format change. **Add one file, and edit nothing shared:**

1. Create `crates/doc/src/format/pending/<what-it-is>.rs` (lowercase, dashes). Start it with a
   `//!` note: what is saved, what older designs get without it, and that the last release asks
   for an update instead of meeting it.
2. If older designs must be rewritten as they open, put `pub fn step(v: &mut Value) ->
   crate::Result<()>` in the same file. Pending steps run in file-name order, after the older
   formats' steps.
3. Don't touch `FORMAT`, `JSON_STEPS`, the history in `crates/doc/src/format.rs`, another
   change's file, or any `v<N>/` folder: those are fixed once released.

That's all. The build counts the files: while any are pending it writes the next format number,
which no release reads, so a release warns about a design that has your change and keeps what it can't read of it.
A file also lists the pending changes it has (`format_pending`), so a later development build
runs only the steps that file is missing. Two pull requests that each add a change merge without
a conflict. `crates/doc/src/format/pending/README.md` has an example.

**At release time** the maintainer runs `scripts/fold-format-pending.sh`, which moves the pending
files into `crates/doc/src/format/v<N>/` and fixes that number for good (see
[docs/releasing.md](docs/releasing.md)). Never add a file to a `v<N>/` folder afterwards. Drawings
follow the same rules with their own numbers, from 1, in `crates/doc/src/drawing/format/` (see its
`pending/README.md`), and the same script folds them.

## Lists many changes add to

Some files gain an item in many pull requests at once. They are kept so that two changes that
each add an item only meet when their items are neighbours, which spares everyone a conflict
and a fresh CI run:

- **Growing Rust lists**: use slices (`const ITEMS: &[Item] = &[...];`) without a written
  length. `cargo xtask lists` rejects non-test const/static arrays with more than ten lines
  of entries, except fixed standard or algorithm tables listed with reasons in `xtask/src/lists.rs`.
  Built-in themes need only a JSON file in `assets/styles/ui/themes/`, with a unique nonnegative
  integer `order` (the picker order) and its own `provenance` line; keep existing order values.
- **Message catalogs** (`assets/locales/*.json`): one message per line, sorted by id. Add your
  message anywhere and run `cargo xtask lists --fix`; `cargo xtask lists` (in `cargo xtask ci`
  and the Lists workflow) fails on an unsorted catalog or an id that appears twice.
- **`mod` lines**: one per line, in the sorted group they belong to (`cargo fmt` keeps each group
  sorted). Add a new module into the existing group, not as a new group at the end.
- **Commands without schemas** (`crates/engine/src/schema/unconverted.txt`): sorted, one id per
  line, and it only shrinks. Converting a command removes its line; a new command is never added
  to it.
- **Toolbar commands without help** (`BACKLOG` in `crates/ui-egui/src/command_help.rs`): sorted,
  one id per line, and it only shrinks. Writing a command's help removes its line; a new command
  is never added to it.
- **`ui.prefs` settings** (`docs/control-protocol.md`): one line per setting, sorted by name, in
  the list under the protocol table; add a new setting on its own line rather than editing a shared
  row.
- **Languages** (`crates/ui-egui/src/i18n.rs`): one per line, English first, then by id (a test
  checks it).
- **Generated files**: `THIRD-PARTY-LICENSES.txt` and `docs/features.md` are rewritten on `main`
  after each merge by the Generated files workflow (`cargo xtask licences`, `cargo xtask
  features`). Don't regenerate or edit them in a pull request.
- **Design format**: one file per change, never an edit to a shared number (above).
- **New UI strings and attribution rows**: one new file per change, `assets/locales/pending/<slug>.json`
  and `attribution/pending/<slug>.md`, instead of editing the catalogs or the ATTRIBUTION.md table;
  `cargo xtask fold-pending` folds them on `main` (see the README in each folder). A new English
  message also needs every catalog delivered complete (`COMPLETE` in `crates/ui-egui/src/i18n.rs`:
  today ar, he, ja, vi, and zh) in the same pending file; `cargo xtask i18n --check` reports any gap.

## Sending a change

1. Open an [issue](https://github.com/Mr5elfDe5truct/RyboCraft/issues) first for anything bigger
   than a bug fix, so the work can be agreed before it starts.
2. Branch from `main` and keep the pull request to one topic.
3. Run `cargo xtask ci` before every commit.
4. Fill the pull request's `Changes a default:` line: `Changes a default: no`, or
   `Changes a default: yes — <what>, old behaviour kept behind preference <name>`. A default here
   is a UX default only: a control, setting, preference or tool behaviour a user sees by default.
5. Show observable changes with real screenshots or a recording.

## Where help is wanted most

**Kernel speed (slow fillets, booleans, STEP):** report slow cases with a file, and put kernel effort into the vendored kernel in `rybocore/`.

See the [`help wanted`](https://github.com/Mr5elfDe5truct/RyboCraft/labels/help%20wanted) and
[`good first issue`](https://github.com/Mr5elfDe5truct/RyboCraft/labels/good%20first%20issue) labels.
In order of how much they'd help users:

1. **Fillet and blend robustness.** Fillets are the vendored kernel's code (`rybocore`).
   Constant, variable-radius and chord fillets work, and so do corners where fillets of
   different radii meet, including three rounds meeting at a vertex. Next are setback corners, fillets along curved edges between curved
   faces, and fillets that consume neighbouring faces.
2. **Boolean robustness.** A random-design fuzzer currently gets about half of its designs
   through without a kernel error. Each failing seed is a reproducible bug.
3. **Drawings, CAM and mesh tools.** These are out of scope for now (see
   [ROADMAP.md](docs/roadmap-detail.md)), but anyone who wants to own one is welcome.

## Updating the generated feature list

After adding or renaming commands, run `cargo xtask features`. The registry is the
source of truth for `docs/features.md`; do not hand-edit the generated rows.
The generator sorts commands by ID and omits global and per-section counters,
so additions to unrelated areas no longer rewrite shared summary lines.

If two branches still conflict in this file (for example, adjacent additions in
the same area), first resolve the command registry and source conflicts, then run
`cargo xtask features` again and stage the regenerated file. Do not use Git's
`union` merge driver: it can silently retain stale or duplicate command rows.
The generator tests cover stable ordering and additions that leave existing lines
unchanged. The initial switch to sorted rows is a one-time larger diff.

## Pictures in the user guide

The user guide's screenshots and short clips (`docs/book/src/media/`) are captured from the real
app by `examples/demo/doc_shots.py`, one function per figure. Each run starts a fresh app with its
own settings, so your preferences never show. When a change makes a figure out of date, run
`python3 examples/demo/doc_shots.py <page or shot>` (it needs Xvfb, ffmpeg, Pillow and a release
build) and commit the new file. Stills come in both themes: `--light` makes `<name>.light.png`,
and the page shows the one that matches the book's theme (an `<img class="dark-ui">` and an
`<img class="light-ui">`). A new figure is a new function there, an ATTRIBUTION row in
`attribution/pending/`, and a `<video>` (clips) or the image pair on the page. Keep a still under
about 250 KB and a clip under about 1 MB.

## Licence

By contributing you agree that your contribution is licensed as MIT OR Apache-2.0, like the rest
of RyboCraft. RyboCraft isn't affiliated with any CAD vendor; product names are trademarks of their owners (see NOTICE).
