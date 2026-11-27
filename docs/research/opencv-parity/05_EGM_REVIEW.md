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
