# oxipix development status

## Direction (2026-10)

oxipix no longer replaces `image` / `rxing`. It is a single crate holding only what cannot live
upstream: wgpu acceleration and generic document-capture CV (mosaic, vectorize). CPU improvements
go to forks of `image` and `rxing` as opt-in features, then upstream as PRs.

## Done (PR 1)

- [x] Single crate; the seven `oxipix-*` workspace crates and the compat facades are removed.
- [x] `gpu`: persistent wgpu context, both kernels dispatched, CPU fallback, parity tests, benchmark.
- [x] `mosaic` and `vectorize` moved in from docula (`cv-mosaic`, `cv-vectorize`) and given tests.
- [x] Fixed: vectorize used `val < otsu` and produced no paths for pure black-on-white input (Otsu returns 0); now `<=`.

## Next

- [ ] docula migrates to real `image` + `oxipix` (separate docula branch).
- [ ] Fork `image` and `rxing`; first rxing change relaxes the `image = "=0.25.8"` pin.
- [ ] Benchmark-gated fork topics (NEON colour conversion, multi-reader speedup, SIMD binarizer).
- [ ] PR 2, only if justified: split into `oxipix-gpu`, `oxipix-mosaic`, `oxipix-vectorize`.
- [ ] Replace `vectorize::otsu_threshold` with `imageproc::contrast::otsu_level` once parity is verified (conventions differ).
- [ ] Document-capture roadmap: [`docs/16_DOCUMENT_CAPTURE_AND_RECEIPT_IMAGING.md`](docs/16_DOCUMENT_CAPTURE_AND_RECEIPT_IMAGING.md).

## Known limitations

- `mosaic` GPU warp fills uncovered pixels white; the `oximedia-cv` CPU warp edge-clamps.
- The Moore tracer caps each contour at 1000 steps.
