//! Row-level building blocks shared by the filters: replicated borders and weighted sums of shifted slices.

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
