# Fork runbook: `image` and `rxing`

Forks live under the `yasuomaidana` GitHub account and are cloned next to this repo (`../image`, `../rxing`).

## Branches

- `main`: mirrors upstream. Update with `git fetch upstream && git merge --ff-only upstream/main`.
- `feat/<topic>` / `fix/<topic>` / `perf/<topic>`: one branch per upstream PR, cut from upstream `main`.
- `oxipix-integration`: merge of all topic branches. Consumers use it via `[patch.crates-io]` pinned by `rev`.

## Rules

1. New `unsafe` or new dependencies go behind an opt-in, non-default cargo feature.
2. A topic is only ported if a criterion benchmark against upstream `main` shows a win; attach the numbers.
3. image-rs requires PR descriptions and comments to be written by a human, and mostly AI-generated PRs must be disclosed. Draft with tools if you like, but write and submit them yourself.
4. Do not add AI co-author trailers to commits.

## Topics

| Fork | Branch | Notes |
|---|---|---|
| rxing | `update-pin` | **Pushed.** `=0.25.8` -> `=0.25.10` (latest). Replaces the earlier `fix/relax-image-pin`, which was dropped. Suite passes (`issue_69_timed` has a hard 30 s limit that debug builds under load exceed, also on the old pin). |
| rxing | `fix/rgb-luminance-source-crop` | **Pushed.** `RGBLuminanceSource::crop` passed the source stride for a compact buffer, so `get_row` panicked inside `GenericMultipleBarcodeReader`. 3 regression tests. |
| rxing | `perf/multiple-reader` | `GenericMultipleBarcodeReader` is documented as slow. |
| rxing | `feat/simd-binarizer` | opt-in NEON Bradley-Roth / histogram. |
| image | `feat/neon-color` | opt-in `neon-color`. |
| image | `feat/nearest-resize-fastpath` | only if it beats current `main`. |

## Consuming

```toml
[patch.crates-io]
image = { git = "https://github.com/yasuomaidana/image", rev = "<sha>" }
rxing = { git = "https://github.com/yasuomaidana/rxing", rev = "<sha>" }
```

## State

Branches on `yasuomaidana/rxing`: `update-pin`, `fix/rgb-luminance-source-crop`. On `yasuomaidana/image`: `fix/resize-accumulator-init` (`e0aed3a8`), `feat/nearest-resize-fastpath` (`353fcd84`, stacked on it). The long-lived `oxipix-integration` branch and `fix/relax-image-pin` were deleted: nothing patches `image` or `rxing`, so the forks are only staging for upstream PRs and can be deleted once those merge. PR text drafts live in `../pr-drafts/` (outside this repo); image-rs requires human-written PR text.

## Benchmark gate results (2026-10-27, Apple M3, 1920x1080 RGBA)

> **Correction:** the scratch comparison below ran against the crates.io release **0.25.10**, not `main` (cargo kept the registry version over the `[patch]`). `upstream/main` @ 075e65d6 has a regression (resize adds +1 to every channel, fixed on `fix/resize-accumulator-init`). The nearest-resize numbers that matter are the in-tree ones at the end of this section.

Candidates were the old oxipix CPU fast paths, rewritten to be **bit-exact** with the `image` function they would replace (tests assert identical output). The machine was noisy; ratios are from two runs.

| Candidate | `image` main | Candidate | Verdict |
|---|---|---|---|
| nearest resize 1080p -> 540p (centre sampling) | 5.5-8.9 ms | 0.25-0.28 ms (~20-30x) | **PR worthy.** Safe code, no feature flag. `image` runs Nearest through its float pipeline (whole image to `Rgba32FImage`, box kernel with support 0). |
| RGBA -> luma, exact `(2126r+7152g+722b)/10000` NEON | `imageops::grayscale` 2.4-2.6 ms | 0.6-1.6 ms (~1.5-2.7x, noisy) | **Skip.** Needs `unsafe`, modest and noisy gain, and upstream just merged "Optimize RGB to Luma" (#2988) and moved `DynamicImage::to_luma8` to a colour-space-aware float path (differs by 1 from `grayscale`). |
| RGBA -> RGB NEON | `to_rgb8` 0.53-0.56 ms | 0.34-0.39 ms (~1.4x, noisy) | **Drop.** Memory-bound. |
| rxing `RGBLuminanceSource::crop` | panics | fixed | Bug fix, not a perf topic. Regression tests are the evidence. |

Caveats to handle in the nearest-resize PR: reproduce `image`'s index math exactly (`f32` ratio, `f32` vertical / `f64` horizontal centre, floor), keep the colour-space copy, and keep the `[0, 1]` clamp the old path applies to float subpixels.

Reproduce: the benchmark crate was a scratch project (not committed); the old code is in git history on `develop` (`crates/oxipix-simd/src/color.rs`, `crates/oxipix-ops/src/resample.rs`).

### Nearest resize, measured in the `image` repo itself (existing `benches/imageops.rs`)

1920x1080 Rgb8 source, same machine, before -> after on `feat/nearest-resize-fastpath`: `resize 400x300 Nearest` 2.20 ms -> 0.21 ms (~10x); `large/resize 2000x2000 Nearest` 23.2 ms -> 6.9 ms (~3.4x). Output is bit-identical to the generic pipeline (630 tests pass). The ~25x in the scratch table above overstated it; those two benches are the ones to quote.
