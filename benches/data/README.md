# Benchmark data

Texts covering the shapes that change how much work truncation does: how many bytes a grapheme
takes, how many columns it occupies, and how many code points a grapheme cluster spans.

| File | Content | Source |
| --- | --- | --- |
| `zhufu.txt` | Chinese, wide graphemes | 《祝福》, Lu Xun, 1924, public domain |
| `alice.txt` | English, narrow single byte graphemes | [Alice's Adventures in Wonderland](https://www.gutenberg.org/ebooks/11), Lewis Carroll, 1865, public domain, Project Gutenberg boilerplate removed |
| `kalila.txt` | Arabic, narrow multi byte graphemes, ligatures spanning graphemes | [كليلة ودمنة](https://ar.wikisource.org/wiki/كليلة_ودمنة), Ibn al-Muqaffa', 8th century, public domain, via Arabic Wikisource |
| `emoji.txt` | Emoji ZWJ sequences, skin tone modifiers, flags, keycaps and stacked combining marks mixed into English | generated |

`emoji.txt` is generated, pseudo random with a fixed seed, from a fixed list of words and
graphemes. It is checked in rather than generated at benchmark time to keep results comparable.
