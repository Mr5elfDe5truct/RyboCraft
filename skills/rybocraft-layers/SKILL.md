---
name: rybocraft-layers
description: "Use when deciding whether code goes in RyboCraft or in rybocore (the vendored kernel in rybocore/), and in which rybocore crate: geometry, a modelling workflow, kernel work, or a review comment about the layer split."
---

# Where code goes: RyboCraft or rybocore

The authority is `rybocore/docs/layers.md`; this is its short form. Every job lives in exactly one layer, and two implementations of one job are a bug.

## Decide
1. **Does it compute geometry?** Anything computed from coordinates: a point, vector, length, angle, curvature, curve or surface, a solve, an intersection, a fit, a mesh, a STEP parse. Then it is a kernel primitive in `rybocore/crates/rybocore`, with its own tests. Dot and cross products, normalising, square roots or trigonometry on coordinates count, however small.
2. **Is it a workflow recipe of kernel calls?** It picks entities, parameters and defaults, sequences kernel calls, compares what they return with tolerances and reports. Then it is `rybocore/crates/rybocore-ops`. Ops computes no geometric quantity of its own: when it needs one the kernel lacks, add a kernel primitive and call it.
3. **Is it state, selection, commands, settings, history or UI?** Then it is RyboCraft (`crates/`, `apps/`), which computes no geometry.

## What RyboCraft may use
- `rybocore-ops` (modelling), `rybocore-tess` (display meshes, polylines) and `rybocore-record` (geometry as data). CI enforces this seam through `cargo xtask layers` (#13498): rybocore's public API is listed in xtask/rybocore-public.txt, and the remaining direct uses in xtask/rybocore-seam.txt, a list that only shrinks. The same command keeps rybocore/ self-contained: no manifest under it may use a `path` that leaves rybocore/ or depend on a `rybocraft-*` crate, and CI's "RyboCore builds on its own" job runs `cargo check --workspace --all-targets --locked` on an export of rybocore/ alone.
- Before writing STEP, meshing, surface intersection, tangent chains or frames, look for rybocore's: a second implementation is flagged.

## Lists that only shrink
- `xtask/mesh-decisions.txt`: model decisions made from a mesh (`cargo xtask mesh-decisions`).
- `rybocore/plan/panic-sites.tsv`: panic-prone calls in rybocore (RyboCore checks in CI).

## Working in rybocore/
- Follow `rybocore/AGENTS.md` and the reference doc for the area (`rybocore/docs/`). A change there runs RyboCore checks (its tests, goldens, kernel rig, clippy, fmt, panic sites) in `.github/workflows/ci.yml`.
- New kernel cases go in `rybocore/crates/rybocore/tests/kernel_cases/`, never in RyboCraft (`cargo xtask lists`).
- Kernel work goes here: CONTRIBUTING.md, "Kernel work goes in rybocore/". Kernel PRs target `main`; big multi-step work goes as stacked PRs into `main`.
- Issues and claims for rybocore are in this repository, like any other change (`skills/rybocraft-contributing/SKILL.md`).
