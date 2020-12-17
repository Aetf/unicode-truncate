// Copyright 2019 Aetf <aetf at unlimitedcodeworks dot xyz>. See the COPYRIGHT
// file at the top-level directory of this distribution.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Unicode-aware algorithm to pad or truncate `str` in terms of displayed width.
//!
//! See the [UnicodeTruncateStr](crate::UnicodeTruncateStr) trait for new methods
//! available on `str`.
//!
//! # Examples
//! Safely truncate string to display width even not at character boundaries.
//! ```rust
//! use unicode_truncate::UnicodeTruncateStr;
//!
//! let (rv, w) = "你好吗".unicode_truncate(5);
//! assert_eq!(rv, "你好");
//! assert_eq!(w, 4);
//! ```
//!
#![cfg_attr(
    feature = "std",
    doc = r##"
Making sure the string is displayed in exactly number of columns by
combining padding and truncating.

```rust
use unicode_truncate::UnicodeTruncateStr;
use unicode_truncate::Alignment;
use unicode_width::UnicodeWidthStr;

let rv = "你好吗".unicode_pad(5, Alignment::Left, true);
assert_eq!(rv, "你好 ");
assert_eq!(rv.width(), 5);
```
"##
)]
#![deny(missing_docs, unsafe_code)]
#![cfg_attr(not(feature = "std"), no_std)]

use core::iter;

use unicode_width::UnicodeWidthChar;
use unicode_width::UnicodeWidthStr;

/// Methods for padding or truncating using displayed width of Unicode strings.
pub trait UnicodeTruncateStr {
    /// Truncates a string to be at most `width` in terms of display width by removing the end
    /// characters.
    ///
    /// For wide characters, it may not always be possible to truncate at exact width. In this case,
    /// the longest possible string is returned. To help the caller determine the situation, the
    /// display width of the returned string slice is also returned.
    ///
    /// Zero-width characters decided by [unicode_width](::unicode_width) are always included when
    /// deciding the truncation point.
    ///
    /// # Arguments
    /// * `width` - the maximum display width
    fn unicode_truncate(&self, width: usize) -> (&str, usize);

    /// Truncates a string to be at most `width` in terms of display width by removing the start
    /// characters.
    ///
    /// For wide characters, it may not always be possible to truncate at exact width. In this case,
    /// the longest possible string is returned. To help the caller determine the situation, the
    /// display width of the returned string slice is also returned.
    ///
    /// Zero-width characters decided by [unicode_width](::unicode_width) are always included when
    /// deciding the truncation point.
    ///
    /// # Arguments
    /// * `width` - the maximum display width
    fn unicode_truncate_start(&self, width: usize) -> (&str, usize);

    /// Pads a string to be `width` in terms of display width.
    /// Only available when the `std` feature of this library is activated,
    /// and it is activated by default.
    ///
    /// When `truncate` is true, the string is truncated to `width` if necessary. In case of wide
    /// characters and truncation point not at character boundary, the longest possible string
    /// is used, and padded to exact `width` according to `align`.
    /// See [unicode_truncate](crate::UnicodeTruncateStr::unicode_truncate) for the behavior of truncation.
    ///
    /// # Arguments
    /// * `width` - the display width to pad to
    /// * `align` - alignment for padding
    /// * `truncate` - whether to truncate string if necessary
    #[cfg(feature = "std")]
    fn unicode_pad(
        &self,
        width: usize,
        align: Alignment,
        truncate: bool,
    ) -> std::borrow::Cow<'_, str>;
}

impl UnicodeTruncateStr for str {
    #[inline]
    fn unicode_truncate(&self, width: usize) -> (&str, usize) {
        let (bidx, new_width) = self.char_indices()
            // map to byte index and the width of char start at the index
            .map(|(bidx, c)| (bidx, c.width().unwrap_or(0)))
            // chain a final element representing the position past the last char
            .chain(iter::once((self.len(), 0)))
            // fold to byte index and the width up to the index
            .scan(0, |w, (bidx, cw)| {
                let curr_w = *w;
                *w = *w + cw;
                Some((bidx, curr_w))
            })
            // take the longest but still shorter than requested
            .take_while(|&(_, w)| w <= width)
            .last()
            .unwrap_or((0, 0));
        (self.get(..bidx).unwrap(), new_width)
    }

    #[inline]
    fn unicode_truncate_start(&self, width: usize) -> (&str, usize) {
        let (bidx, new_width) = iter::once((self.len(), 0))
            .chain(
                self.char_indices()
                    .rev()
                    .map(|(bidx, c)| (bidx, c.width().unwrap_or(0)))
            )
            // fold to byte index and the width from end to the index
            .scan(0, |w, (bidx, cw)| {
                *w = *w + cw;
                Some((bidx, *w))
            })
            .take_while(|&(_, w)| w <= width)
            .last()
            .unwrap_or((self.len(), 0));

        (self.get(bidx..).unwrap(), new_width)
    }

    #[cfg(feature = "std")]
    #[inline]
    fn unicode_pad(
        &self,
        width: usize,
        align: Alignment,
        truncate: bool,
    ) -> std::borrow::Cow<'_, str> {
        pad::unicode_pad(self, width, align, truncate)
    }
}

#[cfg(feature = "std")]
mod pad {
    use super::*;
    pub use std::borrow::Cow;

    /// Defines the alignment for padding.
    /// Only available when the `std` feature of this library is activated,
    /// and it is activated by default.
    #[derive(PartialEq, Eq, Debug, Copy, Clone)]
    pub enum Alignment {
        /// Align to the left
        Left,
        /// Align center
        Center,
        /// Align to the right
        Right,
    }

    pub fn unicode_pad(s: &str, width: usize, align: Alignment, truncate: bool) -> Cow<'_, str> {
        let mut cols = s.width();
        let mut cs = Cow::Borrowed(s);

        if cols >= width {
            if !truncate {
                return Cow::Borrowed(s);
            }
            {
                let (new_s, new_cols) = s.unicode_truncate(width);
                cs = Cow::Borrowed(new_s);
                cols = new_cols;
            }
            if cols == width {
                return cs;
            }
        }

        // the string is less than width, or truncated to less than width
        let diff = width.saturating_sub(cols);

        let (left_pad, right_pad) = match align {
            Alignment::Left => (0, diff),
            Alignment::Right => (diff, 0),
            Alignment::Center => (diff / 2, diff.saturating_sub(diff / 2)),
        };

        let mut rv = String::new();
        rv.reserve(left_pad + cs.len() + right_pad);
        for _ in 0..left_pad {
            rv.push(' ');
        }
        rv.push_str(&cs);
        for _ in 0..right_pad {
            rv.push(' ');
        }
        Cow::Owned(rv)
    }
}
#[cfg(feature = "std")]
pub use pad::Alignment;

#[cfg(test)]
mod tests {
    mod truncate {
        use super::super::*;

        #[test]
        fn empty() {
            let (rv, rw) = "".unicode_truncate(4);
            assert_eq!(rv, "");
            assert_eq!(rw, 0);
        }

        #[test]
        fn zero_width() {
            let (rv, rw) = "ab".unicode_truncate(0);
            assert_eq!(rv, "");
            assert_eq!(rw, 0);

            let (rv, rw) = "你好".unicode_truncate(0);
            assert_eq!(rv, "");
            assert_eq!(rw, 0);
        }

        #[test]
        fn less_than_limit() {
            let (rv, rw) = "abc".unicode_truncate(4);
            assert_eq!(rv, "abc");
            assert_eq!(rw, 3);

            let (rv, rw) = "你".unicode_truncate(4);
            assert_eq!(rv, "你");
            assert_eq!(rw, 2);
        }

        #[test]
        fn at_boundary() {
            let (rv, rw) = "boundary".unicode_truncate(5);
            assert_eq!(rv, "bound");
            assert_eq!(rw, 5);

            let (rv, rw) = "你好吗".unicode_truncate(4);
            assert_eq!(rv, "你好");
            assert_eq!(rw, 4);
        }

        #[test]
        fn not_boundary() {
            let (rv, rw) = "你好吗".unicode_truncate(3);
            assert_eq!(rv, "你");
            assert_eq!(rw, 2);

            let (rv, rw) = "你好吗".unicode_truncate(1);
            assert_eq!(rv, "");
            assert_eq!(rw, 0);
        }
    }

    mod truncate_start {
        use super::super::*;

        #[test]
        fn empty() {
            let (rv, rw) = "".unicode_truncate_start(4);
            assert_eq!(rv, "");
            assert_eq!(rw, 0);
        }

        #[test]
        fn zero_width() {
            let (rv, rw) = "ab".unicode_truncate_start(0);
            assert_eq!(rv, "");
            assert_eq!(rw, 0);

            let (rv, rw) = "你好".unicode_truncate_start(0);
            assert_eq!(rv, "");
            assert_eq!(rw, 0);
        }

        #[test]
        fn less_than_limit() {
            let (rv, rw) = "abc".unicode_truncate_start(4);
            assert_eq!(rv, "abc");
            assert_eq!(rw, 3);

            let (rv, rw) = "你".unicode_truncate_start(4);
            assert_eq!(rv, "你");
            assert_eq!(rw, 2);
        }

        #[test]
        fn at_boundary() {
            let (rv, rw) = "boundary".unicode_truncate_start(5);
            assert_eq!(rv, "ndary");
            assert_eq!(rw, 5);

            let (rv, rw) = "你好吗".unicode_truncate_start(4);
            assert_eq!(rv, "好吗");
            assert_eq!(rw, 4);
        }

        #[test]
        fn not_boundary() {
            let (rv, rw) = "你好吗".unicode_truncate_start(3);
            assert_eq!(rv, "吗");
            assert_eq!(rw, 2);

            let (rv, rw) = "你好吗".unicode_truncate_start(1);
            assert_eq!(rv, "");
            assert_eq!(rw, 0);
        }
    }

    #[cfg(feature = "std")]
    mod pad {
        use super::super::*;

        #[test]
        fn zero_width() {
            let rv = "你好".unicode_pad(0, Alignment::Left, true);
            assert_eq!(&rv, "");

            let rv = "你好".unicode_pad(0, Alignment::Left, false);
            assert_eq!(&rv, "你好");
        }

        #[test]
        fn less_than_limit() {
            let rv = "你".unicode_pad(4, Alignment::Left, true);
            assert_eq!(&rv, "你  ");
            let rv = "你".unicode_pad(4, Alignment::Left, false);
            assert_eq!(&rv, "你  ");
        }

        #[test]
        fn width_at_boundary() {
            let rv = "你好吗".unicode_pad(4, Alignment::Left, true);
            assert_eq!(&rv, "你好");
            let rv = "你好吗".unicode_pad(4, Alignment::Left, false);
            assert_eq!(&rv, "你好吗");
        }

        #[test]
        fn width_not_boundary() {
            // above limit wide chars not at boundary
            let rv = "你好吗".unicode_pad(3, Alignment::Left, true);
            assert_eq!(&rv, "你 ");
            let rv = "你好吗".unicode_pad(1, Alignment::Left, true);
            assert_eq!(&rv, " ");
            let rv = "你好吗".unicode_pad(3, Alignment::Left, false);
            assert_eq!(&rv, "你好吗");
        }
    }
}
