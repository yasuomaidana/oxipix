# 06: Non-CUDA GPU adapters, WebAssembly and cross-compilation

| | |
|---|---|
| Ticket | OXI-99 (addendum) |
| Date | 2026-11-27 |
| Status | Research finding, revision 3, after EGM rounds 1 and 2 ([05](05_EGM_REVIEW.md#review-of-06-round-1)). The recommendation is **proposed**: it changes no decision in 04 or ADR-0005. The one factual correction to 04 §8 (kornia's wasm build) is already applied. |
| Question | (1) Can kornia-rs gain WebAssembly and a non-CUDA GPU path? (2) Can burn, candle or cubecl free GPU image work from CUDA? (3) Where those projects differ too much from oxipix, what should oxipix adopt or build itself? |
| Constraint (owner) | Cross-compilation and `wasm32` must work, and performance must stay as high as possible. Using and improving other repositories is fine. Nothing may depend only on kornia accepting changes. |
| Machine | Apple M3: integrated GPU with unified memory (CPU and GPU share RAM). macOS, Node 26.13 for wasm, rustc 1.100. One machine: x86, discrete-GPU and browser runs are open items (§6). |

**oxipix today.** oxipix is a pure-Rust image crate on top of `image`. Its optional `gpu` feature already avoids CUDA:
it uses wgpu (Vulkan, Metal, DX12, WebGPU) with two hand-written WGSL kernels, a 3x3 convolution and a perspective
warp. kornia-rs is the CUDA-only one.

## Answer in brief

1. **(1a) kornia on wasm: yes, with a 3-line fix, but slowly** (§2). kornia's speed comes from hand-written NEON and
   x86 intrinsics, and it has no wasm SIMD path. Its box and Gaussian filters run about 16x slower in wasm than
   natively.
2. **(1b) kornia with wgpu: not without a redesign of kornia's storage** that its maintainers have not approved (§1).
   kornia added CubeCL and removed it in July 2026. Its GPU work now targets NVIDIA-only cuda-oxide.
3. **(2) burn, candle, cubecl.**
   - **burn: yes for the device, no for fast image kernels.** It runs on wgpu (Vulkan, Metal, DX12, WebGPU) and builds
     for wasm. But burn-vision's image ops are tensor compositions (its GPU median takes 1.6 s, against 48 ms for
     oxipix's CPU median) or silent CPU fallbacks (erode).
   - **candle: no.** It has no non-CUDA, non-Metal GPU backend.
   - **cubecl: yes, as a kernel language.** On one kernel it gave byte-identical output. It is level with oxipix's
     pooled WGSL, or slightly behind, on round trips, and 1.2x to 1.5x ahead on resident compute through the WGSL
     path the browser uses.
4. **(3) oxipix should finish its own path** (§5):
   - compiler-vectorised safe Rust with `simd128` on the CPU;
   - its own wgpu layer: an async context for the browser, pooled buffers, and a resident `GpuImage`.

   cubecl gets a defined evaluation protocol, run only when a trigger fires (§5.2(d)). The kornia wasm fix goes
   upstream. Everything else kornia-related stays optional.

## Glossary

| Term | Meaning |
|---|---|
| Adapter | Code that lets images from one library run on another library's kernels or device. |
| A1 to A8 | The placements compared in §4. |
| Resident | The image is already on the GPU: the timing covers the kernel launch and the wait for it, with no copies. |
| Round trip | Upload, kernel, and download back to the CPU on every call. |
| Pooled | Buffers and bind groups are created once and reused across calls. |
| simd128 | WebAssembly's 128-bit SIMD extension, turned on with `-C target-feature=+simd128`. All current browsers support it. wasm has no runtime feature detection, so it is a build-time choice. |
| cubecl | A Rust language extension (`#[cube]` functions) by the burn team. One kernel source compiles to WGSL (WebGPU, Vulkan, Metal, DX12 through wgpu), MSL, CUDA, HIP (ROCm) and CPU. |
| cudarc | A Rust binding to the CUDA driver. kornia's device memory and kernel launches go through it. |
| Explicit SIMD | Hand-written intrinsics (`core::arch::aarch64`, `core::arch::x86_64`). Fast where written, and absent on other targets. |
| REA, EGM | Source analysis with `dendrite retro`, and the reviews by agents with no prior context. Both are defined in [00](00_SUMMARY.md). |
| The owner | The oxipix maintainer. Upstream PRs and comments are theirs to post. Nothing in Appendix A has been sent. |

## 1. What the upstreams are doing

### kornia-rs (`f60a856`)

- **[kornia/kornia-rs#1135](https://github.com/kornia/kornia-rs/issues/1135) "GPU Backend Roadmap"** is open.
  - It was opened on 2026-09-11 by a contributor, and still carries the `triage` label ("wait for a maintainer to
    approve").
  - Phase 2 lists "RFC: minimal interface a second backend would need".
  - Phase 3 lists "RFC: Vulkan/wgpu backend scope" and, as a stretch goal, "WebGPU/WASM feasibility notes".
  - Nobody has claimed those items. The comments are about CUDA kernels.
- **kornia added CubeCL, then removed it.**
  - It was added in [#927](https://github.com/kornia/kornia-rs/pull/927) (allocator),
    [#938](https://github.com/kornia/kornia-rs/pull/938) (storage aware of where memory lives) and
    [#946](https://github.com/kornia/kornia-rs/pull/946) (resize kernels), in June 2026.
  - The maintainer removed it in [#990](https://github.com/kornia/kornia-rs/pull/990) on 2026-07-07. The reasons:
    - it was never wired into the released API;
    - it duplicated the cudarc kernels;
    - its allocator called cudarc on the caller's thread, while CubeCL keeps the CUDA context on a worker thread.
  - kornia's resize docs (`src/cuda/resize.rs`) also say "CubeCL exposes no texture or shared-memory API". Shared memory
    exists now (`Shared<E>`, `cubecl-core/src/frontend/container/shared_memory.rs`). Textures are still missing.
- **New kernels go to cuda-oxide**, which compiles Rust to NVIDIA PTX (`docs/cuda-oxide.md`). The maintainer told
  contributors on #1135 to target it.
- **kornia's storage is built around CUDA.**
  - `MemoryResource::as_ptr() -> *mut u8` must be a valid address, and the trait requires `Send + Sync`
    (`kornia-tensor/src/resource.rs`).
  - `MemoryDomain::Device { id: i32 }` cannot say which backend owns the memory.
  - A wgpu buffer has no address, and its types are `!Send` in the browser.
  - In `kornia-imgproc/src`, outside `src/cuda/`, there are 110 `#[cfg(feature = "cuda")]` lines in 28 files. 11 of
    those files route through `pair_residency` → `DeviceExec::run` (13 files outside `src/cuda/` mention it in
    total). The rest are imports and gated types.
  - Across all kornia crates the count is 172 lines in 54 files.
  - `kornia-imgproc/src/cuda` is 24,014 lines.
- **kornia's CPU SIMD.** In `kornia-imgproc/src`, files that import `core::arch::aarch64`: 36. Files that import
  `core::arch::x86`/`x86_64` (SSE/AVX2): 20. Files with `core::arch::wasm32` or `simd128`: 0. One file uses `wide`.

### candle, burn, cubecl

| Project (commit) | GPU backends | wasm | Fit for image kernels |
|---|---|---|---|
| candle (`9c1fcff`) | CUDA, Metal. A wgpu backend is an open experiment ([huggingface/candle#3379](https://github.com/huggingface/candle/pull/3379)). A Vulkan PR (#4096) was closed. | CPU only | A tensor and LLM framework (kornia uses it in `kornia-vlm`). It does not remove CUDA for imgproc. |
| burn (`826be49`, 0.23.0-pre.2) | CUDA, ROCm, wgpu (Vulkan, Metal, DX12, WebGPU), native Metal, CPU, through `burn-cubecl`. `burn-candle` is deprecated. | yes: `burn-vision` with `wgpu` passes `cargo check` for `wasm32-unknown-unknown` (with the `getrandom` line from §2) | `burn-vision` (10.5k lines) has OpenCV names: CCL, morphology, median, `filter2d`, colour, NMS. Only CCL has dedicated GPU kernels (1,055 lines). `filter2d` and median are built from tensor ops: the GPU median stacks one shifted copy of the image per window position, then sorts. Erode, dilate and NMS fall back to the CPU, with a device readback. Measured in §3. |
| cubecl (`da10249`, 0.12.0-pre.2) | wgpu (WGSL; MSL or SPIR-V passthrough), CUDA, HIP, Metal, CPU | builds (§3); not run in a browser | A kernel language. It has shared memory, planes (subgroups) and memory pooling, but no textures or samplers. **MSRV 1.95** (oxipix: 1.89). It uses **wgpu 30, the same major version as oxipix**: one copy in the tree (`cargo tree -d`). It can run on a caller's existing wgpu device (`cubecl_wgpu::init_device(WgpuSetup)`). It is pre-1.0, and kornia's #927 code no longer compiles against it. |

## 2. WebAssembly and cross-compilation (measured)

### Builds

`cargo check --lib` per target ([`results/wasm/cross_targets.txt`](results/wasm/cross_targets.txt)):

| Target | oxipix `imgproc` | oxipix `imgproc,vectorize,mosaic` | oxipix `gpu` | kornia-imgproc (patched) |
|---|---|---|---|---|
| x86_64-unknown-linux-gnu | ok | ok | ok | ok |
| aarch64-unknown-linux-gnu | ok | ok | ok | ok |
| x86_64-pc-windows-msvc | ok | ok | ok | ok |
| aarch64-linux-android | ok | ok | ok | ok |
| wasm32-unknown-unknown | ok | ok | **fails**: a `OnceLock` holds `!Send` wgpu types ([`gpu_wasm32_error.txt`](results/wasm/gpu_wasm32_error.txt)). It builds with wgpu's `fragile-send-sync-non-atomic-wasm` (§3). | ok |

**The kornia wasm fix is 3 lines** ([`kornia-wasm.patch.txt`](results/harness/portability/kornia-wasm.patch.txt)):
- It adds a `getrandom` dependency with the `wasm_js` feature, for `wasm32-unknown-unknown` only, in `kornia-algebra`.
  That is the only crate in `kornia-imgproc`'s tree that uses `rand` outside its tests.
- cubecl and burn need the same line in the crate that consumes them, because `cubecl-environment` pulls in
  `rand 0.10`.

**Rayon does not block wasm.**
- On `wasm32-unknown-unknown` without atomics, kornia's rayon calls ran on the calling thread.
- Native and wasm checksums matched for every op.
- Multithreaded wasm (`wasm-bindgen-rayon`, atomics, COOP/COEP headers) was not tried.

### Speed: the same code, natively and in wasm

Setup:
- the same synthetic 1920x1080 frame, one thread;
- the median of at least 10 samples and 300 ms, after 3 warm-ups;
- a checksum computed inside each timed call, for both libraries;
- the default release profile (fat LTO, no `target-cpu`).

Harness: [`wbench_*`](results/harness/portability/). Raw results: [`results/wasm/`](results/wasm/). Times are in ms.
**Bold** marks the faster wasm-simd128 result of each row.

Native runs used `RAYON_NUM_THREADS=1` (commands in
[`RUN.md.txt`](results/harness/portability/RUN.md.txt)). Each cell is a median, and the MAD is in the raw files.

| Op | kornia native | kornia wasm | kornia wasm simd128 | oxipix native | oxipix wasm | oxipix wasm simd128 | oxipix simd128 / native |
|---|---:|---:|---:|---:|---:|---:|---:|
| box 5x5 RGB | 1.63 | 27.1 | 40.8 (MAD 9.5) | 4.03 | 32.4 | **15.0** | 3.7x |
| Gaussian 5x5 RGB | 1.72 | 26.8 | 29.1 | 3.92 | 33.2 | **14.9** | 3.8x |
| median 5x5 RGB | 709 | 685 | 710 | 48.1 | 795 | **44.5** | 0.93x |
| erode 5x5 gray | 132 | 140 | 132 | 0.59 | 4.93 | **1.09** | 1.8x |
| pyrDown RGB | 3.68 | 6.34 | 4.32 | 1.60 | 9.80 | **2.43** | 1.5x |
| warpAffine RGB | 5.23 | 11.4 | **10.3** | 15.2 | 22.6 | 22.0 | 1.4x |
| Canny | 15.4 | 24.6 | **20.7** | 18.4 | 35.8 | 22.9 | 1.2x |

How to read it:
- **kornia's slow median and erode are known**: 3-channel median and erosion are its 100x-slower rows in
  [02](02_GAP_ROOT_CAUSES.md).
- **Outputs.** The checksum is the sum of the output bytes. Native and wasm sums are identical within each library
  for every op. Across the two libraries:
  - Gaussian, median and erode give equal sums.
  - Warp (0.0004%) and pyrDown (77 units) are close. For warp this holds once each library gets the matrix in its own
    convention: kornia takes the forward map, oxipix the destination-to-source map. An earlier run got this wrong
    (EGM finding 06-C4).
  - Box and Canny differ: borders and gradient norm.
  - Read every row except Gaussian, median and erode for time only.
- **One result is not explained yet**: oxipix's median runs faster in wasm than natively (0.93x, MAD under 1%). It is an
  open item (§6). kornia's simd128 box is noise (MAD 9.5 ms).

What this shows:
- **kornia has no wasm SIMD, and its wasm build loses most where it is fastest natively.** Box and Gaussian go from
  1.6 ms to about 27 ms (about 16x), and simd128 does not change that. Its warp stays 2.1x faster than oxipix's in
  wasm, and 2.9x faster natively.
- **oxipix's compiler-vectorised safe Rust carries over well for median, erode, pyrDown, warp and Canny** (within 1.8x
  of native with simd128).
  - Box and Gaussian lose 3.7x and need work. Explicit `wide` or `core::arch::wasm32` bodies are already allowed by
    [04](04_ARCHITECTURE.md) §5.4.
  - `warpAffine` is slow on every target, and needs the fixed-point rewrite already planned in
    [04](04_ARCHITECTURE.md) §12, phase 2.
- **simd128 is the biggest single lever: 1.0x to 18x for oxipix, and 2x or more on five of seven ops** (median: 795
  against 44.5 ms). A library cannot set it
  for its consumers, so it has to be documented and tested (§5).
- **On x86, nothing was measured.** kornia has AVX2 paths there. oxipix's loops get only SSE2 under the default
  profile unless it uses `multiversion` ([04](04_ARCHITECTURE.md) §5.4).

## 3. GPU (measured; Apple M3, unified memory)

### One kernel, five ways

The kernel is a 3x3 Gaussian on packed RGBA8 with clamped borders, with the semantics of oxipix's
`CONVOLUTION_2D_WGSL` and its truncating CPU reference `gpu::cpu`. [04](04_ARCHITECTURE.md) §7.1 will replace that
reference with the rounding `imgproc` one, so the spike must be re-run then (§6).

Sources:
- [`cubecl_spike.rs.txt`](results/harness/portability/cubecl_spike.rs.txt): CubeCL, 30 lines of `#[cube]` Rust.
- [`wgsl_control.rs.txt`](results/harness/portability/wgsl_control.rs.txt): oxipix's WGSL, pooled, plus a "lean"
  variant that unpacks the way the cubecl kernel does (RGB per tap, alpha once).

How it was measured:
- The variants ran **interleaved, eight rounds**, at a load average of 9 to 30
  ([`interleaved_8rounds_lean.txt`](results/gpu/interleaved_8rounds_lean.txt)).
- An earlier four-round run at a load of about 50 ([`interleaved_4rounds.txt`](results/gpu/interleaved_4rounds.txt))
  inflated the WGSL figures: 4.52 ms resident at 4K, against 2.78 ms here. It is superseded.
- Each cell is the median of the eight round medians, in ms, with the min–max range.
- "Resident" is launch-to-sync time, not GPU timestamp time.
- Parity: every variant matches the CPU reference byte for byte at both sizes
  ([`cubecl_parity_check.txt`](results/gpu/cubecl_parity_check.txt), and `exact=true` in every control round).

| Variant | 1080p resident | 1080p round trip | 4K resident | 4K round trip |
|---|---:|---:|---:|---:|
| oxipix WGSL as shipped (5 new buffers, a bind group and an extra host copy per call; from the gspike-wgsl rounds) | n/a (no resident API) | 3.87 (3.38–4.21) | n/a | 17.4 (15.0–22.4) |
| **oxipix WGSL, pooled** | 0.94 (0.91–1.10) | **2.63** (2.45–2.79) | 2.78 (2.76–2.90) | **9.24** (8.45–10.0) |
| oxipix WGSL, pooled, lean unpacking | 0.93 (0.91–0.97) | 2.65 (2.46–5.49) | 2.74 (2.70–3.00) | 9.67 (8.69–10.6) |
| cubecl, WGSL to Metal | 0.79 (0.69–0.95) | 3.09 (2.66–5.76) | 1.90 (1.86–2.14) | 10.7 (9.65–16.0) |
| cubecl, native MSL | **0.64** (0.55–0.77) | 2.87 (2.51–3.25) | **1.28** (1.19–1.73) | 9.78 (9.07–10.6) |

The CPU reference `gpu::cpu` (not the tuned `filter2d`) takes 28 to 45 ms at 1080p and 112 to 142 ms at 4K across the
runs.

What this shows:
- **Pooling is the first win, and it needs no new dependency.** It cuts the round trip by 32% at 1080p and 47% at 4K.
- **Pooled WGSL wins on round trips** against both cubecl paths, at both sizes.
- **cubecl leads on resident compute.**
  - Through the WGSL path the browser uses: 1.19x at 1080p, 1.46x at 4K.
  - Through native MSL: 1.47x and 2.17x.
  - The lean WGSL variant is no faster than the original. So the lead comes from cubecl's code generation and launch
    path, not from the per-tap unpacking.
  - This is one kernel on one GPU.
- **Copies dominate.** The kernel takes about 1 ms of a 2.6 ms round trip, so chains of resident ops are where a GPU
  pays off. `GpuImage` ([04](04_ARCHITECTURE.md) §7.2) is the first GPU task.
- **Discrete GPUs will look worse.** On a discrete GPU the copies cross PCIe. kornia's GTX 1650 round trips are slower
  than OpenCV's CPU path (`kornia-imgproc/benchmarks.md`).

### burn-vision on the same GPU

Data is f32, NCHW (3 planes, 12 bytes per pixel), on the wgpu device
([`burn_spike.rs.txt`](results/harness/portability/burn_spike.rs.txt)). Times are in ms.

| Size | `filter2d` 3x3 RGB (as "resident" / round trip) | median 5x5 RGB | erode 5x5 gray |
|---|---:|---:|---:|
| 1920x1080 | 4.47 / 2.08 | 1,617 | 3.09 |
| 3840x2160 | 17.5 / 13.3 | 12,584 | 8.90 |

- **The `filter2d` "resident" time is not trustworthy.** It is slower than burn's own round trip. The clone, then
  fusion, then sync sequence probably does not measure what its label says. Only the round trip is comparable: 2.08 ms
  at 1080p, close to pooled WGSL (2.63 ms), but on f32 data and through a general convolution.
- **burn's erode is not a GPU op.**
  - burn-vision's cube backend overrides no float ops (`impl FloatVisionOps for CubeBackend {}`).
  - So `float_erode` reads the tensor back, runs the CPU morphology, and uploads the result (`ops/base.rs`).
  - NMS does the same.
- **burn's median is a GPU op built from tensors.**
  - It takes 1.6 s at 1080p, against 48 ms for oxipix's CPU median (§2).
  - That crosses data types (f32 against u8) and devices, so read it as an order of magnitude, not a ratio.
- **Conclusion.** Outside its CCL kernels (1,055 lines), burn-vision offers no GPU image kernel that oxipix lacks.
  A tensor framework is the right tool for DNN inference, not for image kernels.

### The cost of A2: a kornia image to the GPU and back

([`kornia_adapter.rs.txt`](results/harness/portability/kornia_adapter.rs.txt)). Times are in ms.

| Size (RGB u8) | Host copy (floor) | Upload | Upload, download and wrap into `Image<u8, 3>` | Lossless |
|---|---:|---:|---:|---|
| 1920x1080 | 0.09 | 1.66 | 1.88 | yes |
| 3840x2160 | 0.56 | 5.16 | 6.41 | yes |

A kornia `Image` is a contiguous host slice, so the adapter costs the transfer plus one `Image::new`. Kernels that need
RGBA would add an unpacking pass on top; it was not measured. The transfer cost would be the same for A1.

### wasm GPU

- **oxipix's `gpu` feature builds for wasm with one wgpu feature, and no code change.** Turning on wgpu's
  `fragile-send-sync-non-atomic-wasm` (the same feature cubecl uses) gives a 1.27 MB module. The error it fixes is
  recorded in [`gpu_wasm32_error.txt`](results/wasm/gpu_wasm32_error.txt): `Rc<Cell<u32>>` inside wgpu's
  `WebDevice`, held in the `OnceLock`.
- **Building is not running.** `GpuContext::new` blocks on `pollster::block_on`, and a browser cannot block its main
  thread. The async `new` and `download` from [04](04_ARCHITECTURE.md) §7.2 are still required.
- The cubecl kernel crate also builds for wasm (1.27 MB). Its spike blocks on readback too (`read_one`,
  `block_on(sync)`).
- **Neither was run in a browser**: Node 26 has no WebGPU.

## 4. Where a non-CUDA GPU path can live

| | A1: in kornia | A2: `oxipix-kornia` crate | **A6: oxipix wgpu + WGSL, pooled and resident (the plan in 04 §7)** | A3: oxipix on cubecl | A7: WGSL generated by `oxipix-macros` | A8: rust-gpu | A4: burn-vision | A5: candle |
|---|---|---|---|---|---|---|---|---|
| Needs outside approval | yes: kornia storage redesign; #1135 untriaged | no | no | no | no | no | no | no |
| Toolchain / MSRV | kornia's | 1.89 | **1.89** | 1.95 | 1.89 | pinned nightly (fails the MSRV gate) | burn's | n/a |
| Backends | Vulkan, Metal, DX12, WebGPU, ROCm (proposed) | those of A6 or A3 | Vulkan, Metal, DX12, WebGPU, GL | A6's plus CUDA, HIP, native MSL (upstream claim; Metal only tested) | as A6 | SPIR-V, then naga | as A3 | Metal, CUDA |
| wasm | needs the kornia fix and a `!Send`-tolerant resource | yes | builds with wgpu's `fragile-send-sync-non-atomic-wasm`; running needs an async `new`/`download` and `set_global` (04 §7.2: the context is installed explicitly on wasm) | builds, not yet run | as A6 | as A6 | builds | CPU only |
| Measured speed (M3) | not measured | A6's or A3's own times. The adapter adds only `Image::new` (about 0.1 ms at 1080p), plus RGB-to-RGBA unpacking if a kernel needs it (not measured). A trip costs 1.9 ms on its own, already inside A6's round trip | **1080p: 2.63 ms round trip, 0.94 ms resident; 4K: 9.24 / 2.78 ms** | WGSL path: 1080p 3.09 / 0.79 ms, 4K 10.7 / 1.90 ms (MSL: 2.87 / 0.64, 9.78 / 1.28) | as A6 | not measured | `filter2d` round trip 2.08 ms (f32); erode on the CPU | n/a |
| Textures and samplers | backend-specific | as A6 | **yes** | no (manual gathers) | yes | yes | no | n/a |
| Shares a caller's `wgpu::Device` | n/a | yes | yes | yes (`init_device`) | yes | yes | yes | no |
| Exit cost | theirs | small | n/a (baseline) | kernels must be rewritten unless exported to WGSL | low | high | high | n/a |
| Matches oxipix's approach | no: CUDA-shaped storage, mandatory rayon (oxipix keeps threads opt-in, 04 G2) | yes | **yes** | yes | yes | partly | no: tensor-first | no |

Two more notes:
- **A6 has a cheaper variant than cubecl for hot kernels:** wgpu accepts native MSL or SPIR-V through passthrough
  shader modules (behind an `unsafe` wgpu feature). One hand-written MSL kernel next to its WGSL twin would capture the
  MSL gain on Apple without a new dependency. It is not measured, and it is listed as an option in §5.2(d).
- Shader-composition tools (WESL, naga_oil) are not placements. They can serve A6 and A7 when the kernel count grows.
- The table keeps the unmeasured and rejected placements, because round 1 asked for the alternatives to be shown
  (06-D2).

## 5. Recommendation (proposed)

**Effect on other documents.** This changes no decision in 04 or ADR-0005. Four kinds of edit were made elsewhere:
- 04 §7.3 gained a cross-reference to this section.
- One factual sentence or cell about kornia's wasm build was corrected in 04 §8 and §11, in 00, 01, 03 and in the ADR's
  list of alternatives.
- 03 §4 and the ADR gained a wasm performance condition on the "delegate to kornia" triggers.
- 03 §4 gained the cubecl trigger.

### 5.1 CPU: compiler-vectorised safe Rust, with simd128 as part of the contract

- **Docs.** Add a "WebAssembly" section to the README with the `.cargo/config.toml` snippet:
  `[target.wasm32-unknown-unknown] rustflags = ["-C", "target-feature=+simd128"]`. Scalar wasm builds are supported
  for correctness only.
- **Harness.** Promote the `wbench` harness into the repo as `tools/wbench`: unpublished, outside the default
  workspace build, oxipix ops only (no kornia dependency).
- **CI job.** On one runner:
  - build native, wasm scalar and wasm `+simd128`;
  - pin `wasm-bindgen-cli` to the crate's version (`cargo install wasm-bindgen-cli --version 0.2.130 --locked`);
  - run both wasm builds under Node.

  The job passes when:
  - all three give identical output-byte sums per op, computed in the same job, never committed;
  - the in-job ratios hold: simd128 is at least 5x faster than scalar on median, and at least 1.5x on box.

  Absolute timings run nightly or by hand only, because shared runners are too noisy for them.
- **Gaps to close.**
  - Box and Gaussian: simd128 time at most 2.0x native (today 3.7x), using explicit `wide` or `core::arch::wasm32`
    bodies (04 §5.4).
  - `warpAffine`: the fixed-point rewrite (04 §12, phase 2).
- **x86.** No parity claim is made on x86 until it is measured. Under the default profile x86 gets SSE2 only, unless
  `multiversion` is used.
- **Threads in wasm.** wasm stays single-threaded by design for now, consistent with 04 G2's opt-in threads. Threaded
  wasm needs:
  - atomics, which today means a nightly toolchain with `-Zbuild-std` and so fails the 1.89 MSRV gate;
  - COOP/COEP headers on the hosting page;
  - `wasm-bindgen-rayon`.

  Revisit when a consumer asks, or when wasm atomics are available on stable Rust.

### 5.2 GPU: A6, in this order

**(a) The wasm context.**
- **First acceptance item, now:** a CI job running `cargo check --target wasm32-unknown-unknown --features gpu`
  (04 §9).
- **Build fix.** On wasm, install the context with `set_global` into a `thread_local!`, not a `static OnceLock`. This
  is the 04 §7.2 option that keeps a future atomics build possible. wgpu's `fragile-send-sync-non-atomic-wasm`
  (§3) is an interim build fix at most. If it is used, add
  `compile_error!` for `all(target_feature = "atomics", feature = "gpu")`. Cargo form:
  `[target.'cfg(target_arch = "wasm32")'.dependencies] wgpu = { version = "30.0.1", optional = true, features = ["fragile-send-sync-non-atomic-wasm"] }`.
- **Async API.** Add an async `GpuContext::new(GpuOptions)` and `download`. `map_async` completes through a
  `futures-channel` oneshot.
  - `GpuOptions` holds `power_preference` and `force_fallback_adapter`.
  - On native, keep `new_blocking`, and deprecate today's `new` as an alias for it.
  - The lazy, blocking `global()` becomes native-only.
  - Mark `GpuError` `#[non_exhaustive]` and add `DeviceLost`.
  - Together these are a 0.2.0 change.
- **Browser test.** `wasm-bindgen-test` with `wasm-pack test --headless --chrome`. Chrome's WebGPU flags go in
  `webdriver.json` (`goog:chromeOptions`). The test runs on the M3 first. On Linux CI it needs Chrome's software
  WebGPU adapter, and the exact flags are validated in that first CI run.
  - A missing adapter fails the test; it does not skip.
  - Pass: output bytes equal today's `gpu::cpu::convolve3x3` and `warp_perspective` on Metal.
  - On other adapters (browser on non-Apple GPUs, lavapipe), the maximum absolute difference is at most 1, and the
    count of differing pixels is recorded. f32 accumulation is not bit-stable across vendors.
  - All of this is re-baselined when 04 §7.1 moves the reference to `imgproc`.

**(b) Pooling and `GpuImage`.**
- **Start condition.** Work starts once the phase-3 implementation plan (04 §12) defines:
  - the pool key (size, usage) and its cap;
  - the `GpuImage` op signatures, for example `fn convolve3x3(&self, k: &[f32; 9]) -> Result<GpuImage, GpuError>`.
- **Scope.** One submit per op is in scope. A single command buffer for a whole chain stays `GpuPipeline`, which
  04 §7.3 still gates on OXI-98 measuring medium and large sizes across ops. §3 measured one kernel only.
- **Acceptance.** Measured in the same interleaved run, at least 8 rounds, with the load average recorded, at 1080p and
  4K:
  - the public-API pooled round trip is at most 0.75x the as-shipped path;
  - resident time is within 1.15x of the raw `wgsl_control`;
  - a test-only transfer counter on `GpuContext` shows that a chain of N resident ops does exactly 1 upload,
    N dispatches and 1 readback.

**(c) CI on Mesa lavapipe (04 §9).**
- The job runs on `ubuntu-24.04` with `mesa-vulkan-drivers` and `WGPU_BACKEND=vulkan`, using `GpuOptions` with
  `force_fallback_adapter` as needed.
- With `OXIPIX_REQUIRE_GPU=1`, "no adapter" becomes a failure, and the adapter name is printed.
- Tests: `convolve3x3` and `warp_perspective` against the reference, at 2 sizes, with the tolerance from (a).
- lavapipe runs on the CPU, so it checks correctness only, never speed.

**(d) cubecl: a defined evaluation, run only on a trigger.** 04 §7.3 keeps cubecl as a watch item.
- **Triggers** (any one):
  - a consumer needs CUDA or ROCm;
  - a consumer needs resident throughput at 4K or more, where the measured 1.46x lead matters;
  - oxipix has at least 8 GPU kernels, and WGSL upkeep is measured as the bottleneck.
- **Cheaper first step.** Try a passthrough MSL twin for the hot kernel (§4). If it captures the Apple gain, cubecl may
  not be needed.
- **Protocol.**
  - Port three kernels to both pooled WGSL and cubecl: the 3x3 convolution; a bilinear warp (hand WGSL with
    `textureSampleLevel` against cubecl's buffer gather); and 5x5 erode.
  - Run on the M3 and one discrete GPU, which the owner names when the trigger fires.
  - Interleave at least 8 rounds on an idle machine, at 1080p and 4K.
  - Time both sides with wgpu `TIMESTAMP_QUERY` kernel times, plus launch-to-sync.
  - **The decision path is cubecl's WGSL output**, the one that ships to browsers. MSL and SPIR-V passthrough are
    reported only for information. lavapipe is used for correctness only.
- **Adoption** for a kernel family requires all four:
  - a resident lead of 1.5x or more at both sizes, on both GPUs;
  - a round-trip ratio of at least 0.95;
  - a browser run;
  - the preconditions: an MSRV policy (cubecl needs 1.95), one wgpu in the tree, the checked launch API, and each
    kernel's WGSL output exported and diff-checked in CI as the exit path.

  Adoption can be incremental, because cubecl runs on oxipix's own wgpu device.

### 5.3 kornia: optional, never automatic

- Send the wasm fix (A.1).
- Ask on #1135 whether a non-CUDA backend is in scope (A.3).
- Propose optional rayon (A.2) for single-threaded embedders only. Rayon already runs serially on wasm.
- Build `oxipix-kornia` (A2) only when a consumer ticket asks. It is a separate crate the caller uses explicitly, never
  a routing layer (04 §8). Its tests pin kornia's outputs, not oxipix's.
- Delegating an op to kornia (03 §4, ADR-0005) also requires kornia's `wasm32 +simd128` build to pass the same 1.5x gate
  as oxipix (04 §9). A build that merely compiles is not enough.

### 5.4 DNN

burn or [tract](https://github.com/sonos/tract) adapters only. tract is a pure-Rust ONNX/NNEF inference engine. Candle
stays out of the GPU plan.

## 6. Open items

| Item | Status |
|---|---|
| Browser run of a GPU kernel with async readback | deliverable of 5.2(a); blocks any cubecl adoption |
| lavapipe run of the parity tests | deliverable of 5.2(c) |
| x86_64 and one discrete GPU: CPU table, round-trip cost | blocks any x86 or "discrete GPU" claim |
| Re-run the GPU spike against the rounding `imgproc` reference (04 §7.1) | blocks "byte-exact" for the new specification |
| Explain the faster-in-wasm oxipix median (MAD under 1%) | the median row of the wasm table |
| Threaded wasm | waits for a consumer or for stable wasm atomics (5.1) |

## Appendix A: draft upstream texts (the owner posts them; nothing has been sent)

**A.1 kornia-rs PR: "build: support wasm32-unknown-unknown"**
> `kornia-algebra` pulls `rand 0.10` → `getrandom 0.4`, which refuses `wasm32-unknown-unknown` unless `wasm_js` is
> enabled. This adds a target-specific `getrandom` dependency with `wasm_js` (3 lines; other targets unchanged). With
> it, `kornia-imgproc` builds for wasm32 with and without `+simd128`. Rayon runs on the calling thread there. Box,
> Gaussian, median, erode, pyrDown, `warp_affine` and Canny on a 1920x1080 frame give the same output-byte sums as
> native. It adds a
> CI step: `cargo check --target wasm32-unknown-unknown -p kornia-imgproc`.

**A.2 kornia-rs issue: "Optional rayon (`parallel` feature)"**
> Rayon is mandatory in `kornia-image`, `kornia-tensor`, `kornia-imgproc` and `kornia-3d`. Embedders that own their
> threading cannot turn it off. Proposal: a default-on `parallel` feature, with `src/parallel.rs` as the single shim
> (serial `chunks_mut` when off), and `ndarray/rayon` behind the same feature. First step: count which of the rayon call
> sites already go through the shim.

**A.3 Comment on #1135**
> Is a non-CUDA backend (wgpu: Vulkan, Metal, DX12, WebGPU) in scope for kornia? If it is, the first change would be
> `MemoryDomain` naming its backend, and `MemoryResource` allowing a handle without a raw pointer. Happy to share
> measurements from an Apple M3, if useful.

## Evidence

- **REA analyses** (`dendrite retro analyze --format source-tree`): `kornia-cuda-rea`, `kornia-tensor-rea`,
  `kornia-oxide-rea`, `cubecl-wgpu-rea` and `cubecl-core-rea`.
- **References added** with `dendrite afferent add`: `cubecl`, `burn` and `candle`.
- **Commits**: kornia-rs `f60a856`, cubecl `da10249`, burn `826be49`, candle `9c1fcff`.
- **Upstream threads**: kornia-rs #927, #938, #946, #990 and #1135; candle #3379 and #4096.
