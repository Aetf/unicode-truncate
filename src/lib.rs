// Copyright 2019 Aetf <aetf at unlimitedcodeworks dot xyz>.
// See the COPYRIGHT file at the top-level directory of this distribution.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

#![forbid(missing_docs, unsafe_code)]
#![warn(clippy::arithmetic_side_effects)]
#![cfg_attr(not(feature = "std"), no_std)]

//! Unicode-aware algorithm to pad or truncate `str` in terms of displayed width.
//!
//! See the [`UnicodeTruncateStr`] trait for new methods available on `str`.
//!
//! # How truncation works
//!
//! Text is measured in display columns with [`unicode_width`], and is only ever cut at boundaries
//! between grapheme clusters, so a cluster is never split. The width of a slice is taken to be the
//! sum of the widths of its graphemes.
//!
//! Because a grapheme can be several columns wide, the requested width cannot always be hit
//! exactly. Truncating from one end is still unambiguous, the longest slice that fits is returned.
//! Truncating from both ends has to weigh how much is kept against how centered it is.
//!
//! ## Centered truncation
//!
//! Every slice of the input that starts and ends on a grapheme boundary and is at most `max_width`
//! wide is a candidate. Each candidate is scored, in display columns, by
//!
//! ```text
//! penalty = (max_width - width of the slice) + distance from the center of the slice
//!                                              to the center of the whole string
//! ```
//!
//! and the candidate with the lowest penalty is returned. A penalty of zero is a perfect result:
//! the whole budget is used and the slice is exactly centered. Ties are broken towards removing
//! less from the start, and then towards the later slice, which drops zero width graphemes at the
//! start while keeping the ones at the end.
//!
//! The two terms are traded against each other, so a column of width is given up when it buys more
//! than a column of centering:
//!
//! ```rust
//! use unicode_truncate::UnicodeTruncateStr;
//! use unicode_width::UnicodeWidthStr;
//!
//! let text = "好你👪bbᄀᄀᄀ好👪";
//! assert_eq!(text.width(), 18);
//! // "bᄀᄀᄀ好👪" is 11 columns but sits 3.5 columns off center, this is 10 and dead center
//! assert_eq!(text.unicode_truncate_centered(11), ("👪bbᄀᄀᄀ", 10));
//! ```
//!
//! A grapheme wider than `max_width` cannot be part of any candidate. One sitting in the middle
//! therefore pushes the result off to one side, and using up the budget is all that is left to
//! optimize for:
//!
//! ```rust
//! use unicode_truncate::UnicodeTruncateStr;
//! // the family emoji is 2 columns wide and cannot be split
//! assert_eq!("123👨‍👩‍👧‍👦456".unicode_truncate_centered(1), ("3", 1));
//! ```
//!
//! ## Known limitation
//!
//! A few ligatures, such as the Arabic lam-alef, span grapheme clusters and render narrower than
//! their parts, so the width of a string containing one is less than the sum of the widths of its
//! graphemes. Candidates are measured by that sum, which makes truncation conservative on such
//! strings: what is returned can be narrower than what would have fit. The width returned
//! alongside it is always the width of the returned slice.
//!
//! # Examples
//! Safely truncate string to display width even not at character boundaries.
//! ```rust
//! use unicode_truncate::UnicodeTruncateStr;
//! assert_eq!("你好吗".unicode_truncate(5), ("你好", 4));
//! ```
#![cfg_attr(
    feature = "std",
    doc = r##"
Making sure the string is displayed in exactly number of columns by
combining padding and truncating.

```rust
use unicode_truncate::UnicodeTruncateStr;
use unicode_truncate::Alignment;
use unicode_width::UnicodeWidthStr;

let str = "你好吗".unicode_pad(5, Alignment::Left, true);
assert_eq!(str, "你好 ");
assert_eq!(str.width(), 5);
```
"##
)]

use core::cmp::Reverse;

use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

/// Defines the alignment for truncation and padding.
#[derive(PartialEq, Eq, Debug, Copy, Clone)]
pub enum Alignment {
    /// Align to the left
    Left,
    /// Align center
    Center,
    /// Align to the right
    Right,
}

/// Methods for padding or truncating using displayed width of Unicode strings.
pub trait UnicodeTruncateStr {
    /// Truncates a string to be at most `width` in terms of display width by removing the end
    /// characters.
    ///
    /// For wide characters, it may not always be possible to truncate at exact width. In this case,
    /// the longest possible string is returned. To help the caller determine the situation, the
    /// display width of the returned string slice is also returned.
    ///
    /// Zero-width characters decided by [`unicode_width`] are always included when deciding the
    /// truncation point.
    ///
    /// # Arguments
    /// * `max_width` - the maximum display width
    fn unicode_truncate(&self, max_width: usize) -> (&str, usize);

    /// Truncates a string to be at most `width` in terms of display width by removing the start
    /// characters.
    ///
    /// For wide characters, it may not always be possible to truncate at exact width. In this case,
    /// the longest possible string is returned. To help the caller determine the situation, the
    /// display width of the returned string slice is also returned.
    ///
    /// Zero-width characters decided by [`unicode_width`] are always removed when deciding the
    /// truncation point.
    ///
    /// # Arguments
    /// * `max_width` - the maximum display width
    fn unicode_truncate_start(&self, max_width: usize) -> (&str, usize);

    /// Truncates a string to be at most `width` in terms of display width by removing
    /// characters at both start and end.
    ///
    /// For wide characters, it may not always be possible to truncate at exact width, nor to stay
    /// exactly centered. The slice with the lowest sum of unused width and distance from the
    /// center is returned, see [the crate documentation](crate#centered-truncation) for the exact
    /// rule. To help the caller determine the situation, the display width of the returned string
    /// slice is also returned.
    ///
    /// Zero-width characters decided by [`unicode_width`] are included if they are at end, or
    /// removed if they are at the beginning when deciding the truncation point.
    ///
    /// # Arguments
    /// * `max_width` - the maximum display width
    fn unicode_truncate_centered(&self, max_width: usize) -> (&str, usize);

    /// Truncates a string to be at most `width` in terms of display width by removing
    /// characters.
    ///
    /// Depending on the alignment characters are removed. When left aligned characters from the end
    /// are removed. When right aligned characters from the start are removed. When centered
    /// characters from both sides are removed.
    ///
    /// For wide characters, it may not always be possible to truncate at exact width. In this case,
    /// the longest possible string is returned, or for [`Alignment::Center`] the one with the
    /// lowest penalty, see [the crate documentation](crate#centered-truncation). To help the
    /// caller determine the situation, the display width of the returned string slice is also
    /// returned.
    ///
    /// Zero-width characters decided by [`unicode_width`] are included if they are at end, or
    /// removed if they are at the beginning when deciding the truncation point.
    ///
    /// # Arguments
    /// * `max_width` - the maximum display width
    /// * `align` - alignment for truncation
    #[inline]
    fn unicode_truncate_aligned(&self, max_width: usize, align: Alignment) -> (&str, usize) {
        match align {
            Alignment::Left => self.unicode_truncate(max_width),
            Alignment::Center => self.unicode_truncate_centered(max_width),
            Alignment::Right => self.unicode_truncate_start(max_width),
        }
    }

    /// Pads a string to be `width` in terms of display width. Only available when the `std` feature
    /// of this library is activated, and it is activated by default.
    ///
    /// When `truncate` is true, the string is truncated to `width` if necessary. In case of wide
    /// characters and truncation point not at character boundary, the longest possible string is
    /// used, and padded to exact `width` according to `align`.
    /// See [`unicode_truncate`](crate::UnicodeTruncateStr::unicode_truncate) for the behavior of
    /// truncation.
    ///
    /// # Arguments
    /// * `target_width` - the display width to pad to
    /// * `align` - alignment for truncation and padding
    /// * `truncate` - whether to truncate string if necessary
    #[cfg(feature = "std")]
    fn unicode_pad(
        &self,
        target_width: usize,
        align: Alignment,
        truncate: bool,
    ) -> std::borrow::Cow<'_, str>;
}

/// Iterates over all grapheme boundaries of `s`, including the one past the last grapheme,
/// yielding the byte index of the boundary and the display width of the string before it.
#[inline]
fn grapheme_boundaries(s: &str) -> impl Iterator<Item = (usize, usize)> + '_ {
    s.grapheme_indices(true)
        .map(|(byte_index, grapheme)| (byte_index, grapheme.width()))
        // chain a final element representing the position past the last grapheme
        .chain(core::iter::once((s.len(), 0)))
        .scan(0usize, |sum, (byte_index, grapheme_width)| {
            // byte_index is the start while the grapheme_width is at the end. Current width is
            // the sum until now while the next byte_index is including the current
            // grapheme_width.
            let current_width = *sum;
            *sum = sum.checked_add(grapheme_width)?;
            Some((byte_index, current_width))
        })
}

/// A candidate result of a centered truncation: the byte range that would be kept, together with
/// how much display width is removed on each side and how much is kept.
///
/// All widths are the sum of the widths of the graphemes involved.
#[derive(Clone, Copy)]
struct Window {
    start: usize,
    end: usize,
    removed_start: usize,
    removed_end: usize,
    kept: usize,
}

impl Window {
    /// Twice the distance between the center of the window and the center of the whole string.
    ///
    /// Doubled to stay in whole columns: either center can sit on a half column.
    #[inline]
    fn off_center(&self) -> usize {
        self.removed_start.abs_diff(self.removed_end)
    }

    /// How bad this window is, doubled to match [`Window::off_center`]. Zero is a perfect result:
    /// the whole budget is used and the window is exactly centered.
    #[inline]
    fn penalty(&self, max_width: usize) -> usize {
        max_width
            .saturating_sub(self.kept)
            .saturating_mul(2)
            .saturating_add(self.off_center())
    }

    /// The order the result is picked by, smaller is better: least penalty, then least removed
    /// from the start, then the latest start, which drops zero width graphemes at the start while
    /// keeping the ones at the end.
    #[inline]
    fn rank(&self, max_width: usize) -> (usize, usize, Reverse<usize>) {
        (
            self.penalty(max_width),
            self.removed_start,
            Reverse(self.start),
        )
    }
}

/// Grows `window` towards the end of the string as far as `max_width` allows, taking graphemes
/// from `graphemes`, which yields byte indices relative to `base`.
#[inline]
fn grow_window<'a, I>(
    window: &mut Window,
    graphemes: &mut core::iter::Peekable<I>,
    base: usize,
    max_width: usize,
) where
    I: Iterator<Item = (usize, &'a str)>,
{
    while let Some(&(byte_index, grapheme)) = graphemes.peek() {
        let width = grapheme.width();
        let Some(grown) = window.kept.checked_add(width) else {
            break;
        };
        if grown > max_width {
            break;
        }
        window.kept = grown;
        window.removed_end = window.removed_end.saturating_sub(width);
        window.end = base
            .saturating_add(byte_index)
            .saturating_add(grapheme.len());
        graphemes.next();
    }
}

impl UnicodeTruncateStr for str {
    #[inline]
    fn unicode_truncate(&self, max_width: usize) -> (&str, usize) {
        let (byte_index, _) = grapheme_boundaries(self)
            // take the longest but still shorter than requested
            .take_while(|&(_, current_width)| current_width <= max_width)
            .last()
            .unwrap_or((0, 0));

        // unwrap is safe as the index comes from grapheme_indices
        let result = self.get(..byte_index).unwrap();
        // the sum of the grapheme widths the cut was made on is an upper bound of the width of
        // the result, so measure the result itself rather than reporting that sum
        let result_width = result.width();
        debug_assert!(result_width <= max_width);
        (result, result_width)
    }

    #[inline]
    fn unicode_truncate_start(&self, max_width: usize) -> (&str, usize) {
        let (byte_index, _) = self
            .grapheme_indices(true)
            // instead of start checking from the start do so from the end
            .rev()
            // map to byte index and the width of grapheme start at the index
            .map(|(byte_index, grapheme)| (byte_index, grapheme.width()))
            // fold to byte index and the width from end to the index
            .scan(0, |sum: &mut usize, (byte_index, grapheme_width)| {
                *sum = sum.checked_add(grapheme_width)?;
                Some((byte_index, *sum))
            })
            .take_while(|&(_, current_width)| current_width <= max_width)
            .last()
            .unwrap_or((self.len(), 0));

        // unwrap is safe as the index comes from grapheme_indices
        let result = self.get(byte_index..).unwrap();
        let result_width = result.width();
        debug_assert!(result_width <= max_width);
        (result, result_width)
    }

    #[inline]
    fn unicode_truncate_centered(&self, max_width: usize) -> (&str, usize) {
        if max_width == 0 {
            return ("", 0);
        }

        let original_width = self.width();
        if original_width <= max_width {
            return (self, original_width);
        }

        // The window is scored against the sum of the widths of the graphemes, which is not always
        // the width of the whole string, so it has to be summed up separately. Once the two agree
        // this pass can go away and `original_width` can be used instead.
        // unwrap is safe as grapheme_boundaries always yields the position past the last grapheme
        let (_, summed_width) = grapheme_boundaries(self).last().unwrap();

        // Four cursors, each walking away from the anchor found below and never backtracking:
        // `starts`/`ends` shrink the window during the search for the anchor and keep going for
        // the pass towards the end resp. the start.
        let mut starts = self.grapheme_indices(true);
        let mut ends = self.grapheme_indices(true).rev();

        // Shrink the window from both ends, always giving up on the side that gave up less so far,
        // until it fits. `kept > max_width` implies the window is not empty, so the two cursors can
        // never pass each other.
        let mut anchor = Window {
            start: 0,
            end: self.len(),
            removed_start: 0,
            removed_end: 0,
            kept: summed_width,
        };
        while anchor.kept > max_width {
            if anchor.removed_start <= anchor.removed_end {
                // unwrap is safe as the window is not empty
                let (byte_index, grapheme) = starts.next().unwrap();
                let width = grapheme.width();
                anchor.start = byte_index.saturating_add(grapheme.len());
                anchor.removed_start = anchor.removed_start.saturating_add(width);
                anchor.kept = anchor.kept.saturating_sub(width);
            } else {
                // unwrap is safe as the window is not empty
                let (byte_index, grapheme) = ends.next().unwrap();
                anchor.end = byte_index;
                anchor.removed_end = anchor.removed_end.saturating_add(grapheme.width());
                anchor.kept = anchor.kept.saturating_sub(grapheme.width());
            }
        }

        // The cursor used to grow the window towards the end of the string.
        let mut grow_end = self
            .get(anchor.end..)
            .unwrap_or_default()
            .grapheme_indices(true)
            .peekable();
        let base = anchor.end;
        grow_window(&mut anchor, &mut grow_end, base, max_width);

        let mut best = anchor;

        // Slide the window towards the end of the string. `off_center` only grows once past the
        // most balanced window, and the penalty is never smaller than it, so the first window that
        // is more off center than the best one is worth cannot be followed by a better one.
        let mut current = anchor;
        for (byte_index, grapheme) in starts {
            let width = grapheme.width();
            current.start = byte_index.saturating_add(grapheme.len());
            current.removed_start = current.removed_start.saturating_add(width);
            if current.start > current.end {
                // the window was empty and this grapheme is wider than the budget, so it moves
                // from the removed part at the end to the removed part at the start
                current.end = current.start;
                current.removed_end = current.removed_end.saturating_sub(width);
                grow_end.next();
            } else {
                current.kept = current.kept.saturating_sub(width);
            }
            grow_window(&mut current, &mut grow_end, base, max_width);
            if current.off_center() > best.penalty(max_width) {
                break;
            }
            if current.rank(max_width) < best.rank(max_width) {
                best = current;
            }
        }

        // Slide the window towards the start of the string, shrinking it from the end whenever it
        // no longer fits.
        let mut current = anchor;
        let grow_start = self
            .get(..anchor.start)
            .unwrap_or_default()
            .grapheme_indices(true)
            .rev();
        // `ends` stopped before the window was grown to its final size, so it cannot be reused
        let mut shrink_end = self
            .get(..anchor.end)
            .unwrap_or_default()
            .grapheme_indices(true)
            .rev();
        for (byte_index, grapheme) in grow_start {
            let width = grapheme.width();
            current.start = byte_index;
            current.removed_start = current.removed_start.saturating_sub(width);
            current.kept = current.kept.saturating_add(width);
            while current.kept > max_width {
                // unwrap is safe as the window is not empty
                let (end_index, end_grapheme) = shrink_end.next().unwrap();
                current.end = end_index;
                current.removed_end = current.removed_end.saturating_add(end_grapheme.width());
                current.kept = current.kept.saturating_sub(end_grapheme.width());
            }
            if current.off_center() > best.penalty(max_width) {
                break;
            }
            if current.rank(max_width) < best.rank(max_width) {
                best = current;
            }
        }

        // unwrap is safe as both indices come from grapheme_indices and the window is never
        // reversed
        let result = self.get(best.start..best.end).unwrap();
        let result_width = result.width();
        debug_assert!(result_width <= max_width);
        (result, result_width)
    }

    #[cfg(feature = "std")]
    #[inline]
    fn unicode_pad(
        &self,
        target_width: usize,
        align: Alignment,
        truncate: bool,
    ) -> std::borrow::Cow<'_, str> {
        use std::borrow::Cow;

        if !truncate && self.width() >= target_width {
            return Cow::Borrowed(self);
        }

        let (truncated, columns) = self.unicode_truncate(target_width);
        if columns == target_width {
            return Cow::Borrowed(truncated);
        }

        // the string is less than width, or truncated to less than width
        let diff = target_width.saturating_sub(columns);
        let (left_pad, right_pad) = match align {
            Alignment::Left => (0, diff),
            Alignment::Right => (diff, 0),
            Alignment::Center => (diff / 2, diff.saturating_sub(diff / 2)),
        };
        debug_assert_eq!(diff, left_pad.saturating_add(right_pad));

        let new_len = truncated
            .len()
            .checked_add(diff)
            .expect("Padded result should fit in a new String");
        let mut result = String::with_capacity(new_len);
        for _ in 0..left_pad {
            result.push(' ');
        }
        result += truncated;
        for _ in 0..right_pad {
            result.push(' ');
        }
        Cow::Owned(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod truncate_end {
        use super::*;

        #[test]
        fn empty() {
            assert_eq!("".unicode_truncate(4), ("", 0));
        }

        #[test]
        fn zero_width() {
            assert_eq!("ab".unicode_truncate(0), ("", 0));
            assert_eq!("你好".unicode_truncate(0), ("", 0));
        }

        #[test]
        fn less_than_limit() {
            assert_eq!("abc".unicode_truncate(4), ("abc", 3));
            assert_eq!("你".unicode_truncate(4), ("你", 2));
        }

        #[test]
        fn at_boundary() {
            assert_eq!("boundary".unicode_truncate(5), ("bound", 5));
            assert_eq!("你好吗".unicode_truncate(4), ("你好", 4));
        }

        #[test]
        fn not_boundary() {
            assert_eq!("你好吗".unicode_truncate(3), ("你", 2));
            assert_eq!("你好吗".unicode_truncate(1), ("", 0));
        }

        #[test]
        fn zero_width_char_in_middle() {
            // zero width character in the middle is intact
            assert_eq!("y\u{0306}es".unicode_truncate(2), ("y\u{0306}e", 2));
        }

        #[test]
        fn keep_zero_width_char_at_boundary() {
            // zero width character at end is preserved
            assert_eq!(
                "y\u{0306}ey\u{0306}s".unicode_truncate(3),
                ("y\u{0306}ey\u{0306}", 3)
            );
        }

        #[test]
        fn family_stays_together() {
            let input = "123👨‍👩‍👧‍👦456";

            // Family emoji should be of width 2
            assert_eq!("👨‍👩‍👧‍👦".width(), 2);

            assert_eq!(input.unicode_truncate(4), ("123", 3));
            assert_eq!(input.unicode_truncate(5), ("123👨‍👩‍👧‍👦", 5));
            assert_eq!(input.unicode_truncate(6), ("123👨‍👩‍👧‍👦4", 6));
            assert_eq!(input.unicode_truncate(20), (input, 8));
        }
    }

    mod truncate_start {
        use super::*;

        #[test]
        fn empty() {
            assert_eq!("".unicode_truncate_start(4), ("", 0));
        }

        #[test]
        fn zero_width() {
            assert_eq!("ab".unicode_truncate_start(0), ("", 0));
            assert_eq!("你好".unicode_truncate_start(0), ("", 0));
        }

        #[test]
        fn less_than_limit() {
            assert_eq!("abc".unicode_truncate_start(4), ("abc", 3));
            assert_eq!("你".unicode_truncate_start(4), ("你", 2));
        }

        #[test]
        fn at_boundary() {
            assert_eq!("boundary".unicode_truncate_start(5), ("ndary", 5));
            assert_eq!("你好吗".unicode_truncate_start(4), ("好吗", 4));
        }

        #[test]
        fn not_boundary() {
            assert_eq!("你好吗".unicode_truncate_start(3), ("吗", 2));
            assert_eq!("你好吗".unicode_truncate_start(1), ("", 0));
        }

        #[test]
        fn zero_width_char_in_middle() {
            // zero width character in middle is preserved
            assert_eq!(
                "y\u{0306}ey\u{0306}s".unicode_truncate_start(2),
                ("y\u{0306}s", 2)
            );
        }

        #[test]
        fn remove_zero_width_char_at_boundary() {
            // zero width character in the middle at the cutting boundary is removed
            assert_eq!("y\u{0306}es".unicode_truncate_start(2), ("es", 2));
        }

        #[test]
        fn family_stays_together() {
            let input = "123👨‍👩‍👧‍👦456";

            // Family emoji should be of width 2
            assert_eq!("👨‍👩‍👧‍👦".width(), 2);

            assert_eq!(input.unicode_truncate_start(4), ("456", 3));
            assert_eq!(input.unicode_truncate_start(5), ("👨‍👩‍👧‍👦456", 5));
            assert_eq!(input.unicode_truncate_start(6), ("3👨‍👩‍👧‍👦456", 6));
            assert_eq!(input.unicode_truncate_start(20), (input, 8));
        }
    }

    mod truncate_centered {
        use super::*;

        #[test]
        fn empty() {
            assert_eq!("".unicode_truncate_centered(4), ("", 0));
        }

        #[test]
        fn zero_width() {
            assert_eq!("ab".unicode_truncate_centered(0), ("", 0));
            assert_eq!("你好".unicode_truncate_centered(0), ("", 0));
        }

        #[test]
        fn less_than_limit() {
            assert_eq!("abc".unicode_truncate_centered(4), ("abc", 3));
            assert_eq!("你".unicode_truncate_centered(4), ("你", 2));
        }

        /// The source code has special handling for small `min_removal_width` (half-point)
        #[test]
        fn truncate_exactly_one() {
            assert_eq!("abcd".unicode_truncate_centered(3), ("abc", 3));
        }

        #[test]
        fn at_boundary() {
            assert_eq!(
                "boundaryboundary".unicode_truncate_centered(5),
                ("arybo", 5)
            );
            assert_eq!(
                "你好吗你好吗你好吗".unicode_truncate_centered(4),
                ("你好", 4)
            );
        }

        #[test]
        fn not_boundary() {
            assert_eq!("你好吗你好吗".unicode_truncate_centered(3), ("吗", 2));
            assert_eq!("你好吗你好吗".unicode_truncate_centered(1), ("", 0));
        }

        #[test]
        fn zero_width_char_in_middle() {
            // zero width character in middle is preserved
            assert_eq!(
                "yy\u{0306}es".unicode_truncate_centered(2),
                ("y\u{0306}e", 2)
            );
        }

        #[test]
        fn zero_width_char_at_boundary() {
            // zero width character at the cutting boundary in the start is removed
            // but those in the end is kept.
            assert_eq!(
                "y\u{0306}ea\u{0306}b\u{0306}y\u{0306}ea\u{0306}b\u{0306}"
                    .unicode_truncate_centered(2),
                ("b\u{0306}y\u{0306}", 2)
            );
            assert_eq!(
                "ay\u{0306}ea\u{0306}b\u{0306}y\u{0306}ea\u{0306}b\u{0306}"
                    .unicode_truncate_centered(2),
                ("a\u{0306}b\u{0306}", 2)
            );
            assert_eq!(
                "y\u{0306}ea\u{0306}b\u{0306}y\u{0306}ea\u{0306}b\u{0306}a"
                    .unicode_truncate_centered(2),
                ("b\u{0306}y\u{0306}", 2)
            );
        }

        #[test]
        fn control_char() {
            use unicode_width::UnicodeWidthChar;
            assert_eq!("\u{0019}".width(), 1);
            assert_eq!('\u{0019}'.width(), None);
            assert_eq!("\u{0019}".unicode_truncate(2), ("\u{0019}", 1));
        }

        #[test]
        fn family_stays_together() {
            let input = "123👨‍👩‍👧‍👦456";

            // Family emoji should be of width 2
            assert_eq!("👨‍👩‍👧‍👦".width(), 2);

            // the family cannot be split, so the widest window of width 1 is a single digit
            assert_eq!(input.unicode_truncate_centered(1), ("3", 1));
            assert_eq!(input.unicode_truncate_centered(2), ("👨‍👩‍👧‍👦", 2));
            assert_eq!(input.unicode_truncate_centered(4), ("3👨‍👩‍👧‍👦4", 4));
            assert_eq!(input.unicode_truncate_centered(6), ("23👨‍👩‍👧‍👦45", 6));
            assert_eq!(input.unicode_truncate_centered(20), (input, 8));
        }

        /// The removal used to be tracked from the end only after the first grapheme was removed
        /// from the end, resulting in a reversed range.
        #[test]
        fn removal_only_from_start() {
            assert_eq!("a你".unicode_truncate_centered(2), ("你", 2));
            assert_eq!("a你b".unicode_truncate_centered(2), ("你", 2));
        }

        /// A single grapheme can be wider than any assumed constant.
        #[test]
        fn very_wide_grapheme() {
            // a run of Hangul leading jamo is a single grapheme cluster
            const WIDE: &str = "\u{1100}\u{1100}\u{1100}\u{1100}\u{1100}\u{1100}\u{1100}";
            assert_eq!(WIDE.graphemes(true).count(), 1);
            assert_eq!(WIDE.width(), 14);

            let input =
                "abcdefghij\u{1100}\u{1100}\u{1100}\u{1100}\u{1100}\u{1100}\u{1100}klmnopqrst";
            assert_eq!(
                input.unicode_truncate_centered(16),
                (
                    "j\u{1100}\u{1100}\u{1100}\u{1100}\u{1100}\u{1100}\u{1100}k",
                    16
                )
            );
            // the wide grapheme does not fit, so the widest window is next to it
            assert_eq!(input.unicode_truncate_centered(12), ("abcdefghij", 10));
            assert_eq!(input.unicode_truncate_centered(1), ("j", 1));

            let input =
                "\u{1100}\u{1100}\u{1100}\u{1100}\u{1100}\u{1100}\u{1100}abcdefghijklmnopqrst";
            assert_eq!(
                input.unicode_truncate_centered(16),
                ("abcdefghijklmnop", 16)
            );
            assert_eq!(input.unicode_truncate_centered(12), ("abcdefghijkl", 12));
        }
    }

    /// The centered variant picks the best window out of a large search space, compare it against
    /// a brute force search over all grapheme boundaries on pseudo random inputs.
    #[cfg(feature = "std")]
    #[test]
    fn centered_matches_brute_force() {
        fn brute_force(input: &str, max_width: usize) -> (&str, usize) {
            if max_width == 0 {
                return ("", 0);
            }
            let total_width = input.width();
            if total_width <= max_width {
                return (input, total_width);
            }
            let bounds: Vec<(usize, usize)> = grapheme_boundaries(input).collect();
            let mut best = None;
            for (i, &(start_index, start_removed)) in bounds.iter().enumerate() {
                for &(end_index, end_kept) in &bounds[i..] {
                    let kept = end_kept - start_removed;
                    if kept > max_width {
                        break;
                    }
                    let window = Window {
                        start: start_index,
                        end: end_index,
                        removed_start: start_removed,
                        removed_end: total_width - end_kept,
                        kept,
                    };
                    // least penalty, then least removed from the start, then the latest window
                    let key = (window.rank(max_width), Reverse(end_index));
                    if best.is_none_or(|(best_key, _)| key < best_key) {
                        best = Some((key, window));
                    }
                }
            }
            let window = best.unwrap().1;
            (&input[window.start..window.end], window.kept)
        }

        // no character forming a ligature with a neighboring grapheme, those make the width of a
        // string differ from the sum of the widths of its graphemes
        let alphabet: Vec<char> =
            "ab \u{4f60}\u{597d}\u{0306}\u{1100}\u{200d}\u{1f468}\u{1f469}\r\n"
                .chars()
                .collect();
        let mut seed = 0x2545_f491_4f6c_dd1d_u64;
        let mut random = move || {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            seed
        };
        for _ in 0..2000 {
            let len = random() as usize % 12;
            let input: String = (0..len)
                .map(|_| alphabet[random() as usize % alphabet.len()])
                .collect();
            for max_width in 0..8 {
                let actual = input.unicode_truncate_centered(max_width);
                assert_eq!(
                    actual,
                    brute_force(&input, max_width),
                    "input {input:?} max_width {max_width}"
                );
                assert_eq!(
                    actual.0.width(),
                    actual.1,
                    "input {input:?} max_width {max_width}"
                );
                assert!(
                    actual.1 <= max_width,
                    "input {:?} max_width {}",
                    input,
                    max_width
                );
            }
        }
    }

    /// A few ligatures span grapheme clusters and render narrower than their parts, so the width
    /// of the string is not the sum of the widths of its graphemes.
    mod cross_grapheme_ligature {
        use super::*;

        /// Arabic lam followed by alef, which renders as a single ligature one column wide
        const LAM_ALEF: &str = "\u{0644}\u{0627}";
        /// The same pair eight times over
        const LAM_ALEFS: &str = "\u{0644}\u{0627}\u{0644}\u{0627}\u{0644}\u{0627}\u{0644}\u{0627}\u{0644}\u{0627}\u{0644}\u{0627}\u{0644}\u{0627}\u{0644}\u{0627}";

        #[test]
        fn width_differs_from_the_sum_of_its_graphemes() {
            assert_eq!(LAM_ALEF.width(), 1);
            assert_eq!(
                LAM_ALEF.graphemes(true).map(|g| g.width()).sum::<usize>(),
                2
            );
            assert_eq!(LAM_ALEFS.width(), 8);
            assert_eq!(
                LAM_ALEFS.graphemes(true).map(|g| g.width()).sum::<usize>(),
                16
            );
        }

        #[test]
        fn reported_width_is_the_width_of_the_result() {
            for input in [LAM_ALEF, LAM_ALEFS] {
                for max_width in 0..10 {
                    for (result, width) in [
                        input.unicode_truncate(max_width),
                        input.unicode_truncate_start(max_width),
                        input.unicode_truncate_centered(max_width),
                    ] {
                        assert_eq!(result.width(), width, "{input:?} at {max_width}");
                        assert!(width <= max_width, "{:?} at {}", input, max_width);
                    }
                }
            }
        }

        #[cfg(feature = "std")]
        #[test]
        fn padding_reaches_the_target_width() {
            for align in [Alignment::Left, Alignment::Center, Alignment::Right] {
                for target_width in 0..10 {
                    let padded = LAM_ALEFS.unicode_pad(target_width, align, true);
                    assert_eq!(padded.width(), target_width, "{align:?} at {target_width}");
                }
            }
        }
    }

    #[test]
    fn truncate_aligned() {
        assert_eq!("abc".unicode_truncate_aligned(1, Alignment::Left), ("a", 1));
        assert_eq!(
            "abc".unicode_truncate_aligned(1, Alignment::Center),
            ("b", 1)
        );
        assert_eq!(
            "abc".unicode_truncate_aligned(1, Alignment::Right),
            ("c", 1)
        );
    }

    #[cfg(feature = "std")]
    mod pad {
        use super::*;

        #[test]
        fn zero_width() {
            assert_eq!("你好".unicode_pad(0, Alignment::Left, true), "");
            assert_eq!("你好".unicode_pad(0, Alignment::Left, false), "你好");
        }

        #[test]
        fn less_than_limit() {
            assert_eq!("你".unicode_pad(4, Alignment::Left, true), "你  ");
            assert_eq!("你".unicode_pad(4, Alignment::Left, false), "你  ");
        }

        #[test]
        fn width_at_boundary() {
            assert_eq!("你好吗".unicode_pad(4, Alignment::Left, true), "你好");
            assert_eq!("你好吗".unicode_pad(4, Alignment::Left, false), "你好吗");
        }

        #[test]
        fn width_not_boundary() {
            // above limit wide chars not at boundary
            assert_eq!("你好吗".unicode_pad(3, Alignment::Left, true), "你 ");
            assert_eq!("你好吗".unicode_pad(1, Alignment::Left, true), " ");
            assert_eq!("你好吗".unicode_pad(3, Alignment::Left, false), "你好吗");

            assert_eq!("你好吗".unicode_pad(3, Alignment::Center, true), "你 ");

            assert_eq!("你好吗".unicode_pad(3, Alignment::Right, true), " 你");
        }
    }
}
