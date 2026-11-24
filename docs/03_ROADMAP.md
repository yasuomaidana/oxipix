# 03 — Roadmap

Replaces [`archive/16_DOCUMENT_CAPTURE_AND_RECEIPT_IMAGING.md`](archive/16_DOCUMENT_CAPTURE_AND_RECEIPT_IMAGING.md),
which assessed the deleted seven-crate workspace, and carries forward the still-valid requirements of the draft
document-layout spec (LAY-1 to LAY-4, below).

Direction (2026-10, see [ADR-0001](adr/0001-single-crate-on-top-of-image.md)): oxipix holds only what cannot live in
`image` or `rxing`: wgpu acceleration and generic document-capture computer vision. CPU improvements to those crates
go upstream; see [`FORKS.md`](FORKS.md).

IDs below are local (`RM-`). Map each to a Plane ticket (`OXI-nn`) with `dendrite` when it is scheduled.

## 1. What changed since the 16 spec

| Old ticket | Requirement | Status against today's code |
|---|---|---|
| OXI-1501 | Real JPEG and PNG decoding with EXIF orientation | **Dropped.** Decoding is `image`'s job; oxipix operates on `image::ImageBuffer`. |
| OXI-1502 | Quad detection, homography, bilinear warp | **Partial.** Homography (`quad_to_rect`, via `oximedia-cv`) and a bilinear GPU/CPU warp exist. Quad *detection* does not: see RM-QUAD. |
| OXI-1503 | Thermal-receipt enhancement | **Open.** See RM-ENHANCE. |
| OXI-1504 | QR finder patterns under rotation and perspective | **Out of scope.** Barcode decoding belongs to `rxing`; improvements go upstream. |
| OXI-1505 | `gpu` as a non-default feature | **Done.** `gpu` is opt-in. `cargo build --no-default-features` works. |
| OXI-1506 | Reconcile DEV_STATUS with the code | **Done.** DEV_STATUS describes the single crate. |

## 2. Release 0.1.0

First crates.io release. Nothing new in the API: the `mosaic`, `vectorize` and `gpu` modules as they are, with the
quality gates, 100% coverage, documentation and packaging described in [`02_QUALITY_GATES.md`](02_QUALITY_GATES.md).

## 3. Open work

| ID | Priority | Requirement | Notes |
|---|---|---|---|
| RM-ECC | high | Make `estimate_orthogonal_drift_ecc` return the drift of a shifted strip | Observed to return 0 for shifts of 1 to 3 px; see [architecture §5](01_ARCHITECTURE.md). Either fix the use of `register_ecc` (initial warp, iteration count, tolerance) or replace it with phase correlation. Needs a regression test with a known shift. |
| RM-QUAD | high | Detect the largest document quadrilateral in a photograph | Canny, contours, polygon approximation, quad scoring; replaces the bounding-box crop in `auto_dewarp_dominant_quad`. Acceptance: corners within 2% of the image diagonal on a fixture set of receipts on tables. |
| RM-ENHANCE | medium | Thermal-receipt enhancement chain | CLAHE, background division for shadows, denoise, unsharp mask, and a `receipt_enhance()` preset that feeds a local threshold. Check what `imageproc` already provides before writing anything. Acceptance: measurable OCR improvement on faded fixtures. |
| RM-WARP | medium | One border policy for the CPU and GPU warps | The GPU warp fills with white and the CPU warp edge-clamps; make it a parameter. |
| RM-DOCS | high | OpenCV-style documentation site | Plan: [`plans/OPENCV_STYLE_DOCS_PLAN.md`](plans/OPENCV_STYLE_DOCS_PLAN.md). |
| RM-BENCH | high | Performance comparison against the `opencv` crate | Part of the same plan; kept out of the published crate's dependency graph. |
| RM-SPLIT | low | Split into `oxipix-gpu`, `oxipix-mosaic` and `oxipix-vectorize` | Only if justified, as a separate pull request. |
| RM-OTSU | low | Replace `vectorize::otsu_threshold` with `imageproc::contrast::otsu_level` | Only after parity is verified; the two use different conventions (`<=` versus `<`). |

## 4. Planned feature: `layout`

Locating printed pages, smaller paper items and machine-readable code regions in photographs, as a step before
recognition by another library. It complements the capture and flattening work above. No tickets exist yet; this is the
requirement and its acceptance criteria.

### 4.1 Gaps

1. Finding and flattening one document in a phone photograph is covered by RM-QUAD. Several items in one frame (for
   example a page lying next to a smaller slip of paper) are not.
2. Locating (as opposed to decoding) linear barcodes and 2D code regions is not covered anywhere.
3. No feature exposes detected regions of an input image.

### 4.2 Requirements

| ID | Priority | Requirement |
|---|---|---|
| `LAY-1` | high | Detect up to N rectangular paper regions in one image; return ordered quadrilaterals with confidence |
| `LAY-2` | high | Deskew and crop each region to an upright image at a requested resolution |
| `LAY-3` | medium | Locate candidate regions of linear barcodes and 2D codes; return quadrilaterals and a coarse kind (linear / matrix) without decoding |
| `LAY-4` | low | Stable region ordering (reading order) so results are reproducible |

### 4.3 Specifications

**LAY-1 and LAY-2: multi-region detection and normalisation.**
Request: `detect_pages(image, max) -> Vec<Region>` and `rectify(image, &Region, dpi) -> Image`. Pure Rust, no network,
deterministic for equal inputs.

Acceptance criteria:

- On a labelled set of at least 50 photographs with one to three paper items, intersection-over-union with ground
  truth is at least 0.85 on 90% of items.
- Rectified text baselines are within 1 degree of horizontal on 95% of items.
- Equal inputs produce identical outputs across runs and platforms.

**LAY-3: code regions.**
Request: `detect_codes(image) -> Vec<CodeRegion>` returning location and coarse kind. Decoding stays with the caller.

Acceptance criteria: recall of at least 0.95 for codes occupying at least 2% of the frame at up to 30 degrees tilt on
the labelled set; at most one false positive per ten images.

**LAY-4: ordering.**
Acceptance criteria: regions are returned top-to-bottom, then left-to-right, by centroid, with a documented tolerance.

## 5. Constraints that apply to everything above

- **Toolchain:** pure Rust and zero C FFI on the default path. Native runtimes go behind opt-in features only.
- **Builds:** the default features should stay lean enough for `wasm32-unknown-unknown`, `aarch64-apple-ios` and
  `aarch64-linux-android` (`gpu` is opt-in for this reason). Building those targets is **not yet checked in CI** and
  `oximedia-cv` has not been tried on them; adding that job is part of RM-DOCS's platform matrix.
- **Determinism:** equal inputs give equal outputs; every pinned output is covered by the characterization tests.
- **Quality gates:** [`02_QUALITY_GATES.md`](02_QUALITY_GATES.md) applies to every change, including new code.
