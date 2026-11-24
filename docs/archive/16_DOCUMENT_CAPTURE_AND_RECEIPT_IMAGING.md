# 16 — Document Capture & Receipt Imaging

## 1. Purpose

This module makes oxipix usable for photographs of real documents taken on a phone: decoding JPEG and PNG camera output, finding and flattening the document, enhancing faded thermal paper, and reading tilted QR codes.

Requested by: Kakebo receipt capture, through docula (Plane `KAKEBODM`).

## 2. Assessment of the Current Implementation

1. **Decoding doesn't work yet.** `compat-image` returns errors such as "JPEG baseline parser reached EOF" and "PNG stream incomplete" (`compat-image/src/lib.rs:797-815`). `jpeg.rs` contains an encoder only. `00_TECHNICAL_ASSESSMENT.md` §2.4 plans federated decoders (for example `oximedia-codec` for PNG and WebP), but they aren't wired in.
2. **The status overstates what exists.** DEV_STATUS reports 15 codecs and 569/569 `image` test parity, while Hough deskew and contour tracing are absent from `oxipix-ops/src/cv.rs`.
3. **The QR decoder assumes axis alignment.** It takes the bounding box of dark pixels and finds no finder patterns, so it can't handle rotation or perspective.
4. **Perspective warp has two gaps.** `dispatch_perspective_transform` uses nearest-neighbour sampling, and nothing estimates the homography.
5. **`gpu` (wgpu 30) is a default feature,** which works against CPU-only mobile and wasm builds.

## 3. Requirements (Milestone 15: Document Capture & Receipt Imaging)

| Ticket | Priority | Points | Crate | Requirement |
|---|---|---|---|---|
| `OXI-1501` | urgent | 8 | `oxipix-codecs` | Real JPEG (baseline + progressive) and PNG decoding with EXIF orientation — federated per 00_TECHNICAL_ASSESSMENT §2.4 |
| `OXI-1502` | high | 8 | `oxipix-ops` | Document/receipt quad detection + homography estimation + bilinear warp |
| `OXI-1503` | high | 5 | `oxipix-ops` | Thermal-receipt enhancement: CLAHE, illumination/shadow removal, denoise, sharpen |
| `OXI-1504` | high | 5 | `oxipix-barcode` | Robust QR finder-pattern detection under rotation & perspective (CFDI QR) |
| `OXI-1505` | medium | 1 | `oxipix` | Make `gpu` (wgpu) a non-default feature; document mobile/wasm build |
| `OXI-1506` | medium | 2 | `oxipix` | Reconcile DEV_STATUS with code (codec claims, Hough/contours, 569-test parity) |

## 4. Specifications

### OXI-1501: Real JPEG (baseline + progressive) and PNG decoding with EXIF orientation — federated per 00_TECHNICAL_ASSESSMENT §2.4

`oxipix-compat-image` decoding returns errors such as "JPEG baseline parser reached EOF" and "PNG stream incomplete" (`compat-image/src/lib.rs:797-815`). `jpeg.rs` has no decoder, so no phone photo can be loaded, and that blocks every receipt flow downstream (docula M11, Kakebo).

**Request:**
- Implement the federation the assessment already plans: PNG and WebP decoding through the pure-Rust `oximedia-codec`, plus an in-house baseline and progressive JPEG decoder alongside the existing patent-clean encoder.
- Apply EXIF orientation.
- HEIC can go behind an optional `heic` feature.

**Acceptance criteria:** decode a corpus of real iOS and Android photos, pixel-matching a reference decoder within tolerance.

*Unblocks Kakebo KAK-406.*

### OXI-1502: Document/receipt quad detection + homography estimation + bilinear warp

`dispatch_perspective_transform` applies a matrix you already have, using nearest-neighbour sampling.

**Request:**
- Detect the largest document quadrilateral (Canny → contours → polygon approximation → quad scoring).
- Estimate the homography with 4-point DLT, plus RANSAC when there are more points.
- Warp on the CPU and the GPU with bilinear or bicubic interpolation.

**Acceptance criteria:** corners found to within 2% of the image diagonal on a fixture set of receipts photographed on tables.

*Unblocks Kakebo KAK-406.*

### OXI-1503: Thermal-receipt enhancement: CLAHE, illumination/shadow removal, denoise, sharpen

Thermal receipts fade and get photographed under uneven light.

**Request:**
- CLAHE.
- Background estimation and division to remove shadows and illumination gradients.
- Median or bilateral denoise.
- Unsharp mask.
- A preset `receipt_enhance()` chain that feeds `bradley_roth_threshold`.

**Acceptance criteria:** measurable OCR improvement on faded fixtures, using docula DOC-1202.

*Unblocks Kakebo KAK-406.*

### OXI-1504: Robust QR finder-pattern detection under rotation & perspective (CFDI QR)

The decoder takes the bounding box of dark pixels and assumes the QR code is axis-aligned.

**Request:**
- Finder-pattern search using 1:1:3:1:1 ratio scans.
- Alignment-pattern refinement.
- Perspective sampling.
- Rotation in any direction.
- Multiple codes per image.

**Acceptance criteria:** decode the SAT CFDI verification QR from receipt photos at 30° tilt and with blur.

*Unblocks Kakebo KAK-404.*

### OXI-1505: Make `gpu` (wgpu) a non-default feature; document mobile/wasm build

Default features pull in wgpu 30 and pollster. Kakebo's mobile and wasm profiles need a CPU-only default.

**Acceptance criteria:**
- `cargo build --no-default-features` works.
- CI builds iOS, Android and `wasm32` without `gpu`.

*Unblocks Kakebo KAK-403.*

### OXI-1506: Reconcile DEV_STATUS with code (codec claims, Hough/contours, 569-test parity)

DEV_STATUS reports 15 codecs and 569/569 `image` test parity, but the code has no JPEG or PNG decode, and Hough and contour tracing are absent from `cv.rs`.

**Request:** reopen or annotate the affected tickets so that downstream planning (docula M11, Kakebo) can rely on the status report.

*Unblocks Kakebo KAK-007.*

## 5. Cross-Cutting Constraints

- **Toolchain:** pure Rust and zero C FFI on the default path. Native runtimes go behind opt-in features only.
- **Async:** no `tokio/full` on library crates. Use a minimal runtime, and keep HTTP clients on rustls.
- **Builds:** default features stay lean enough to build for `aarch64-apple-ios`, `aarch64-linux-android` and `wasm32-unknown-unknown` (where applicable).
- **Determinism:** every model-backed output records the engine, model id and version, and a confidence score, so consumers can store provenance.
