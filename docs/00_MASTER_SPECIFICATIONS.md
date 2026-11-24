# 00 — Master specifications

The index of every document in `docs/`. A document is **Current**, **Plan** (agreed, not built) or **Archived**
(historical, kept for the record). When a document changes status, change it here in the same pull request.

| ID | Document | Domain | Status |
|---|---|---|---|
| OXI-SPEC-01 | [`01_ARCHITECTURE.md`](01_ARCHITECTURE.md) | Design of the single crate, the GPU path, behaviours to know about | Current |
| OXI-SPEC-02 | [`02_QUALITY_GATES.md`](02_QUALITY_GATES.md) | Every CI gate, its limit and how to run it | Current |
| OXI-SPEC-03 | [`03_ROADMAP.md`](03_ROADMAP.md) | Done, open and planned work; the `layout` requirements | Current |
| OXI-SPEC-04 | [`04_BENCHMARK_METHODOLOGY.md`](04_BENCHMARK_METHODOLOGY.md) | How performance numbers are measured and compared | Current |
| OXI-FORKS | [`FORKS.md`](FORKS.md) | Runbook for the `image` and `rxing` forks and their upstream pull requests | Current |
| OXI-ADR-0001 | [`adr/0001-single-crate-on-top-of-image.md`](adr/0001-single-crate-on-top-of-image.md) | One crate on top of `image`, upstream-first CPU work | Accepted |
| OXI-ADR-0002 | [`adr/0002-docs-as-an-unpublished-package.md`](adr/0002-docs-as-an-unpublished-package.md) | Documentation lives in an unpublished package | Accepted |
| OXI-ADR-0003 | [`adr/0003-opencv-comparison-in-its-own-workspace.md`](adr/0003-opencv-comparison-in-its-own-workspace.md) | The `opencv` comparison lives in its own workspace | Accepted |
| OXI-PLAN-01 | [`plans/OPENCV_STYLE_DOCS_PLAN.md`](plans/OPENCV_STYLE_DOCS_PLAN.md) | OpenCV-style documentation site and performance comparison | Plan |
| OXI-ARCH | [`archive/`](archive/) | The original seven-crate replacement plan, guides 00 to 16 | Archived |

Outside `docs/`: [`../README.md`](../README.md) (front page, also the crate's rustdoc), [`../CHANGELOG.md`](../CHANGELOG.md),
[`../CONTRIBUTING.md`](../CONTRIBUTING.md), [`../DEV_STATUS.md`](../DEV_STATUS.md).

## Glossary

Each term is defined once, here.

| Term | Meaning |
|---|---|
| ZMAD | Zero-mean absolute difference: the mean of `|(a - mean_a) - (b - mean_b)|` over sampled pixel pairs; smaller is a better overlap |
| ECC | Enhanced correlation coefficient: an iterative image-alignment method |
| Moore-neighbour tracing | Walking the boundary of a set region by probing the eight neighbours clockwise |
| RDP | Ramer-Douglas-Peucker: simplifying a polyline by dropping points closer than an epsilon to the chord |
| Otsu | Choosing the binarisation threshold that maximises between-class variance |
| BT.601 | The luma weights 0.299 R + 0.587 G + 0.114 B |
| Golden test | A test that compares an output's hash with a recorded value to pin behaviour |
| Quad | Four corner points, ordered top-left, top-right, bottom-right, bottom-left |
