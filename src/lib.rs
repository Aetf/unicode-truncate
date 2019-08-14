// Copyright 2019 Aetf <aetf at unlimitedcodeworks dot xyz>. See the COPYRIGHT
// file at the top-level directory of this distribution.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Unicode-aware, `O(n)` algorithm to pad or truncate `str` in terms of displayed width.
//!
//! # Examples
//! ```rust
//! use unicode_truncate::UnicodeTruncateStr;
//!
//! fn main() {
//!     let (rv, w) = "你好吗".unicode_truncate(5);
//!     assert_eq!(rv, "你好");
//!     assert_eq!(w, 4);
//! }
//! ```

#![deny(missing_docs, unsafe_code)]

use unicode_width::UnicodeWidthChar;

/// Methods for padding or truncating using displayed width of Unicode strings.
pub trait UnicodeTruncateStr {
    /// Truncates a string to be at most `width` in terms of display width.
    ///
    /// For wide characters, it may not always be possible to truncate at exact width. In this case,
    /// the longest possible string is returned. To help the caller determine the situation, the
    /// display width of the returned string slice is also returned.
    ///
    /// # Arguments
    /// * `width` - the maximum display width
    ///
    /// # Examples
    /// Simple ascii string
    /// ...
    fn unicode_truncate(&self, width: usize) -> (&str, usize);
}

impl UnicodeTruncateStr for str {
    #[inline]
    fn unicode_truncate(&self, width: usize) -> (&str, usize) {
        // bail out fast
        if width == 0 {
            return (self.get(..0).unwrap(), 0);
        }

        // pre-process the str into a prefix array of (byte index, width), at char boundaries
        let ch_widths: Vec<(usize, usize)> = self
            .char_indices()
            .map(|(bidx, c)| (bidx, c.width().unwrap_or(0)))
            // chain an extra end value so acc for last value is returned
            .chain(vec![(self.len(), 0)].into_iter())
            .scan(0, |acc, (bidx, x)| {
                let last_acc = *acc;
                *acc = *acc + x;
                Some((bidx, last_acc))
            })
            .collect();

        // fast path
        let total_width = ch_widths.last().unwrap().1;
        if total_width < width {
            return (self, total_width);
        }

        let (bidx, new_total_width) = match ch_widths.binary_search_by_key(&width, |&(_, w)| w) {
            Ok(idx) => ch_widths[idx],
            // the first elem of ch_widths is always (0, 0), width > 1, thus idx > 0
            Err(idx) => ch_widths[idx - 1],
        };

        (self.get(..bidx).unwrap(), new_total_width)
    }
}

#[cfg(test)]
mod tests {
    use crate::UnicodeTruncateStr;

    #[test]
    fn truncate_empty() {
        let (rv, rw) = "".unicode_truncate(4);
        assert_eq!(rv, "");
        assert_eq!(rw, 0);
    }

    #[test]
    fn truncate_zero_width() {
        let (rv, rw) = "ab".unicode_truncate(0);
        assert_eq!(rv, "");
        assert_eq!(rw, 0);

        let (rv, rw) = "你好".unicode_truncate(0);
        assert_eq!(rv, "");
        assert_eq!(rw, 0);
    }

    #[test]
    fn truncate_less_than_limit() {
        let (rv, rw) = "abc".unicode_truncate(4);
        assert_eq!(rv, "abc");
        assert_eq!(rw, 3);

        let (rv, rw) = "你".unicode_truncate(4);
        assert_eq!(rv, "你");
        assert_eq!(rw, 2);
    }

    #[test]
    fn truncate_at_boundary() {
        let (rv, rw) = "boundary".unicode_truncate(5);
        assert_eq!(rv, "bound");
        assert_eq!(rw, 5);

        let (rv, rw) = "你好吗".unicode_truncate(4);
        assert_eq!(rv, "你好");
        assert_eq!(rw, 4);
    }

    #[test]
    fn truncate_not_boundary() {
        let (rv, rw) = "你好吗".unicode_truncate(3);
        assert_eq!(rv, "你");
        assert_eq!(rw, 2);
    }
}
