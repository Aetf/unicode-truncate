use unicode_truncate::Alignment;
use unicode_truncate::UnicodeTruncateStr;

#[test]
fn main() {
    let (rv, w) = "你好吗".unicode_truncate(5);
    assert_eq!(rv, "你好");
    assert_eq!(w, 4);

    let (rv, w) = "你好吗".unicode_truncate_start(5);
    assert_eq!(rv, "好吗");
    assert_eq!(w, 4);

    let rv = "你好吗".unicode_pad(5, Alignment::Left, true);
    assert_eq!(rv, "你好 ");
}
