# ADR-0005: OpenCV-class kernels in oxipix, delegation for the rest of OpenCV

- Status: Proposed
- Date: 2026-11-27

## Context

The guide's benchmarks showed pure-Rust crates 1.2x to 75x slower than OpenCV 5.1 with KleidiCV. A full pure-Rust
OpenCV was proposed: opt-in sub-crates, proc macros, opt-in threads, wgpu and WebAssembly. The study in
[`../research/opencv-parity/`](../research/opencv-parity/00_SUMMARY.md) (OXI-99) measured oxipix `imgproc`
(OXI-94 to OXI-96), kornia-rs, imageproc and a safe-Rust spike against OpenCV on the same fixtures, at 1 and 8 threads.
OpenCV speed is reachable in Rust. Single-threaded, the best Rust crate is at or below OpenCV time on 10 of 23
operations with the guide fixtures, and on 7 to 8 of 23 at 1920 and 3840 px.
It is within 3x on 19 of 23 at every size. No upstream crate meets the threading, wgpu and wasm requirements together. Half of OpenCV is wrappers around
problems that existing Rust crates already solve.

## Decision

1. oxipix builds the kernel-heavy modules: imgproc (including features), photo, and video background subtraction
   once a ticket with a consumer exists. It uses the architecture in
   [`04_ARCHITECTURE.md`](../research/opencv-parity/04_ARCHITECTURE.md) revision 2:
   - strided views;
   - a stripe scheduler behind `parallel`;
   - `macro_rules!` kernel helpers, and a `#[kernel]` proc macro only if its prototype passes the coverage gate;
   - a wgpu layer with an explicit, async context that also builds for wasm.

   Sub-crates are introduced in stages, each with a trigger (04 §3). The first stage adds only `oxipix-macros`.
2. Codecs, video I/O, DNN, ml, flann and GUI are delegated to existing crates through optional adapters.
   calib/geometry/stereo/ptcloud reuse kornia-rs.
3. Performance and correctness fixes for kornia-rs and oximedia-cv go upstream, written and submitted by the user.
4. Kernels are written clean-room from REA notes, which keeps "MIT OR Apache-2.0".
5. No operation is described as at parity with OpenCV without a matching output and a measured ratio of 1.5x or less.

This supersedes two parts of ADR-0001:
- the scope limit to GPU and document capture;
- the consequence that "oxipix publishes as one crate". It is replaced by a workspace with lockstep versions,
  published in this order: macros, core, modules, facade. Each crate is dry-run in that order in CI, and the public
  paths stay stable.

RM-SPLIT is widened to that staged plan. Upstream-first still applies to `image` and `rxing`.

## Alternatives considered

- **Upstream PRs only (kornia-rs, imageproc).** Rejected: wgpu, opt-in threads and a wasm build are design changes
  their maintainers have not signalled interest in. imageproc's generic API blocks fast paths.
- **Reimplement all of OpenCV.** Rejected: it repeats the failure recorded in ADR-0001, for modules where Rust crates
  are mature.
- **Depend on kornia-rs for imgproc.** Deferred: an operation can be delegated to kornia as a planning decision
  (03 §4). This becomes the default if kornia gains optional threads and wasm.

## Risks

- Scope creep. Mitigated by the per-module decisions and the freeze rule in `03_UPSTREAM_VS_NEW.md`.
- Maintenance cost of a proc macro and of ISA clones (coverage, compile time).
- kornia-rs overtaking oxipix on CPU kernels. That is acceptable: the operation is then routed to kornia.

## Consequences

- RM-SPLIT becomes phase 1 of the roadmap.
- `comparisons/opencv` (ADR-0003) gains golden outputs, medium and large fixtures, and a default-profile run.
- The rust-cv-guide harness should add kornia-rs and oxipix `imgproc` as backends (in the guide repository).
