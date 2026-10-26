//! What a line break is in `.spec` text, read by one module (ADR 0266): a LF, with one CR directly before it part of
//! it. The parse reads every line break as LF (a `"…"` through [`read`], a `"""…"""` and the header line through
//! `str::lines`, the same rule) and the formatter formats [`read`] of its source. A lone CR is no line break: the parse refuses it as E001 (ADR 0276). A writer that
//! changes part of a file reads and writes it in the file's [`LineBreak`].

use std::borrow::Cow;

/// `text` with each CR LF read as LF. A CR with no LF after it stays here (the parse refuses it, ADR 0276); so
/// does an escaped `\r`, which is two characters here. Borrowed when `text` holds no CR LF.
pub fn read(text: &str) -> Cow<'_, str> {
    if text.contains("\r\n") {
        Cow::Owned(text.replace("\r\n", "\n"))
    } else {
        Cow::Borrowed(text)
    }
}

/// The line break a writer writes into a text it changes part of.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineBreak {
    Lf,
    Crlf,
}

impl LineBreak {
    /// `Crlf` when `text` holds a line break and every one is CR LF; `Lf` otherwise: no line break, all LF, or mixed.
    pub fn of(text: &str) -> LineBreak {
        let lf = text.matches('\n').count();
        if lf > 0 && lf == text.matches("\r\n").count() {
            LineBreak::Crlf
        } else {
            LineBreak::Lf
        }
    }

    /// `text`, of this line break, as a writer reads it: [`read`] for `Crlf`, as it is for `Lf` (a mixed text keeps
    /// every byte, so nothing outside what the writer changes moves).
    pub fn lf(self, text: &str) -> Cow<'_, str> {
        match self {
            LineBreak::Crlf => read(text),
            LineBreak::Lf => Cow::Borrowed(text),
        }
    }

    /// `text`, a writer's LF text, written in this line break: each LF made CR LF for `Crlf`; as it is for `Lf`.
    /// `Crlf.write(&Crlf.lf(t)) == t` for every `t` with `LineBreak::of(t) == Crlf`. `text` holds no raw CR LF of its
    /// own (a typed edit spells a CR as the escape `\r`).
    pub fn write(self, text: &str) -> Cow<'_, str> {
        match self {
            LineBreak::Crlf if text.contains('\n') => Cow::Owned(text.replace('\n', "\r\n")),
            _ => Cow::Borrowed(text),
        }
    }

    /// The line break itself: `"\n"` or `"\r\n"`.
    pub fn as_str(self) -> &'static str {
        match self {
            LineBreak::Lf => "\n",
            LineBreak::Crlf => "\r\n",
        }
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
