# ADR-0002: Documentation lives in an unpublished package

- Status: Accepted
- Date: 2026-11-24

## Context

The documentation is meant to be as complete as docs.opencv.org: tutorials per module, an API reference per function,
migration guides and performance comparisons. That is far more than a crate README, and it needs tooling (a site
generator, charts, large images) that does not belong in a published crate.

## Decision

The documentation is a separate package in the same repository (`website/`, a Docusaurus site marked
`"private": true`) and is never published to npm or crates.io. Code shown in tutorials lives in an unpublished Rust
package next to it so that it is compiled and tested in CI. The published crate's `include` list excludes the site, the
archive, and everything except sources, examples, benches, README, CHANGELOG and licences.

The crate's own rustdoc (built from `src/` and `README.md`) remains the API documentation on docs.rs.

## Alternatives considered

- **mdBook or rustdoc-only.** Better built-in code checking, but a weaker fit for the OpenCV-style navigation and
  chart-heavy performance pages. Compiled snippets in an unpublished crate recover the code checking.
- **Publish the docs crate.** Rejected: nobody depends on it and it would add a second release to maintain.

## Risks

- A Node toolchain in CI. Mitigated by a separate workflow that only runs when `website/` changes.
- Snippets drifting from the API. Mitigated by importing them from the compiled snippets package.

## Consequences

- See [`../plans/OPENCV_STYLE_DOCS_PLAN.md`](../plans/OPENCV_STYLE_DOCS_PLAN.md) for the structure and phasing.
