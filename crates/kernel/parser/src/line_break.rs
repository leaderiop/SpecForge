//! What a line break is in `.spec` text, read by one module (ADR 0266): a LF, with one CR directly before it part of
//! it. The parse reads every line break as LF (a `"…"` through [`read`], a `"""…"""` and the header line through
//! `str::lines`, the same rule) and the formatter formats [`read`] of its source. A lone CR is text.

use std::borrow::Cow;

/// `text` with each CR LF read as LF. A CR with no LF after it is text and stays; so does an escaped `\r`, which is
/// two characters here. Borrowed when `text` holds no CR LF.
pub fn read(text: &str) -> Cow<'_, str> {
    if text.contains("\r\n") {
        Cow::Owned(text.replace("\r\n", "\n"))
    } else {
        Cow::Borrowed(text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_keeps_a_lone_cr_and_borrows_a_text_with_no_cr_lf() {
        assert_eq!(read("a\r\nb\r\r\nc"), "a\nb\r\nc");
        assert_eq!(read("a\rb\\r\\nc"), "a\rb\\r\\nc");
        assert!(matches!(read("a\rb\nc"), Cow::Borrowed(_)));
    }
}
