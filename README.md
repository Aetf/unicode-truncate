# unicode-truncate
Unicode-aware algorithm to pad or truncate `str` in terms of displayed width.

[![Build Status](https://travis-ci.org/Aetf/unicode-truncate.svg)](https://travis-ci.org/Aetf/unicode-truncate)
[![Documentation](https://docs.rs/unicode-truncate/badge.svg)](https://docs.rs/unicode-truncate)

## examples
Safely truncate string to display width even not at character boundaries.
```rust
use unicode_truncate::UnicodeTruncateStr;

fn main() {
    let (rv, w) = "你好吗".unicode_truncate(5);
    assert_eq!(rv, "你好");
    assert_eq!(w, 4);
}
```

Making sure the string is displayed in exactly number of columns by combining padding
and truncating.
```rust
use unicode_truncate::UnicodeTruncateStr;
use unicode_truncate::Alignment;
use unicode_width::UnicodeWidthStr;

fn main() {
    let rv = "你好吗".unicode_pad(5, Alignment::Left, true);
    assert_eq!(rv, "你好 ");
    assert_eq!(rv.width(), 5);
}
```

## features
`unicode-truncate` can be built without libstd by disabling the default feature `std`. However in that
case `unicode_truncate::UnicodeTruncateStr::unicode_pad` won't be available because it depends on
`std::string::String` and `std::borrow::Cow`.
