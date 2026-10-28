# Archived documentation

These guides describe the original goal of replacing `image`, `imageproc`, `fast_image_resize`
and `rxing` with in-tree crates. That approach was abandoned: the replacements were partial
(the PNG/JPEG decoders were stubs, barcode support was QR-only) and slower than upstream on
most operations. The code they document no longer exists. See [`../../DEV_STATUS.md`](../../DEV_STATUS.md)
and [`../FORKS.md`](../FORKS.md) for the current direction.
