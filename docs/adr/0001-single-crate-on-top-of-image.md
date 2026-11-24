# ADR-0001: One crate on top of `image`, with upstream-first CPU work

- Status: Accepted
- Date: 2026-10-27

## Context

oxipix started as a seven-crate workspace meant to replace `image` and `rxing` (its own pixmap type, codecs, SIMD
kernels, operations, barcode reader and compatibility facades). Measured against the real crates the replacements were
partial (the JPEG and PNG decoders were stubs, barcode reading handled only axis-aligned QR codes) and slower than
upstream. The one genuinely unique piece was wgpu acceleration, which `image-rs` is unlikely to accept. The
document-capture algorithms (mosaic, vectorize) had been written inside the downstream project `docula`.

## Decision

oxipix is a single crate that builds on `image::ImageBuffer`. It contains only what cannot live upstream: the opt-in
`gpu` module and the `mosaic` and `vectorize` modules. CPU improvements to `image` and `rxing` are developed on forks,
one branch per topic, each gated on a benchmark win measured in the upstream repository, and submitted upstream by hand
(see [`../FORKS.md`](../FORKS.md)).

## Alternatives considered

- **Keep replacing `image` and `rxing`.** Rejected: large maintenance surface, no measured advantage.
- **Patch `image` and `rxing` through `[patch.crates-io]`.** Rejected: it would make oxipix unpublishable and would tie
  consumers to forks. Nothing patches either crate.
- **Split into several crates immediately.** Deferred (RM-SPLIT in the roadmap): any split is a separate pull request
  and only if justified.

## Risks

- Upstream may decline the CPU topics. They simply stay on the forks and oxipix is unaffected.
- `mosaic` depends on `oximedia-cv`, a young pure-Rust crate; two behaviours of it are recorded in
  [`../01_ARCHITECTURE.md`](../01_ARCHITECTURE.md) §5.

## Consequences

- The workspace crates, compatibility facades and the 569-test parity claim are gone; the old plans live in
  [`../archive/`](../archive/).
- `oxipix` publishes as one crate with no path or git dependencies.
