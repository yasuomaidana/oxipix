# oxipix development status

## Direction (2026-10)

oxipix no longer replaces `image` / `rxing`. It is a single crate holding only what cannot live
upstream: wgpu acceleration and generic document-capture CV (mosaic, vectorize). CPU improvements
go to forks of `image` and `rxing` as opt-in features, then upstream as PRs. See
[`docs/adr/0001-single-crate-on-top-of-image.md`](docs/adr/0001-single-crate-on-top-of-image.md).

## Done

- [x] Single crate; the seven `oxipix-*` workspace crates and the compat facades are removed.
- [x] `gpu`: persistent wgpu context, both kernels dispatched, CPU fallback, parity tests, benchmark.
- [x] `mosaic` and `vectorize` moved in from docula (`cv-mosaic`, `cv-vectorize`) and given tests.
- [x] Fixed: vectorize used `val < otsu` and produced no paths for pure black-on-white input (Otsu returns 0); now `<=`.
- [x] docula depends on real `image` and on oxipix (`mosaic`, `vectorize`).
- [x] Quality gates in CI: zero duplication, cyclomatic 10 / cognitive 15, 500 lines per file, 100% line coverage,
      feature matrix, MSRV, rustdoc, cargo-deny, package dry run ([`docs/02_QUALITY_GATES.md`](docs/02_QUALITY_GATES.md)).
- [x] Behaviour of every public `mosaic` and `vectorize` function pinned by golden-output tests; duplication and
      complexity refactors verified against them.
- [x] README, architecture, roadmap, benchmark methodology and ADRs rewritten for the single crate.

## Next

- [ ] **Release 0.1.0 to crates.io** (see [`docs/03_ROADMAP.md`](docs/03_ROADMAP.md) §2): confirm the crate name is free,
      docula CI green on the release candidate, then publish and tag.
- [ ] docula switches from its path dependency to `oxipix = "0.1"` after the release.
- [ ] Fork `image` and `rxing` topics, each gated on a benchmark win ([`docs/FORKS.md`](docs/FORKS.md)).
- [ ] OpenCV-style documentation site and the comparison with the `opencv` crate
      ([`docs/plans/OPENCV_STYLE_DOCS_PLAN.md`](docs/plans/OPENCV_STYLE_DOCS_PLAN.md)).
- [ ] Open work items RM-ECC, RM-QUAD, RM-ENHANCE, RM-WARP and the planned `layout` feature
      ([`docs/03_ROADMAP.md`](docs/03_ROADMAP.md)).

## Known limitations

- `mosaic` GPU warp fills uncovered pixels white; the `oximedia-cv` CPU warp edge-clamps.
- The Moore tracer caps each contour at 1000 steps.
- `estimate_orthogonal_drift_ecc` returned 0 for strips shifted by 1 to 3 px (RM-ECC).
- `auto_dewarp_dominant_quad` finds an outlined page and crops to its bounding box (RM-QUAD).
- Building for wasm32, iOS and Android is not checked in CI and `oximedia-cv` has not been tried there.
