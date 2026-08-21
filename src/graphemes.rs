//! Walking a string by grapheme cluster and measuring its display width, with a shared fast
//! path over ASCII.
//!
//! Both the segmentation rules and the width rules are contextual, so neither clusters nor
//! widths can in general be read off a substring in isolation. The insight this module is built
//! on is that one class of positions is provably safe for both at once: a printable ASCII byte
//! whose successor is ASCII. Segmentation breaks around it (GB4, GB5 and GB999, and it cannot be
//! the CR of a CR LF pair), and the width state machine reaches it in a state it ignores, since
//! every state an ASCII byte can produce is either the default or one only a carriage return
//! reacts to. Such a byte is therefore always a complete cluster exactly one column wide, and
//! the text between two such bytes can be segmented and measured on its own.
//!
//! [`Graphemes`] and [`GraphemesRev`] walk clusters from either end, yielding runs of ASCII
//! straight from byte comparisons and handing everything else to a real segmenter.
//! [`measure_width`] sums display width the same way. Sharing the cut points is what makes the
//! walkers and the measurement agree with each other by construction.

use core::convert::TryInto;

use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

/// A grapheme cluster as the truncation scans walk over it: where it starts, how many bytes it
/// takes and how many columns it occupies.
#[derive(Clone, Copy)]
pub(crate) struct Grapheme {
    pub(crate) start: usize,
    pub(crate) len: usize,
    pub(crate) width: usize,
}

impl Grapheme {
    #[inline]
    pub(crate) fn end(&self) -> usize {
        self.start.saturating_add(self.len)
    }
}

/// Whether the byte at `index` is a complete grapheme cluster on its own, decidable from one
/// neighboring byte: a printable ASCII byte breaks from any ASCII byte after it (GB4, GB5 and
/// GB999, and it cannot be the CR of a CR LF pair), and every non-ASCII combining character,
/// joiner or modifier starts with a non-ASCII byte.
#[inline]
fn is_ascii_cluster(bytes: &[u8], index: usize) -> bool {
    matches!(bytes[index], 0x20..=0x7E) && bytes.get(index.wrapping_add(1)).is_none_or(u8::is_ascii)
}

/// The display width of a string, equal to [`width`](UnicodeWidthStr::width) but skipping over
/// runs of ASCII. The cut points are the bytes passing [`is_ascii_cluster`]: the width state
/// machine reaches such a byte in its default state, as every state a following ASCII byte can
/// produce is one no printable ASCII byte reacts to, so the total width is the sum over the
/// pieces. This makes the same positions the provable reset points of both segmentation and
/// width measurement.
#[inline]
pub(crate) fn measure_width(text: &str) -> usize {
    let bytes = text.as_bytes();
    let mut width = 0usize;
    let mut pos = 0usize;
    while pos < bytes.len() {
        if is_ascii_cluster(bytes, pos) {
            width = width.saturating_add(1);
            pos = pos.saturating_add(1);
        } else {
            let start = pos;
            pos = pos.saturating_add(1);
            loop {
                // every byte of a multi byte character has its high bit set, so a whole word of
                // such bytes cannot contain a cut point and is skipped in one comparison
                if let Some(chunk) = bytes.get(pos..pos.saturating_add(8)) {
                    // unwrap is safe as the chunk is exactly eight bytes
                    let word = u64::from_ne_bytes(chunk.try_into().unwrap());
                    if word & 0x8080_8080_8080_8080 == 0x8080_8080_8080_8080 {
                        pos = pos.saturating_add(8);
                        continue;
                    }
                }
                if pos >= bytes.len() || is_ascii_cluster(bytes, pos) {
                    break;
                }
                pos = pos.saturating_add(1);
            }
            // unwrap is safe as both positions sit on ASCII bytes or the ends of the string
            width = width.saturating_add(text.get(start..pos).unwrap().width());
        }
    }
    width
}

/// Iterates over the grapheme clusters of a string like
/// [`grapheme_indices`](UnicodeSegmentation::grapheme_indices), but yields each cluster with its
/// width, and short circuits runs of ASCII: a cluster passing [`is_ascii_cluster`] needs neither
/// segmentation nor a width lookup. Other text is delegated to a real segmenter, which is entered
/// and left only at cluster boundaries next to ASCII, where segmenting the rest of the string in
/// isolation provably matches segmenting the whole.
pub(crate) struct Graphemes<'a> {
    text: &'a str,
    /// The boundary the next cluster starts at.
    pos: usize,
    /// The segmenter for the current run of non-ASCII text and the offset it counts from.
    inner: Option<(usize, unicode_segmentation::GraphemeIndices<'a>)>,
}

impl<'a> Graphemes<'a> {
    #[inline]
    pub(crate) fn new(text: &'a str) -> Self {
        Self::starting_at(text, 0)
    }

    /// Starts walking at `pos`, which must be a cluster boundary of `text`.
    #[inline]
    pub(crate) fn starting_at(text: &'a str, pos: usize) -> Self {
        Graphemes {
            text,
            pos,
            inner: None,
        }
    }
}

impl Iterator for Graphemes<'_> {
    type Item = Grapheme;

    #[inline]
    fn next(&mut self) -> Option<Grapheme> {
        let bytes = self.text.as_bytes();
        if self.pos >= bytes.len() {
            return None;
        }
        if is_ascii_cluster(bytes, self.pos) {
            self.inner = None;
            let cluster = Grapheme {
                start: self.pos,
                len: 1,
                width: 1,
            };
            self.pos = self.pos.saturating_add(1);
            return Some(cluster);
        }
        if self.inner.is_none() {
            // unwrap is safe as pos is a cluster boundary
            let rest = self.text.get(self.pos..).unwrap();
            self.inner = Some((self.pos, rest.grapheme_indices(true)));
        }
        // unwrap is safe as it was just filled in
        let (base, segmenter) = self.inner.as_mut().unwrap();
        // unwrap is safe as pos is not past the end
        let (index, grapheme) = segmenter.next().unwrap();
        let cluster = Grapheme {
            start: base.saturating_add(index),
            len: grapheme.len(),
            width: grapheme.width(),
        };
        self.pos = cluster.end();
        Some(cluster)
    }
}

/// The backwards counterpart of [`Graphemes`].
pub(crate) struct GraphemesRev<'a> {
    text: &'a str,
    /// The boundary the next cluster ends at.
    pos: usize,
    /// The segmenter for the current run of non-ASCII text; it works on a prefix of the string,
    /// so its indices need no offset.
    inner: Option<unicode_segmentation::GraphemeIndices<'a>>,
}

impl<'a> GraphemesRev<'a> {
    #[inline]
    pub(crate) fn new(text: &'a str) -> Self {
        Self::ending_at(text, text.len())
    }

    /// Starts walking backwards from `pos`, which must be a cluster boundary of `text`.
    #[inline]
    pub(crate) fn ending_at(text: &'a str, pos: usize) -> Self {
        GraphemesRev {
            text,
            pos,
            inner: None,
        }
    }
}

impl Iterator for GraphemesRev<'_> {
    type Item = Grapheme;

    #[inline]
    fn next(&mut self) -> Option<Grapheme> {
        let bytes = self.text.as_bytes();
        if self.pos == 0 {
            return None;
        }
        let previous = self.pos.saturating_sub(1);
        if matches!(bytes[previous], 0x20..=0x7E)
            && (previous == 0 || bytes[previous.saturating_sub(1)].is_ascii())
        {
            // the mirror image of is_ascii_cluster: a printable ASCII byte also breaks from any
            // ASCII byte before it
            self.inner = None;
            self.pos = previous;
            return Some(Grapheme {
                start: previous,
                len: 1,
                width: 1,
            });
        }
        if self.inner.is_none() {
            // unwrap is safe as pos is a cluster boundary
            let rest = self.text.get(..self.pos).unwrap();
            self.inner = Some(rest.grapheme_indices(true));
        }
        // unwrap is safe as it was just filled in
        let segmenter = self.inner.as_mut().unwrap();
        // unwrap is safe as pos is not zero
        let (index, grapheme) = segmenter.next_back().unwrap();
        self.pos = index;
        Some(Grapheme {
            start: index,
            len: grapheme.len(),
            width: grapheme.width(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Strings mixing every edge the fast path has to get right: runs of ASCII, ASCII next to
    /// combining marks and joiners, keycap bases whose successor is a variation selector, CR LF,
    /// controls, regional indicator runs and ligatures spanning clusters.
    const TRICKY: &[&str] = &[
        "",
        "a",
        "plain ascii only",
        "y\u{0306}es no\u{0306}",
        "#\u{FE0F}\u{20E3}5 #5",
        "a\r\nb\rc\nd",
        "\t\u{7f}\u{0019}",
        "\u{1F1E6}\u{1F1E7}\u{1F1E8}x\u{1F1E6}\u{1F1E7}",
        "\u{0644}\u{0627}ab\u{0644}\u{0627}",
        "你好 world 你好",
        "\u{1100}\u{1100}\u{1100}a",
        "123\u{1F468}\u{200D}\u{1F469}\u{200D}\u{1F467}\u{200D}\u{1F466}456",
    ];

    #[cfg(feature = "std")]
    #[test]
    fn walks_like_the_segmenter() {
        for &text in TRICKY {
            let expected: Vec<(usize, usize, usize)> = text
                .grapheme_indices(true)
                .map(|(index, grapheme)| (index, grapheme.len(), grapheme.width()))
                .collect();
            let forwards: Vec<(usize, usize, usize)> = Graphemes::new(text)
                .map(|cluster| (cluster.start, cluster.len, cluster.width))
                .collect();
            assert_eq!(forwards, expected, "{text:?}");
            let mut backwards: Vec<(usize, usize, usize)> = GraphemesRev::new(text)
                .map(|cluster| (cluster.start, cluster.len, cluster.width))
                .collect();
            backwards.reverse();
            assert_eq!(backwards, expected, "{text:?}");
        }
    }

    #[test]
    fn measures_like_the_width_tables() {
        for &text in TRICKY {
            assert_eq!(measure_width(text), text.width(), "{text:?}");
        }
    }

    #[cfg(feature = "std")]
    #[test]
    fn resumes_from_any_boundary() {
        for &text in TRICKY {
            let boundaries: Vec<usize> = Graphemes::new(text)
                .map(|cluster| cluster.start)
                .chain(core::iter::once(text.len()))
                .collect();
            for &pos in &boundaries {
                let resumed: Vec<usize> = Graphemes::starting_at(text, pos)
                    .map(|cluster| cluster.start)
                    .collect();
                let expected: Vec<usize> = boundaries
                    .iter()
                    .copied()
                    .filter(|&b| b >= pos && b < text.len())
                    .collect();
                assert_eq!(resumed, expected, "{text:?} from {pos}");
                let mut rev: Vec<usize> = GraphemesRev::ending_at(text, pos)
                    .map(|cluster| cluster.start)
                    .collect();
                rev.reverse();
                let expected: Vec<usize> =
                    boundaries.iter().copied().filter(|&b| b < pos).collect();
                assert_eq!(rev, expected, "{text:?} up to {pos}");
            }
        }
    }
}
