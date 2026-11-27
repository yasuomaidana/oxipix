//! Row-level building blocks shared by the filters: replicated borders, weighted sums of shifted slices and
//! the row bands that the `parallel` feature runs on the rayon pool.

/// `acc[i] += weight * src[i]` for every element of `acc`; `src` may be longer.
///
/// Written as a plain zip over slices so it compiles to widening multiply-accumulate on vectors.
#[inline]
pub(crate) fn accumulate<T: Copy + Into<u16>>(acc: &mut [u16], src: &[T], weight: u16) {
    acc.iter_mut()
        .zip(src)
        .for_each(|(a, s)| *a = a.wrapping_add(weight * (*s).into()));
}

/// `row` with its first and last pixel repeated `pixels` times on each side (`channels` bytes per pixel).
pub(crate) fn pad_row<T: Copy>(row: &[T], channels: usize, pixels: usize, out: &mut Vec<T>) {
    out.clear();
    for _ in 0..pixels {
        out.extend_from_slice(&row[..channels]);
    }
    out.extend_from_slice(row);
    for _ in 0..pixels {
        out.extend_from_slice(&row[row.len() - channels..]);
    }
}

/// Output elements below which a kernel runs on the calling thread: threads cost more than they save on small
/// pictures. About a quarter of a megapixel of one channel.
pub(crate) const PARALLEL_MIN_ELEMENTS: usize = 1 << 18;

/// Calls `work(first_row, band)` on bands of whole rows of `out` (`row_len` elements per row).
///
/// With the `parallel` feature and at least [`PARALLEL_MIN_ELEMENTS`] elements the bands run on the rayon pool
/// (the caller's pool, so `ThreadPoolBuilder::install` sets the thread count); otherwise `work` gets every
/// row at once. Each output row must depend only on shared input, so both paths give identical results.
#[inline(always)]
pub(crate) fn for_each_band<T, F>(out: &mut [T], row_len: usize, work: F)
where
    T: Send,
    F: Fn(usize, &mut [T]) + Sync,
{
    #[cfg(test)]
    let min = FORCED_MIN.get().unwrap_or(PARALLEL_MIN_ELEMENTS);
    #[cfg(not(test))]
    let min = PARALLEL_MIN_ELEMENTS;
    for_each_band_with(min, out, row_len, work);
}

#[cfg(test)]
thread_local! {
    /// Test-only override of the threshold for the calling thread.
    static FORCED_MIN: std::cell::Cell<Option<usize>> = const { std::cell::Cell::new(None) };
}

/// Runs `f` with the parallel threshold set to `min` elements on this thread (tests only).
#[cfg(test)]
pub(crate) fn with_min_elements<R>(min: usize, f: impl FnOnce() -> R) -> R {
    FORCED_MIN.set(Some(min));
    let result = f();
    FORCED_MIN.set(None);
    result
}

/// [`for_each_band`] with an explicit threshold, so tests can force the parallel path on small data.
#[inline(always)]
pub(crate) fn for_each_band_with<T, F>(min_elements: usize, out: &mut [T], row_len: usize, work: F)
where
    T: Send,
    F: Fn(usize, &mut [T]) + Sync,
{
    #[cfg(feature = "parallel")]
    {
        use rayon::prelude::*;
        if row_len > 0 && out.len() >= min_elements {
            let per_band = (out.len() / row_len)
                .div_ceil(rayon::current_num_threads() * 4)
                .max(1);
            out.par_chunks_mut(per_band * row_len)
                .enumerate()
                .for_each(|(band, rows)| work(band * per_band, rows));
            return;
        }
    }
    let _ = (min_elements, row_len);
    work(0, out);
}

/// Index of row `y + dy` clamped to `0..height`.
#[inline]
pub(crate) fn clamp_row(y: usize, dy: isize, height: usize) -> usize {
    y.saturating_add_signed(dy).min(height - 1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    #[test]
    fn accumulate_adds_the_weighted_source() {
        let mut acc = [1u16, 2, 3];
        accumulate(&mut acc, &[10u8, 20, 30, 99], 2);
        assert_eq!(acc, [21, 42, 63]);
        accumulate(&mut acc, &[1u16, 1, 1], 3);
        assert_eq!(acc, [24, 45, 66]);
    }

    #[rstest]
    #[case(1, 2, &[1, 2, 3], &[1, 1, 1, 2, 3, 3, 3])]
    #[case(3, 1, &[1, 2, 3, 4, 5, 6], &[1, 2, 3, 1, 2, 3, 4, 5, 6, 4, 5, 6])]
    #[case(1, 0, &[7, 8], &[7, 8])]
    fn borders_repeat_the_edge_pixels(
        #[case] channels: usize,
        #[case] pixels: usize,
        #[case] row: &[u8],
        #[case] want: &[u8],
    ) {
        let mut out = vec![9];
        pad_row(row, channels, pixels, &mut out);
        assert_eq!(out, want);
    }

    /// Fills every element with its row number, so the bands' first rows can be checked.
    fn rows_of(min: usize, rows: usize, row_len: usize) -> Vec<usize> {
        let mut out = vec![usize::MAX; rows * row_len];
        for_each_band_with(min, &mut out, row_len, |first, band| {
            for (k, row) in band.chunks_exact_mut(row_len).enumerate() {
                row.fill(first + k);
            }
        });
        out
    }

    #[rstest]
    #[case(usize::MAX, 37, 5)]
    #[case(0, 37, 5)]
    #[case(0, 1, 1)]
    #[case(0, 0, 4)]
    fn every_row_is_visited_once_with_its_own_number(
        #[case] min: usize,
        #[case] rows: usize,
        #[case] row_len: usize,
    ) {
        let out = rows_of(min, rows, row_len);
        assert!(
            out.chunks(row_len)
                .enumerate()
                .all(|(y, row)| row.iter().all(|v| *v == y))
        );
        assert_eq!(out.len(), rows * row_len);
    }

    #[test]
    fn a_zero_row_length_still_calls_the_work_once() {
        let mut empty: Vec<u8> = Vec::new();
        let calls = std::sync::atomic::AtomicUsize::new(0);
        for_each_band_with(0, &mut empty, 0, |_, _| {
            calls.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        });
        assert_eq!(calls.load(std::sync::atomic::Ordering::SeqCst), 1);
        let mut big = vec![0u8; 8];
        for_each_band(&mut big, 4, |first, band| band.fill(first as u8 + 1));
        assert_eq!(big, [1; 8]);
    }

    #[rstest]
    #[case(0, -2, 5, 0)]
    #[case(4, 3, 5, 4)]
    #[case(2, 1, 5, 3)]
    #[case(2, -1, 5, 1)]
    fn rows_clamp_to_the_picture(
        #[case] y: usize,
        #[case] dy: isize,
        #[case] height: usize,
        #[case] want: usize,
    ) {
        assert_eq!(clamp_row(y, dy, height), want);
    }
}
