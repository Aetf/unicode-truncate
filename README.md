# unicode-truncate
Unicode-aware, `O(n)` algorithm to pad or truncate `str` in terms of displayed width.

[![Build Status](https://travis-ci.org/Aetf/unicode-truncate.svg)](https://travis-ci.org/Aetf/unicode-truncate)
[![Documentation](https://docs.rs/unicode-truncate/badge.svg)](https://docs.rs/unicode-truncate)

```rust
use unicode_truncate::UnicodeTruncateStr;

fn main() {
    let rv = "你好吗".unicode_pad(5, Alignment::Left, true);
    assert_eq!(&rv, "你好 ");
    assert_eq!(rv.len(), 5);
}
```

## features
