# ADR-0003: The `opencv` comparison lives in its own workspace

- Status: Accepted
- Date: 2026-11-24

## Context

The documentation will compare oxipix with the [`opencv`](https://crates.io/crates/opencv) crate, which binds a
system-installed C++ OpenCV through libclang. Building it requires libopencv and libclang, takes a long time, and is
not available on every target oxipix cares about.

## Decision

The comparison benchmarks are a separate Cargo workspace in `comparisons/opencv/`, excluded from the root workspace and
unpublished. The published crate, its lockfile and the default CI never see `opencv`, libclang or libopencv. A manually
triggered workflow runs the comparison on a runner with `libopencv-dev` installed, and the results are committed as
JSON for the documentation site to render.

## Alternatives considered

- **A dev-dependency of oxipix.** Rejected: every contributor and every CI job would need OpenCV installed, and the
  lockfile would pull in libclang bindings.
- **A cargo feature.** Rejected for the same reason; features are additive and still resolve in the lockfile.

## Risks

- The comparison can go stale. Mitigated by recording the OpenCV and crate versions in every result file.
- Unfair comparisons. Mitigated by checking output parity before timing and reporting conversion costs separately
  (see the plan).

## Consequences

- Performance claims in the README link to committed result files, not to numbers typed by hand.
