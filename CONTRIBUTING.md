# Contributing to oxipix

You do not need Dendrite to contribute. Fork, branch, change, and open a pull request as usual.

## Checks to run before a pull request

```bash
cargo fmt --all --check
cargo clippy --all-targets --all-features -- -D warnings -W clippy::cognitive_complexity
cargo test --all-features
cargo llvm-cov --fail-under-lines 100      # default features; GPU code is exercised by the feature job
cargo deny check                           # licence and advisory policy, see deny.toml
RUSTDOCFLAGS="-D warnings" cargo doc --all-features --no-deps
```

The structural gates run through [`loom-gate`](https://github.com/yasuomaidana/dendrite-rag-rs) and read their
limits from `.github/dendrite/directives.toml`:

```bash
loom-gate loc && loom-gate clones && loom-gate complexity && loom-gate structure && loom-gate rstest
```

| Gate | Limit |
|---|---|
| Production lines per file | 500 (warning at 450) |
| Duplicated code | 0%: no block of 5 lines / 50 tokens repeated, tests included |
| Cyclomatic complexity per function | 10 |
| Cognitive complexity per function | 15 |
| Line coverage | 100% |
| Files per directory | 12 |

Tests that repeat each other belong in one `#[rstest]` table. See [`docs/02_QUALITY_GATES.md`](docs/02_QUALITY_GATES.md).

## Changing behaviour

`tests/characterization.rs` pins the exact output of the `mosaic` and `vectorize` functions that downstream
projects depend on. A refactor must leave those hashes unchanged. When a change is meant to alter a result, say so in
the pull request and in `CHANGELOG.md`.

## Pull requests to upstream projects

CPU improvements for `image` and `rxing` are developed on forks and submitted upstream by hand; see
[`docs/FORKS.md`](docs/FORKS.md). image-rs requires pull request text to be written by a human.

## Dendrite is optional

The maintainers organise work with Dendrite tickets (`OXI-nn`). It never blocks you when `dendrite` is not installed.

## Licence

By contributing you agree that your work is licensed under `MIT OR Apache-2.0`, like the rest of the crate.
