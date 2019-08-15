#![no_std]

use unicode_truncate::UnicodeTruncateStr;

#[test]
fn main() {
    let (rv, w) = "你好吗".unicode_truncate(5);
    assert_eq!(rv, "你好");
    assert_eq!(w, 4);
}
