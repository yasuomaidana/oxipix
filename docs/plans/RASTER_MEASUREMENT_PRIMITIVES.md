# Planned feature: raster measurement primitives

Three small measurements on images, built on kernels the library already has (thresholds, morphology, the exact
Euclidean distance transform). They answer questions a caller asks before sending an image to an output device: which
parts are thinner than a given width, how much of each channel is used, and what physical density does the file claim.
No tickets exist yet; this is the requirement and its acceptance criteria.

## 1. Gaps

1. The distance transform exists, but nothing reports which parts of a region are thinner than a given width, or the place where a region is thinnest. Morphology with a disc-shaped element is not available; it can be built from the exact distance transform.
2. Nothing reports per-channel usage of an image, with or without a mask.
3. The physical pixel density some formats record is neither read nor written.

## 2. Requirements

| ID | Priority | Requirement |
|---|---|---|
| `MEAS-1` | high | Regions of a binary image thinner than a given width, found by morphological opening with a disc, for the foreground and for background gaps within a band around it; with the smallest width found and its location |
| `MEAS-2` | high | Per-channel coverage of an image over the whole frame or a mask |
| `MEAS-3` | low | Read and write the physical pixel density recorded in an image file |

## 3. Specifications

**MEAS-1: regions thinner than a width.**
Request: `thin_regions(mask, width, options) -> ThinReport`. The input is a binary image and a width `t` in pixels.

1. **Foreground.** The thin foreground is the mask minus its morphological opening by a disc of diameter `t`: the pixels that no disc of that size, lying wholly inside the mask, covers.
2. **Background gaps.** The same on the complement of the mask, restricted to a band of pixels within `2t` of the mask. The band leaves the open background around the shapes out, and includes gaps open on one side.
3. **Corners.** Opening also rounds off convex corners. A residue pixel counts only if its distance to the opened set is more than `t / 4` plus one pixel. The rounded-off tip of a corner of 90 degrees or more is at most (√2 − 1) × `t` / 2 deep, so such corners are not reported; a sharper tip is.
4. **Regions.** A thin region is a connected part of the residue that contains at least one counting pixel. For each region the report gives its area in pixels, a bounding box, and its width `2d − 1`, where `d` is the largest value of the exact distance transform of the mask (or of the complement) inside the region, measured between pixel centres.
5. **Smallest width.** The report gives the smallest region width for the foreground and for the background, with the pixel location of each.

Options: ignore regions smaller than an area; ignore pixels closer than a margin to the frame; report the first K regions by width. The discrete form of the disc is fixed with the first implementation so that the criteria below hold. Widths are in pixels; converting to a physical length is the caller's business.

Acceptance criteria:

- An axis-aligned bar W pixels wide is reported thin exactly when W is below `t`, within one pixel, for W from 1 to 64 and `t` from 2 to 40; its reported width is within one pixel of W.
- The same holds for a bar at 45 degrees.
- Two bars separated by a gap of G pixels report a background region exactly when G is below `t`, within one pixel, with a width within one pixel of G. The same holds when the gap is open at one end.
- A filled square larger than `t` reports no thin region, at its corners or elsewhere.
- A triangle with a 30-degree corner reports one thin region at that corner.
- A disc of diameter D larger than `t` reports nothing; a disc of diameter D smaller than `t` reports one region of width D within one pixel.
- A region below the area option does not appear in the report.
- Background far from the mask (outside the `2t` band) is never reported.
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
