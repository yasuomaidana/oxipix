# 05: EGM review of the architecture

The review used the Elephant-Goldfish Model (EGM plugin 0.1.0). The author (the "elephant", who holds the full
context) drafted [04](04_ARCHITECTURE.md) revision 1. Five reviewers (the "goldfish") then read only the documents and
the code they cite, with no memory of the conversation, each with a different focus:

| Reviewer | Focus | Findings (high / medium / low) |
|---|---|---|
| `egm-design` (D) | Architecture and design | 22 (8 / 10 / 4) |
| `egm-zoom-out` (Z) | Simplicity and scope | 13 (3 / 7 / 3) |
| `egm-implementation-readiness` (R) | Can it be built as written | 21 (7 / 11 / 3) |
| `egm-codebase-details` (C) | Accuracy against the code | 19 (4 / 8 / 7) |
| `egm-comprehension` (K) | Naive reader | 27 (6 / 12 / 9) |

The verdicts:
- **Z**: the hybrid decision is sound, but 04 rev 1 was much bigger than the problem.
- **R**: not ready to implement.
- **D**: the direction is sound, but stripes, wasm GPU, the kornia backend and the speed goal do not hold as written.
- **C**: several statements about the existing code were wrong.

Revision 2 of 04, together with the updates to 00, 02, 03 and ADR-0005, resolves the findings below. "Accepted" means
the text changed. "Partly" means it changed in a different way than suggested, for the reason given. "Rejected" means
it did not change, for the reason given.

## Themes raised by several reviewers

| Theme | Raised by | Resolution |
|---|---|---|
| The border defaults change silently while 04 promised "no API change" | D8, Z8, R3, C5 | **Accepted.** Existing functions keep replicate borders and the imageproc radius rule. OpenCV behaviour is reached through `*_into(..., Border)`. A default switch would be a separate, versioned breaking change (04 §4). |
| The crate split contradicts ADR-0001 and is not justified by G2 | D7, Z1, R1, R2, C12, C17 | **Partly.** The request allows sub-crates, so they stay as the end state, but staged with triggers (04 §3). Stage 1 adds only `oxipix-macros`. ADR-0005 now supersedes ADR-0001's single-crate consequence and records the release policy. |
| The phase-2 exit criterion is all-or-nothing and contradicts 03 §4 | D6, Z4, R16 | **Accepted.** An operation meets G1 or gets a recorded delegate/freeze decision. Phase 3 runs alongside phase 2. |
| GPU parity tests pass trivially on CI, because there is no adapter and the code falls back to the CPU | D3, C11, R14 | **Accepted.** A lavapipe CI job; tests call `GpuContext` directly and report "skipped" instead of passing (04 §9). |
| wasm GPU readback must be async; the API is undecided | D2, R6, C10 | **Accepted.** The API split is specified: async `new` and `download`, native-only blocking forms, `set_global` (04 §7.2). |
| Golden hashes cannot express tolerances; the golden source build is not pinned | D16, R15, K17 | **Accepted.** Exact hashes for bit-exact operations; for the others, an output PNG plus a maximum difference and count; each golden records the OpenCV build, HAL and architecture (04 §9). |
| The kornia "light HAL" breaks feature additivity, G2 and wasm | D4, Z5, R20, K16 | **Accepted.** §8 removed; delegation is a planning decision, not runtime routing. |
| The `#[kernel]` macro has too many responsibilities and an unclear contract | Z2, R7, R8, R9, D11, D18, K15 | **Partly.** The request asks for macros, so they stay, but in two stages: `macro_rules!` helpers first, then `#[kernel]` only if a prototype passes the coverage gate. The macro no longer generates ISA clones, benchmarks or GPU registration. The `crate =` path override, `tolerance`, `halo`, and the full channel list including `LumaA` are specified (04 §5). |
| The threshold units are inconsistent | R12, K14 | **Accepted.** One unit, pixel-taps (`min_work`), measured in phase 2 (04 §6). |

## Design (D)

| # | Sev | Finding | Resolution |
|---|---|---|---|
| D1 | H | The stripe model does not fit global or sequential operations | **Accepted.** Three shapes, `row_local`, `map_reduce` and `whole`, with every operation classified (04 §6) |
| D2 | H | wasm readback must be async | **Accepted** (theme) |
| D3 | H | GPU parity test is CPU against CPU on CI | **Accepted** (theme) |
| D4 | H | The kornia backend breaks G2 and additivity | **Accepted** (theme) |
| D5 | H | G1 was measured on one machine with `native` and fat LTO; no x86 baseline; one size | **Accepted.** G1 compares each platform with its own OpenCV build, gates on the default profile, and uses three sizes. Medium and large fixtures and a default-profile run were added (02 §6 and §7). x86 is still to be measured. |
| D6 | H | The phase-2 exit criterion | **Accepted** (theme) |
| D7 | H | The crate split versus ADR-0001 | **Partly** (theme) |
| D8 | H | Border change | **Accepted** (theme) |
| D9 | M | No scratch buffers or halo policy | **Accepted.** `Scratch` arena per worker; halo recomputed; stripe height at least 4 x halo (04 §6) |
| D10 | M | Dispatch from `P` to `const C`; `LumaA` missing | **Accepted.** Sealed `Channels` trait covering 1 to 4 channels (04 §4) |
| D11 | M | Macro path and crate cycle | **Accepted.** `#[oxipix_macros::kernel(crate = ...)]` |
| D12 | M | Cross-crate inlining and `target_feature` | **Accepted.** Kernel helpers are `#[inline]` or generic; default-profile check added (04 §5.4, 02 §7) |
| D13 | M | Coverage of generated instances | **Accepted.** A prototype with its llvm-cov result before stage B; coverage jobs for `parallel` and aarch64 (04 §3, §5.3) |
| D14 | M | `wasm-threads` needs nightly | **Accepted.** Deferred, outside the MSRV gate (04 §6) |
| D15 | M | wasm and mobile builds asserted, not checked | **Accepted.** Checked now: `imgproc`, `vectorize` and `mosaic` build for wasm32 and `gpu` does not. The CI matrix is in phase 0. |
| D16 | M | Golden outputs and performance gate | **Accepted** (theme); weekly scheduled performance run added |
| D17 | M | `GpuPipeline` ownership and pixel formats | **Accepted.** Pipeline deferred until OXI-98 measures transfer cost; operation trait in `gpu`; packed u32 layouts (04 §7.2, §7.3) |
| D18 | M | G4 overstated ("written once") | **Accepted.** Restated: one body plus one reference, extras only where measured (04 §1) |
| D19 | L | Nested-parallelism guard versus rayon work stealing | **Accepted.** Guard removed; `ThreadPool::install` respected |
| D20 | L | `Border::Constant(T)`, morphology default, `Transparent`, small images | **Accepted** (04 §4) |
| D21 | L | Clean-room safeguard was weak | **Accepted.** Separation required; the implementing session has no access to the source or its index; recorded in the PR (04 §10) |
| D22 | L | Build-time budget, API policy, FAST location, core topology | **Accepted.** Build-time budget 120%; `*_into` returns `Result`; FAST stays in `imgproc`; 4 and 8 threads reported |

## Zoom-out (Z)

| # | Sev | Finding | Resolution |
|---|---|---|---|
| Z1 | H | Keep a single crate | **Partly** (theme): staged split; the end state is kept because the request asks for opt-in sub-crates |
| Z2 | H | Drop `#[kernel]` | **Partly** (theme): gated behind a prototype; `macro_rules!` first |
| Z3 | H | Remove video and the features crate; narrow photo | **Partly.** Features stay in `imgproc`. Photo is narrowed to RM-ENHANCE. Video stays on the roadmap, because the request covers all OpenCV modules, but only once a ticket with a consumer exists. |
| Z4 | M | Limit G1 to the document-capture operations | **Rejected** for the scope, because the request's rule is "replicate OpenCV functionalities with similar or better performance". **Accepted** for the exit criterion (theme). |
| Z5 | M | Delete the light HAL | **Accepted** (theme) |
| Z6 | M | Custom ISA clones are speculative | **Accepted.** One safe body; `wide` only where measured; `multiversion` for x86 |
| Z7 | M | Narrow the GPU plan; drop cubecl | **Accepted.** Phase 3 is the context, the wasm build and `GpuImage`; pipeline gated on OXI-98; cubecl is a watch item |
| Z8 | M | Border defaults | **Accepted** (theme); the enum keeps 4 variants, `Transparent` for warps |
| Z9 | M | `kornia`, `ndarray` and `wasm-threads` features are speculative | **Accepted.** Deferred until a consumer asks |
| Z10 | M | 00, 03 and 04 disagree (video LK, objdetect, stitching, missing links) | **Accepted.** 00 now matches 03; stitching is marked unchanged; ADR-0005 and 05 exist |
| Z11 | L | Benchmark registration couples the crate to ADR-0003's workspace | **Accepted.** Hand-written list |
| Z12 | L | Too much implementation detail | **Partly.** Mechanics moved to the interfaces; REA commands moved to `docs/rea/README.md` (phase 1). The `#[kernel]` example stays, because R7 asked for more detail, not less. |
| Z13 | L | Risks out of date | **Accepted** (04 §13) |

## Implementation readiness (R)

| # | Sev | Finding | Resolution |
|---|---|---|---|
| R1 | H | ADR-0005 and 05 missing; ADR-0001 contradicted | **Accepted** (both written; ADR-0005 supersedes the consequence) |
| R2 | H | Packaging and release mechanics of the split | **Accepted.** Lockstep versions, publish order, per-crate dry runs, compile test for public paths (04 §3) |
| R3 | H | Border change | **Accepted** (theme) |
| R4 | H | The stripe entry point needs `&mut` aliasing | **Accepted.** `ViewMut::split_rows` gives disjoint stripes (04 §4) |
| R5 | H | `View` invariants undefined | **Accepted.** Stride in samples, minimum length, `Send`/`Sync`/`Copy`, `ViewError` (04 §4) |
| R6 | H | wasm GPU API | **Accepted** (theme) |
| R7 | H | `#[kernel]` contract | **Accepted.** Grammar, `C` injected, reference signature, `tolerance`, `LumaA` (04 §5.3); a before/after expansion is part of the prototype PR |
| R8 | M | Macro paths | **Accepted** |
| R9 | M | Mechanisms behind the generated items (registry, RNG, forced ISA, inlining) | **Accepted.** No registry; the existing `noise` with a fixed seed; no ISA clones in the macro; `#[inline]` rule |
| R10 | M | Proc macro versus the coverage gate | **Accepted.** `proc_macro2` core with unit and snapshot tests; aarch64 and `parallel` coverage jobs |
| R11 | M | `wide` and WGSL bodies versus the zero-duplication gate | **Partly.** `wide` bodies exist only where measured and share helpers. How loom-gate treats WGSL is decided in phase 3 through a directive change, never an ad-hoc exclusion. |
| R12 | M | Threshold units | **Accepted** (theme) |
| R13 | M | `wasm-threads` and cargo-hack | **Accepted** (deferred) |
| R14 | M | GPU layout, `GpuPipeline` API, adapter on CI | **Accepted** (04 §7, §9) |
| R15 | M | Golden outputs | **Accepted** (theme) |
| R16 | M | Performance gates not measurable | **Accepted.** Ratio definition, sizes, profiles, machine; the x86 machine is still to be chosen |
| R17 | M | Phase 1 too big | **Accepted.** Three ordered PRs with exits (04 §12) |
| R18 | M | Phase 0 not actionable | **Accepted.** Each probe fix is listed |
| R19 | L | Path of `features`, the mosaic edge, the `image` feature | **Accepted** (04 §3, §4) |
| R20 | L | Light-HAL registration | **Accepted** (removed) |
| R21 | L | Build budget, wgpu pin | **Accepted.** 120% budget; the lockfile pins wgpu |

## Codebase details (C)

| # | Sev | Finding | Resolution |
|---|---|---|---|
| C1 | H | Median already splits channels; the real costs are the 234-exchange network, no reuse of sorted columns, and copies | **Accepted.** 02 §3, 00 and 04 phase 2 corrected |
| C2 | H | Only median (plus `map_channels` and `warp_cpu`) is per channel; bilateral cannot be | **Accepted** (04 §5.1) |
| C3 | H | The separable shapes were described wrongly; the spike is not in `src`; private SSIM blur | **Accepted** (04 §5.1, 02 §3) |
| C4 | H | `gpu::cpu` is a second specification that disagrees with imgproc | **Accepted.** 04 §7.1 makes the imgproc kernel the reference and widens RM-WARP |
| C5 | M | Border defaults | **Accepted** (theme) |
| C6 | M | The Canny note: both sides use L2; the real differences are the frame and the border | **Accepted** (02 §2) |
| C7 | M | Parity list wrong (pyrDown, Gaussian) and four rows with digest gaps not mentioned | **Accepted.** 02 §2 corrected; those rows are marked not like for like |
| C8 | M | The cause of the `warpAffine` difference | **Accepted** (02 §2) |
| C9 | M | The mosaic dependency edges; Otsu duplicated | **Accepted.** 04 §3 edges fixed; one Otsu in `core` |
| C10 | M | Blocking readback and buffers created on every call | **Accepted** (04 §7.2) |
| C11 | M | Silent GPU fallback | **Accepted** (theme) |
| C12 | M | Quality-gate interactions of the split | **Accepted** (04 §3) |
| C13 | L | Constant channel counts are not new | **Accepted.** 04 §5.3 no longer claims it as a benefit |
| C14 | L | Extend the existing golden and test helpers | **Accepted** (04 §5.1, §9) |
| C15 | L | No ORB; the module path is `imgproc::detect::features` | **Accepted** |
| C16 | L | The `image` feature on core | **Accepted.** `image` is required |
| C17 | L | Header: missing link, OXI-96, what ADR-0005 supersedes | **Accepted.** OXI-96 was merged inside PR #6 (its PR #7 targeted the OXI-95 branch), and the text now says so |
| C18 | L | 8-thread Gaussian (1.82x) missing; 1.49x should be 1.48x | **Accepted** |
| C19 | L | Cite the morph allocations and the pyrDown columns | **Accepted** (02 §3) |

## Comprehension (K)

| # | Sev | Finding | Resolution |
|---|---|---|---|
| K1 | H | REA, dendrite and EGM undefined | **Accepted.** Glossaries in 00 and 04 |
| K2 | H | Decided and proposed are blurred | **Accepted.** "Proposed (pending ADR-0005)" headers and wording |
| K3 | H | The headline mixes oxipix-only and best-crate evidence | **Accepted.** Separate counts in the 00 findings table; a "crate" column in the slower-operations table |
| K4 | H | "The guide" not named | **Accepted.** Named, linked and defined |
| K5 | H | Evidence section not reproducible | **Partly.** IDs and the commands are explained. dendrite is not public, so the REA ids can be reproduced only with it; the benchmarks can be reproduced without it. |
| K6 | H | `bouton`/agy undefined; the separation was optional | **Accepted.** Separation is required and described without tool names (04 §10) |
| K7 | M | RM-SPLIT and RM-ENHANCE undefined | **Accepted** |
| K8 | M | HAL, KleidiCV, carotene and T-API undefined | **Accepted** (glossaries) |
| K9 | M | Stripe and band | **Accepted.** One term, with the OXI-97 synonym noted |
| K10 | M | Ticket references opaque; OXI-96 | **Accepted** |
| K11 | M | Probe and the phase-0 items undefined | **Accepted** |
| K12 | M | "Sovereign workspace" unexplained; 16 versus 17 modules | **Accepted** |
| K13 | M | Stitching in scope with no crate | **Accepted** (unchanged `mosaic`) |
| K14 | M | Threshold units | **Accepted** (theme) |
| K15 | M | ISA dispatch description incomplete; `C` undeclared | **Accepted** (clones removed from the macro; `C` declared) |
| K16 | M | Kornia backend conformance | **Accepted** (removed) |
| K17 | M | Hashes and tolerances | **Accepted** (theme) |
| K18 | M | The spike and the "same mean" proxy | **Accepted.** Called a cheap check, not a full comparison |
| K19 | L | G1's lower bound | **Accepted.** "At most 1.5x" |
| K20 | L | Feature forwarding; Lucas-Kanade | **Accepted** |
| K21 | L | `Border::Ignore` | **Accepted** (replaced by `Constant` with the neutral element) |
| K22 | L | "No compile time" overclaim | **Accepted** |
| K23 | L | "Document sizes" | **Accepted.** Three explicit sizes |
| K24 | L | Freeze rule undefined | **Accepted.** "A module enters only with a ticket and a consumer" |
| K25 | L | "Every imgproc row" | **Accepted.** G1 and 02's tables |
| K26 | L | Harness terms | **Accepted** (00 evidence section) |
| K27 | L | Mixed list | **Accepted** |

## Still open after revision 2

- No x86_64 measurement yet: an AVX2 reference machine is needed for G1.
- The PSNR row in the default profile is suspect: the call is probably optimised away (02 §7).
- How loom-gate treats WGSL and `wide` bodies (R11) is decided in phase 3.
- A second EGM review round on revision 2, if ADR-0005 goes ahead.

## Review of 06 (round 1)

[06](06_PORTABILITY_AND_GPU_BACKENDS.md) (non-CUDA GPU adapters, wasm, cross-compilation) went through the same five
reviewers. Finding ids carry a `06-` prefix and keep the letter scheme above.

| Reviewer | Findings (high / medium / low) |
|---|---|
| `egm-design` (06-D) | 15 (6 / 7 / 2) |
| `egm-zoom-out` (06-Z) | 12 (3 / 6 / 3) |
| `egm-implementation-readiness` (06-R) | 12 (8 / 3 / 1) |
| `egm-codebase-details` (06-C) | 15 (4 / 6 / 5) |
| `egm-comprehension` (06-K) | 12 (3 / 8 / 1) |

**The verdicts.**
- D, Z and R: the CPU and wasm half holds. The GPU recommendation, cubecl as the kernel layer, rested on a comparison
  with **no control**: hand-written WGSL with images kept on the GPU and buffers reused was never measured.
- C: every table number matches the raw files. But two harness errors made rows invalid:
  - the warp matrix convention;
  - burn's "GPU" erode, which actually runs on the CPU.

**What changed.** Revision 2 of 06 re-measured, and the recommendation changed as a result:
- The control was run (`results/harness/portability/wgsl_control.rs.txt`), and all GPU variants were re-run
  interleaved, for four rounds.
- Pooled WGSL ties cubecl on round trips. cubecl leads only on resident compute at 4K.
- **cubecl went from "adopt" to "scheduled three-kernel evaluation"** (made trigger-based in round 2). 04 §7.3 stays unchanged, and so does
  ADR-0005.

| Theme | Raised by | Resolution |
|---|---|---|
| No control for cubecl: pooled, resident WGSL was never measured | 06-Z1, 06-D1, 06-R6 | **Accepted, measured.** Pooled WGSL: 2.68 ms round trip and 1.09 ms resident at 1080p. cubecl: 2.85 to 3.19 and 0.61 to 1.02 ms. At 4K cubecl is 1.9x to 3.2x ahead resident and level on round trips. The recommendation is now A6 first, then a cubecl evaluation (06 §5.2(d)). |
| 06 reversed 04 §7.3 and §8 without saying so | 06-Z2, 06-D5, 06-R1 | **Accepted.** 06 no longer reverses anything. 04 §7.3 stands. 04 §8 changes only in the sentence about kornia's wasm build. The kornia adapter is caller-chosen and gated on a consumer. |
| "Removes the CUDA dependency" answers a question oxipix does not have | 06-Z3, 06-K2 | **Accepted.** 06 opens with what oxipix's `gpu` feature is today (wgpu, no CUDA). cubecl's possible benefit is restated as one kernel source and resident throughput. |
| Missing alternatives: hand WGSL with pooling, macro-generated WGSL, rust-gpu, WESL/naga_oil | 06-D2 | **Accepted.** A6, A7 and A8 were added to §4, with rows for toolchain, textures, device sharing and exit cost. WESL and naga_oil are listed as tooling, not placements. |
| cubecl has no textures, yet warps and resize want them | 06-D3 | **Accepted.** There is a texture row in §4, and the bilinear warp is one of the three evaluation kernels (texture against gather). |
| The browser gate must come first; the spike blocks on readback | 06-D4, 06-Z9, 06-R7 | **Accepted.** Step (a) is first. It found that `gpu` builds for wasm with wgpu's `fragile-send-sync-non-atomic-wasm`, while the blocking `new`/`download` still has to become async. The harness and pass condition are named. |
| The byte-exact claim checks against the truncating reference that 04 §7.1 retires | 06-D6, 06-R2 | **Partly.** The claim is now scoped to "the current `gpu::cpu` reference". Re-running against the rounding reference is an open item. Integer accumulation is left to phase 3. |
| kornia falls back to scalar on x86 | 06-C3, 06-Z6, 06-D10 | **Accepted. It was wrong.** kornia has x86 SSE/AVX2 paths (20 files in `kornia-imgproc`, recounted in round 2) as well as NEON (36). The text now says only wasm lacks SIMD in kornia, and drops "by construction". |
| warpAffine compared with mismatched matrix conventions | 06-C4 | **Accepted, re-measured.** kornia now receives the inverse matrix, and the checksums agree (780,814,742 against 780,817,719). kornia's warp is 2.9x faster natively and 2.1x in wasm. It is no longer listed as something to route to kornia. |
| burn's erode is a CPU fallback with readback, and burn's own `filter2d` "resident" is slower than its round trip | 06-C1, 06-C2 | **Accepted.** Erode is relabelled as a CPU fallback. `filter2d` reports both numbers, and the resident one is marked untrustworthy. The burn figure was removed from the upstream text. |
| Mixed baselines (f32 against u8, GPU against CPU) behind the 35x and 5.6x figures | 06-C8, 06-D12, 06-Z11, 06-K9 | **Accepted.** These are stated as orders of magnitude, with the data type and device named. |
| Single runs; no spread; not interleaved | 06-C7, 06-R12 | **Accepted.** The GPU variants ran interleaved for four rounds, with ranges. MAD was added to the wasm harness. The load caveat (about 50) is stated. |
| The wasm harness thread count was not recorded | 06-C9 | **Accepted.** `RAYON_NUM_THREADS=1` and all commands are in `RUN.md.txt`. |
| simd128 has no delivery plan | 06-Z5, 06-D11 | **Accepted.** 06 §5.1 adds docs and a CI job that runs the harness under Node with `+simd128`. Scalar wasm builds are supported for correctness only. |
| MSRV, pinning, wgpu coexistence and exit path for cubecl | 06-R3, 06-R4, 06-R5, 06-D7, 06-D8, 06-D14 | **Partly.** Measured: one wgpu (30.0.2) in the tree, and cubecl can adopt oxipix's device (`init_device(WgpuSetup)`). The MSRV mechanism and the pin are deferred to the evaluation's outcome, because cubecl is no longer being adopted now. The exit path is a WGSL snapshot per kernel. |
| `GpuImage` and pooling lack acceptance numbers | 06-R6 | **Accepted.** At most 2.8 ms round trip and 1.1 ms resident at 1080p on the M3, and the chain-cost rule. Struct and pool design stay in 04 §7.2 and phase 3. |
| lavapipe and CI for GPU parity | 06-R8, 06-Z8, 06-D13 | **Accepted** as step (c), and as an evaluation GPU in (d). The lavapipe run is an open item. |
| Upstream drafts out of proportion | 06-Z10 | **Accepted.** A.3 is cut to a one-line scope question. A.2 is justified only by embedders. |
| References and terms: appendix B, A4 and A5, EGM/REA, tract, the owner, oxipix context | 06-K1, 06-K5, 06-K6, 06-K12, 06-Z12 | **Accepted.** |
| Answer the three questions up front | 06-K4 | **Accepted** ("Answer in brief"). |
| Count and wording precision: the 110 cfg sites, 5 buffers, 1,055 CCL lines, 16x, pyrDown differs | 06-C5, 06-C6, 06-C10, 06-C11, 06-C12 | **Accepted.** |
| Raw stderr for the wasm `gpu` failure, manifests, timing method per spike | 06-C13, 06-C14 | **Accepted** (`gpu_wasm32_error.txt`, `manifests.toml.txt`, `RUN.md.txt`). |
| "Kernel time" versus launch-to-sync | 06-C15 | **Accepted.** |
| `launch_unchecked` in the spike | 06-D15 | **Accepted** as a precondition: production code uses the checked launch only. |
| Fallback and error mapping for cubecl errors | 06-R9 | **Deferred.** It applies only if cubecl is adopted after (d). |
| kornia wasm patch tracking and the A2 adapter's shape | 06-R10, 06-R11, 06-D5, 06-Z4, 06-K3 | **Accepted.** A2 is a separate crate the caller uses explicitly, gated on a consumer ticket. The kornia PR is the owner's to post. oxipix does not need it, since oxipix's wasm path does not use kornia. |
| Rayon as a reason A1 does not match oxipix | 06-K10 | **Accepted.** Threads stay opt-in in oxipix (04 G2). |
| Ratios in the "carries over" claim | 06-K7, 06-K8, 06-D9 | **Accepted.** There is a per-op ratio column, and box and Gaussian (3.7x) are named as gaps. |
| `OnceLock` fix versus cubecl | 06-K11 | **Accepted.** It is fixed by a wgpu feature, independent of cubecl. |

## Review of 06 (round 2)

Round 2 ran the same five reviewers on revision 2 of 06, and on the lines it changed in 00, 01, 03, 04 and ADR-0005.
They checked whether the round-1 resolutions held. Ids carry a `06r2-` prefix.

| Reviewer | Findings (high / medium / low) | Verdict |
|---|---|---|
| `egm-design` (06r2-D) | 8 (1 / 4 / 3) | The A6-first order is sound. The cubecl adoption rule could not be evaluated as written. |
| `egm-zoom-out` (06r2-Z) | 8 (0 / 4 / 4) | The simplest path, except that the cubecl evaluation was "scheduled" without a consumer. |
| `egm-implementation-readiness` (06r2-R) | 8 (0 / 6 / 2) | (a) and (c) can start. (b) needs a phase-3 plan first, and its thresholds sat inside the noise. |
| `egm-codebase-details` (06r2-C) | 10 (0 / 6 / 4) | The wasm table, the counts and the burn facts all check out. Some GPU cells mixed runs. The x86 file count was wrong. |
| `egm-comprehension` (06r2-K) | 8 (0 / 6 / 2) | Readable end to end. Summary statements drifted from the tables. |

**New measurement in round 2.**
- 06r2-D5 asked whether cubecl's lead was only its leaner per-tap unpacking. A "lean" WGSL variant was added to the
  control, and all variants re-ran interleaved for **eight rounds at a lower load** (9 to 30).
- Result: the lean WGSL is no faster. Through WGSL, cubecl's resident lead is 1.19x at 1080p and 1.46x at 4K. The
  round-1 figure of 1.9x at 4K was load-inflated.
- Pooled WGSL wins on round trips at both sizes.
- The missing cubecl parity lines were captured: byte-exact, for every variant and size.

| Theme | Raised by | Resolution |
|---|---|---|
| The cubecl adoption rule is ill-posed: which code path, lavapipe as a "GPU", a discrete GPU nobody has, no sizes, no tolerance, launch-to-sync across frameworks | 06r2-D1, 06r2-K6 | **Accepted.** The decision path is cubecl's WGSL output (MSL and SPIR-V for information only). lavapipe is for correctness only. The GPUs are the M3 plus a discrete GPU the owner names when a trigger fires. The lead is required at 1080p and 4K, with `TIMESTAMP_QUERY` kernel times, at least 8 interleaved rounds on an idle machine, and a round-trip ratio of at least 0.95. |
| The evaluation was "scheduled" with no consumer (YAGNI) | 06r2-Z1 | **Accepted.** It is now triggered: a consumer needs CUDA or ROCm, or a consumer needs resident throughput at 4K or more, or oxipix reaches 8 or more GPU kernels with WGSL upkeep as the bottleneck. 00, 03, 04 and the ADR now say "triggered". |
| Is cubecl's lead only the unpacking? Is there a cheaper A6 variant? | 06r2-D5 | **Accepted, measured.** The unpacking is not the cause (see above). A6 with a passthrough MSL/SPIR-V twin is listed as the first step once a trigger fires. |
| `fragile-send-sync-non-atomic-wasm` rules out threaded wasm | 06r2-D2, 06r2-R3 | **Accepted.** `set_global` uses a `thread_local!` on wasm. The feature is an interim build fix at most, guarded by a `compile_error!` under atomics. 04 §7.2 was updated to match. |
| The acceptance thresholds for (b) sit inside the noise | 06r2-D3, 06r2-Z7, 06r2-R4 | **Accepted.** The thresholds are now relative within one interleaved run: round trip at most 0.75x as shipped, resident within 1.15x of raw WGSL, at 1080p and 4K, with at least 8 rounds and the load recorded. The chain rule is checked with a transfer counter. |
| Documents still contradicted each other on kornia wasm, and on what 06 changes | 06r2-D4, 06r2-Z5, 06r2-C6, 06r2-R8, 06r2-C9 | **Accepted.** 00 finding 2, 03 R5, 04 §11 (ISA and wasm cells) and the ADR's alternatives were corrected. 06 §5 lists every edit made elsewhere. |
| The kornia revisit triggers need a wasm performance condition | 06r2-Z3 | **Accepted** in 03 §4, ADR-0005 and 06 §5.3. kornia's `wasm32 +simd128` build must pass the same 1.5x gate. |
| The answers did not map onto the questions; burn's answer was blurred | 06r2-Z2, 06r2-K3 | **Accepted.** The answers are numbered 1a, 1b, 2 and 3. burn is "yes for the device, no for fast image kernels". |
| No position on threaded wasm | 06r2-Z4 | **Accepted.** Single-threaded by design for now. The cost is named (nightly `-Zbuild-std`, COOP/COEP). Revisit on consumer demand or once atomics are on stable. |
| The CPU CI job was underspecified (harness location, checksum source, noisy timing, CLI pin) | 06r2-R1, 06r2-Z6 | **Accepted.** The harness moves to `tools/wbench` (oxipix only). Checksums are computed in-job. Timing is checked only as in-job ratios. `wasm-bindgen-cli` is pinned. The README gets a "WebAssembly" section. The box and Gaussian target is at most 2.0x. |
| Which reference, and what tolerance, for the GPU pass conditions | 06r2-R2 | **Accepted.** Today's `gpu::cpu`, re-baselined after 04 §7.1. Exact on Metal; elsewhere a maximum absolute difference of at most 1, with the differing-pixel count recorded. |
| wasm mechanics: harness, oneshot, `global()`, Cargo form, circular open items | 06r2-R3, 06r2-D6 | **Accepted.** `wasm-bindgen-test` with `webdriver.json`. A missing adapter fails the test. `futures-channel` oneshot. `global()` native-only. The Cargo snippet is given. The open items are reworded as deliverables. |
| (b) needs a design before it can start; its relation to `GpuPipeline` and OXI-98 | 06r2-R5, 06r2-D8 | **Accepted.** (b) starts after the phase-3 plan defines the pool key and cap and the op signatures. One submit per op is in scope. `GpuPipeline` stays gated on OXI-98. |
| The lavapipe job is underspecified | 06r2-R6 | **Accepted.** `ubuntu-24.04`, `mesa-vulkan-drivers`, `WGPU_BACKEND=vulkan`, and `OXIPIX_REQUIRE_GPU=1` turns a skip into a failure. The tests are named. |
| Public API breaks (`new` becomes async, `GpuError` grows) | 06r2-R7 | **Accepted.** 0.2.0, `#[non_exhaustive]`, a deprecated `new` alias on native, and minimal `GpuOptions` fields. |
| Warp and pyrDown wording; A.1 "byte for byte" overstated equal sums | 06r2-C1 | **Accepted.** The groups are equal, close and different. A.1 says "same output-byte sums". |
| The "as shipped" row mixed runs; the CPU row format; parity lines missing for cubecl | 06r2-C2, 06r2-C7, 06r2-C3 | **Accepted.** Superseded by the eight-round table, whose sources are stated. The parity lines were captured (`cubecl_parity_check.txt`). |
| The simd128 lever range; 16x to 17x; 47 ms | 06r2-C4, 06r2-K4, 06r2-C8, 06r2-K8 | **Accepted.** "1.0x to 18x, at least 2x on five of seven". "About 16x". 48 ms. |
| The x86 file count (35) could not be reproduced | 06r2-C5 | **Accepted.** Recounted with one pattern in `kornia-imgproc/src`: aarch64 36, x86 20, wasm32 0. Fixed in 06, 01 and 05. |
| The routing-file overlap | 06r2-C10 | **Accepted.** 11 of the 28 cfg files (13 files outside `src/cuda` in total). The 24,014-line scope is stated. |
| The A2 cost cell read as double counting | 06r2-K5 | **Accepted.** The adapter adds only `Image::new`, plus any unpacking. The transfer is already inside A6's round trip. |
| tract undefined; `set_global` undefined; stale MAD item; Status line | 06r2-K1, 06r2-K7, 06r2-K2, 06r2-D7, 06r2-K8 | **Accepted.** |
| Fold the §4 table to the measured columns | 06r2-Z8 | **Rejected.** Round 1 (06-D2) asked for the alternatives to be shown with their reasons. The table keeps them, and a note says why. |

**Still open after round 2:**
- the browser run;
- the lavapipe run;
- x86 and a discrete GPU;
- a re-run against the rounding reference;
- the faster-in-wasm median.

All are listed in 06 §6. A third round is not planned: round 2 found one high-severity issue, and it is resolved.
