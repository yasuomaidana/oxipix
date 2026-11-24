# 02 — Quality gates

Every gate runs in [`.github/workflows/ci.yml`](../.github/workflows/ci.yml). The limits for the structural gates are
not in the workflow: they are read from [`.github/dendrite/directives.toml`](../.github/dendrite/directives.toml) and
`clippy.toml`, so changing a limit is a reviewed one-line change in one place.

| Gate | Limit | Enforced by | Run locally |
|---|---|---|---|
| Formatting | rustfmt defaults | `cargo fmt` | `cargo fmt --all --check` |
| Lints | `-D warnings`, cognitive complexity 15 | `cargo clippy` + `clippy.toml` | `cargo clippy --all-targets --all-features -- -D warnings -W clippy::cognitive_complexity` |
| File size | 500 production lines (warn at 450) | `loom-gate loc` | `loom-gate loc` |
| Duplication | 0% (5 lines / 50 tokens, tests included) | `loom-gate clones` | `loom-gate clones` |
| Complexity | cyclomatic 10, cognitive 15 per function | `loom-gate complexity` | `loom-gate complexity` |
| Structure | 12 files per directory; `README.md` and `CHANGELOG.md` per package | `loom-gate structure` | `loom-gate structure` |
| Test style | repeated tests become one `#[rstest]` table | `loom-gate rstest` | `loom-gate rstest` |
| Coverage | 100% of lines, default features | `cargo llvm-cov` | `cargo llvm-cov --fail-under-lines 100` |
| Features | each feature builds alone; all-features tests pass | `cargo hack`, `cargo test` | `cargo hack check --each-feature --no-dev-deps` |
| MSRV | `rust-version` in `Cargo.toml` (1.88) | CI job `msrv` | `cargo +1.88 check --all-features` |
| Documentation | rustdoc builds with `-D warnings` | CI job `docs` | `RUSTDOCFLAGS="-D warnings" cargo doc --all-features --no-deps` |
| Supply chain | permissive licences, banned crates, advisories | `cargo deny` + `deny.toml` | `cargo deny check` |
| Packaging | `cargo publish --dry-run --locked` | CI job `package` | same |

`loom-gate` comes from [`dendrite-rag-rs`](https://github.com/yasuomaidana/dendrite-rag-rs) and is installed in CI at a
pinned commit (`LOOM_GATE_REV` in the workflow).

## How the numbers are kept honest

- **Behaviour is pinned, not assumed.** `tests/characterization.rs` hashes the output of every public `mosaic` and
  `vectorize` function on fixed inputs, and `tests/characterization_gpu_cpu.rs` does the same for the CPU kernels. The
  hashes were recorded on the code before the duplication and complexity refactors and re-verified against it, so the
  refactors are behaviour-preserving by construction. A failing test prints a paste-ready table.
- **Coverage is per binary.** llvm-cov's line summary counts a line as missed if any test binary that contains the
  function never executes it. A function exercised only by integration tests therefore also needs a unit test.
  Generic helpers with several instantiations hit the same effect: `map_channels` takes a `&mut dyn FnMut` so that
  there is exactly one instance.
- **GPU code is gated separately.** The default-feature coverage build compiles the `gpu` module out. The CPU
  reference kernels are covered by `tests/characterization_gpu_cpu.rs` and the GPU kernels by the unit tests in
  `gpu/ext.rs`, which run in the `features` job and fall back to the CPU reference on machines without an adapter.

## Changing a limit

Edit `.github/dendrite/directives.toml` (or `clippy.toml`) and say why in the pull request. Do not add per-file
exclusions to get around a gate; split the function or file instead.
