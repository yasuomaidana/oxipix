# Planned feature: raster measurement primitives

Three small measurements on images, built on kernels the library already has (thresholds, morphology, the exact
Euclidean distance transform). They answer questions a caller asks before sending an image to an output device: how thin
is the thinnest stroke, how much of each channel is used, and what physical density does the file claim. No tickets
exist yet; this is the requirement and its acceptance criteria.

## 1. Gaps

1. The distance transform exists, but nothing reports the local width of a region or the place where it is smallest.
2. Nothing reports per-channel usage of an image, with or without a mask.
3. The physical pixel density some formats record is neither read nor written.

## 2. Requirements

| ID | Priority | Requirement |
|---|---|---|
| `MEAS-1` | high | Smallest local width of foreground regions, and of background gaps, in a binary image, with the location of each minimum |
| `MEAS-2` | high | Per-channel coverage of an image over the whole frame or a mask |
| `MEAS-3` | low | Read and write the physical pixel density recorded in an image file |

## 3. Specifications

**MEAS-1: minimum feature width.**
Request: `min_width(mask, options) -> WidthReport`. The input is a binary image. The local width at a ridge point of the
distance transform is twice its distance value. The report gives the smallest width over all ridge points of the
foreground, the same for the background (gaps between foreground regions), the pixel location of each, and a histogram
of widths. Options: ignore regions smaller than an area, ignore ridge points closer than a margin to the frame, and
report the first K minima. Widths are in pixels; converting to a physical length is the caller's business.

Acceptance criteria:

- A horizontal bar N pixels high reports a foreground width of N for N from 1 to 64.
- Two bars separated by a gap of G pixels report a background width of G.
- A disc of diameter D reports D within one pixel.
- A region below the area option does not appear in the report.
- Equal inputs produce identical outputs across runs and platforms.
- A 4,000 by 4,000 image completes within the benchmark budget recorded with the first implementation.

**MEAS-2: channel coverage.**
Request: `coverage(image, mask?, threshold) -> CoverageReport`. For each channel: the mean value weighted by alpha, as a
fraction of full scale; the count of pixels whose alpha-weighted value exceeds the threshold; and the pixel count of
the frame or mask. For images with alpha, the report also gives the same two numbers for alpha itself. The function
does no colour conversion: the caller passes the image in the channel layout it wants measured.

Acceptance criteria:

- A frame filled with one value V in one channel reports a mean of V over full scale for that channel and zero for
  the others.
- A half-transparent full-value frame reports half the mean of the opaque one.
- With a mask, only masked pixels are counted, and the reported pixel count equals the mask's.
- 8-bit and 16-bit inputs of the same picture agree within one part in 255.

**MEAS-3: physical pixel density.**
Request: `read_density(bytes) -> Option<Density>` and an encoder option that writes one. A density is a horizontal and
a vertical value with a unit. Formats without the field return none. The function reads the header only.

Acceptance criteria:

- A file written with a density reads back the same value for each supported format.
- A file without the field returns none, not a default.
- A malformed field is an error, not a panic.

## 4. Out of scope

Colour management, conversion between colour spaces, and any judgement about whether a width or a coverage is
acceptable. Those belong to callers.
