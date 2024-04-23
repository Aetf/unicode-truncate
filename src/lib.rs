// Copyright 2019 Aetf <aetf at unlimitedcodeworks dot xyz>.
// See the COPYRIGHT file at the top-level directory of this distribution.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

#![forbid(missing_docs, unsafe_code)]
#![cfg_attr(not(feature = "std"), no_std)]

//! Unicode-aware algorithm to pad or truncate `str` in terms of displayed width.
//!
//! See the [UnicodeTruncateStr](crate::UnicodeTruncateStr) trait for new methods available on
//! `str`.
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

use unicode_width::UnicodeWidthChar;

/// Defines the alignment for padding.
/// Only available when the `std` feature of this library is activated,
/// and it is activated by default.
#[cfg(feature = "std")]
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
    /// Zero-width characters decided by [`unicode_width`] are always included when deciding the
    /// truncation point.
    ///
    /// # Arguments
    /// * `max_width` - the maximum display width
    fn unicode_truncate_start(&self, max_width: usize) -> (&str, usize);

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
    /// * `align` - alignment for padding
    /// * `truncate` - whether to truncate string if necessary
    #[cfg(feature = "std")]
    fn unicode_pad(
        &self,
        target_width: usize,
        align: Alignment,
        truncate: bool,
    ) -> std::borrow::Cow<'_, str>;
}

impl UnicodeTruncateStr for str {
    #[inline]
    fn unicode_truncate(&self, max_width: usize) -> (&str, usize) {
        let (byte_index, new_width) = self
            .char_indices()
            // map to byte index and the width of char start at the index
            .map(|(byte_index, char)| (byte_index, char.width().unwrap_or(0)))
            // chain a final element representing the position past the last char
            .chain(core::iter::once((self.len(), 0)))
            // fold to byte index and the width up to the index
            .scan(0, |sum, (byte_index, char_width)| {
                // byte_index is the start while the char_width is at the end. Current width is the
                // sum until now while the next byte_start width is including the current
                // char_width.
                let current_width = *sum;
                *sum += char_width;
                Some((byte_index, current_width))
            })
            // take the longest but still shorter than requested
            .take_while(|&(_, current_width)| current_width <= max_width)
            .last()
            .unwrap_or((0, 0));
        (self.get(..byte_index).unwrap(), new_width)
    }

    #[inline]
    fn unicode_truncate_start(&self, max_width: usize) -> (&str, usize) {
        let (byte_index, new_width) = self
            .char_indices()
            // instead of start checking from the start do so from the end
            .rev()
            // map to byte index and the width of char start at the index
            .map(|(byte_index, char)| (byte_index, char.width().unwrap_or(0)))
            // fold to byte index and the width from end to the index
            .scan(0, |sum, (byte_index, char_width)| {
                *sum += char_width;
                Some((byte_index, *sum))
            })
            .take_while(|&(_, current_width)| current_width <= max_width)
            .last()
            .unwrap_or((self.len(), 0));
        (self.get(byte_index..).unwrap(), new_width)
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

        use unicode_width::UnicodeWidthStr;

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

        let mut result = String::with_capacity(left_pad + truncated.len() + right_pad);
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
    mod truncate {
        use super::super::*;

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
    }

    mod truncate_start {
        use super::super::*;

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
    }

    #[cfg(feature = "std")]
    mod pad {
        use super::super::*;

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
        }
    }
}
