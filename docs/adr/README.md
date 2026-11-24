# Architecture decision records

One file per decision, numbered, never rewritten: a changed decision gets a new record that supersedes the old one.

Template:

```markdown
# ADR-NNNN: Title

- Status: Proposed | Accepted | Superseded by ADR-NNNN
- Date: YYYY-MM-DD

## Context
## Decision
## Alternatives considered
## Risks
## Consequences
```

| ADR | Decision | Status |
|---|---|---|
| [0001](0001-single-crate-on-top-of-image.md) | One crate on top of `image`, with upstream-first CPU work | Accepted |
| [0002](0002-docs-as-an-unpublished-package.md) | Documentation lives in an unpublished package | Accepted |
| [0003](0003-opencv-comparison-in-its-own-workspace.md) | The `opencv` comparison lives in its own workspace | Accepted |
