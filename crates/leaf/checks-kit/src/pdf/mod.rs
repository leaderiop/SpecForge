//! How many pages a PDF has, read from the ranges of it that were served: the header, `startxref` at the end, the
//! cross-reference sections it names (tables and streams, newest first, along `/Prev`), the trailer's `/Root`, the
//! catalog's `/Pages` and its `/Count`, through object streams when they hold them. What is not served yet is a
//! [`Failure::Need`] naming the spans to read next; the `/Linearized` dictionary's `/N` rides along as a hint for when
//! they cannot be read.
//!
//! A file is untrusted: every count, length, nesting and inflated size is capped, and no input panics.

use std::collections::{BTreeMap, BTreeSet};

mod lexer;
mod xref;

use lexer::{Dict, Lexer, Obj, find, rfind};
use xref::{decoded, table, xref_stream};

/// What a first read takes from the start, for the header and the linearization dictionary.
pub const HEAD: u32 = 1024;
/// What a first read takes from the end, for `startxref`, the last cross-reference section and its trailer.
pub const TAIL: u32 = 64 * 1024;
/// What a need asks at an object's offset.
const OBJECT_SPAN: u32 = 64 * 1024;
/// The most a need asks at a cross-reference section's offset.
const SECTION_SPAN: u32 = 1024 * 1024;
/// The most cross-reference sections followed along `/Prev` and `/XRefStm`.
const MAX_SECTIONS: usize = 32;
/// The most entries one section holds.
const MAX_ENTRIES: u64 = 1 << 22;
/// The most bytes a stream inflates to.
const MAX_INFLATED: usize = 8 << 20;
/// The deepest arrays and dictionaries nest.
const MAX_DEPTH: usize = 32;

/// Bytes of the file from `start`.
#[derive(Debug, Clone, Copy)]
pub struct Read<'a> {
    pub start: u64,
    pub bytes: &'a [u8],
}

/// Bytes of the file to read: `len` from `offset`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Span {
    pub offset: u64,
    pub len: u32,
}

/// What the reads say of the page count.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Opened {
    Pages(u64),
    /// The file is a PDF, but its page count cannot be read from it, and why.
    Undetermined(String),
}

/// Why the page count is not known yet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Failure {
    /// The spans that must be read next; `hint` is the page count the linearization dictionary states, if any.
    Need { spans: Vec<Span>, hint: Option<u64> },
    /// The file is no readable PDF, and why.
    Invalid(String),
}

/// The page count of the PDF of length `len` that `reads` serve.
pub fn open(reads: &[Read<'_>], len: u64) -> Result<Opened, Failure> {
    let mut pdf = Pdf {
        file: File { reads, len },
        sections: Vec::new(),
        pending: Vec::new(),
        seen: BTreeSet::new(),
        encrypted: false,
        root: None,
    };
    let hint = pdf.hint();
    match pdf.count() {
        Ok(count) => Ok(Opened::Pages(count)),
        Err(Stop::Short(span)) => Err(Failure::Need {
            spans: vec![span],
            hint,
        }),
        Err(Stop::Bad(why)) => Err(Failure::Invalid(why)),
        Err(Stop::Undetermined(why)) => Ok(Opened::Undetermined(why)),
    }
}

/// Why reading stopped.
#[derive(Debug)]
enum Stop {
    /// The bytes at this span are not served.
    Short(Span),
    Bad(String),
    Undetermined(String),
}

fn bad<T>(why: impl Into<String>) -> Result<T, Stop> {
    Err(Stop::Bad(why.into()))
}

/// The served reads of a file of length `len`.
struct File<'a> {
    reads: &'a [Read<'a>],
    len: u64,
}

impl<'a> File<'a> {
    /// The longest served run from `offset`, and whether it reaches the file's end.
    fn from(&self, offset: u64) -> Option<(&'a [u8], bool)> {
        self.reads
            .iter()
            .filter(|read| read.start <= offset)
            .filter_map(|read| {
                let skip = usize::try_from(offset - read.start).ok()?;
                let bytes = read.bytes.get(skip..)?;
                (!bytes.is_empty() || offset == self.len).then_some(bytes)
            })
            .max_by_key(|bytes| bytes.len())
            .map(|bytes| (bytes, offset + bytes.len() as u64 >= self.len))
    }

    /// The span `want` bytes from `offset` asks, held to the file.
    fn span(&self, offset: u64, want: u32) -> Span {
        let left = self.len.saturating_sub(offset);
        Span {
            offset,
            len: u32::try_from(left.min(u64::from(want))).unwrap_or(want),
        }
    }

    /// The bytes from `offset` to parse, or a need for `want` bytes there.
    fn at(&self, offset: u64, want: u32) -> Result<Lexer<'a>, Stop> {
        if offset >= self.len {
            return bad("its cross-reference points past its end");
        }
        match self.from(offset) {
            Some((bytes, end)) => Ok(Lexer {
                bytes,
                at: 0,
                end,
                need: self.span(offset, want),
            }),
            None => Err(Stop::Short(self.span(offset, want))),
        }
    }
}

/// Where an object is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Entry {
    Free,
    At(u64),
    /// In an object stream; found there by its number, not its index.
    InStream {
        stream: u32,
    },
}

/// One cross-reference section: its entries, and the older sections it names.
struct Section {
    entries: BTreeMap<u32, Entry>,
}

struct Pdf<'a> {
    file: File<'a>,
    /// The sections read, newest first.
    sections: Vec<Section>,
    /// The offsets of sections named and not read yet, the next to read first.
    pending: Vec<u64>,
    seen: BTreeSet<u64>,
    encrypted: bool,
    root: Option<u32>,
}

impl<'a> Pdf<'a> {
    /// The page count the linearization dictionary states, when the head was served and holds one.
    fn hint(&self) -> Option<u64> {
        let (bytes, end) = self.file.from(0)?;
        let mut lexer = Lexer {
            bytes,
            at: 0,
            end,
            need: self.file.span(0, HEAD),
        };
        let number = u32::try_from(lexer.peek_int()?).ok()?;
        let (Obj::Dict(dict), _) = lexer.object(number).ok()? else {
            return None;
        };
        dict.get(b"Linearized")?;
        u64::try_from(dict.int(b"N")?).ok()
    }

    fn count(&mut self) -> Result<u64, Stop> {
        self.header()?;
        let start = self.startxref()?;
        self.pending.push(start);
        let root = loop {
            if let Some(root) = self.root {
                break root;
            }
            if !self.next_section()? {
                return bad("its trailer names no /Root");
            }
        };
        let catalog = self.dict_of(root, "its catalog")?;
        let Some(pages) = catalog.reference(b"Pages") else {
            return bad("its catalog names no /Pages");
        };
        let tree = self.dict_of(pages, "its page tree")?;
        match tree.int(b"Count").and_then(|n| u64::try_from(n).ok()) {
            Some(count) => Ok(count),
            None => bad("its page tree states no /Count"),
        }
    }

    fn header(&self) -> Result<(), Stop> {
        let Some((head, _)) = self.file.from(0) else {
            return Err(Stop::Short(self.file.span(0, HEAD)));
        };
        let within = &head[..head.len().min(HEAD as usize)];
        if find(within, b"%PDF-").is_some() {
            return Ok(());
        }
        if (head.len() as u64) < self.file.len.min(u64::from(HEAD)) {
            return Err(Stop::Short(self.file.span(0, HEAD)));
        }
        bad("it has no %PDF- header in its first 1024 bytes")
    }

    /// The offset `startxref` gives, read from the end of the file.
    fn startxref(&self) -> Result<u64, Stop> {
        let from = self.file.len.saturating_sub(u64::from(TAIL));
        let tail = self
            .file
            .reads
            .iter()
            .filter(|read| {
                read.start <= from && read.start + read.bytes.len() as u64 >= self.file.len
            })
            .find_map(|read| read.bytes.get(usize::try_from(from - read.start).ok()?..));
        let Some(tail) = tail else {
            return Err(Stop::Short(self.file.span(from, TAIL)));
        };
        let Some(at) = rfind(tail, b"startxref") else {
            return bad("it has no startxref at its end");
        };
        let mut lexer = Lexer {
            bytes: &tail[at + 9..],
            at: 0,
            end: true,
            need: self.file.span(from, TAIL),
        };
        let offset = lexer.int()?;
        u64::try_from(offset).map_or_else(|_| bad("its startxref is negative"), Ok)
    }

    /// Read the next pending section; false when none is left.
    fn next_section(&mut self) -> Result<bool, Stop> {
        let Some(offset) = self.pending.first().copied() else {
            return Ok(false);
        };
        if self.sections.len() >= MAX_SECTIONS {
            return bad(format!(
                "it has more than {MAX_SECTIONS} cross-reference sections"
            ));
        }
        let mut lexer = self.file.at(offset, SECTION_SPAN)?;
        lexer.white();
        let (entries, trailer) = if lexer.keyword(b"xref")? {
            table(&mut lexer)?
        } else {
            let number = lexer.peek_int().and_then(|n| u32::try_from(n).ok());
            let Some(number) = number else {
                return bad("its startxref points at no cross-reference");
            };
            let (Obj::Dict(dict), Some(data)) = lexer.object(number)? else {
                return bad("its startxref points at no cross-reference");
            };
            if !dict.is(b"Type", b"XRef") {
                return bad("its startxref points at no cross-reference");
            }
            (xref_stream(&dict, data)?, dict)
        };
        self.pending.remove(0);
        self.seen.insert(offset);
        if self.root.is_none() {
            self.root = trailer.reference(b"Root");
        }
        self.encrypted |= trailer.get(b"Encrypt").is_some();
        // A hybrid file's stream section comes before the older ones.
        let mut next = Vec::new();
        for key in [&b"XRefStm"[..], b"Prev"] {
            if let Some(offset) = trailer.int(key).and_then(|n| u64::try_from(n).ok())
                && !self.seen.contains(&offset)
                && !self.pending.contains(&offset)
            {
                next.push(offset);
            }
        }
        self.pending.splice(0..0, next);
        self.sections.push(Section { entries });
        Ok(true)
    }

    /// Where object `number` is: the newest section that lists it, reading older ones as they are needed.
    fn entry(&mut self, number: u32) -> Result<Entry, Stop> {
        let mut checked = 0;
        loop {
            for section in &self.sections[checked..] {
                if let Some(entry) = section.entries.get(&number) {
                    return Ok(*entry);
                }
            }
            checked = self.sections.len();
            if !self.next_section()? {
                return bad(format!("object {number} is not in its cross-reference"));
            }
        }
    }

    /// The dictionary of object `number`, which is `what`.
    fn dict_of(&mut self, number: u32, what: &str) -> Result<Dict, Stop> {
        let value = match self.entry(number)? {
            Entry::Free => return bad(format!("{what} is a free object")),
            Entry::At(offset) => self.file.at(offset, OBJECT_SPAN)?.object(number)?.0,
            Entry::InStream { stream } => {
                if self.encrypted {
                    return Err(Stop::Undetermined(format!(
                        "{what} is in an encrypted object stream"
                    )));
                }
                self.in_stream(stream, number)?
            }
        };
        match value {
            Obj::Dict(dict) => Ok(dict),
            _ => bad(format!("{what} is not a dictionary")),
        }
    }

    /// Object `number`, the `index`th of object stream `stream`.
    fn in_stream(&mut self, stream: u32, number: u32) -> Result<Obj, Stop> {
        let Entry::At(offset) = self.entry(stream)? else {
            return bad(format!("object stream {stream} is not at an offset"));
        };
        let (Obj::Dict(dict), Some(data)) = self.file.at(offset, SECTION_SPAN)?.object(stream)?
        else {
            return bad(format!("object {stream} is no object stream"));
        };
        let data = decoded(&dict, data)?;
        let (Some(count), Some(first)) = (dict.int(b"N"), dict.int(b"First")) else {
            return bad(format!("object stream {stream} states no /N or /First"));
        };
        let first = usize::try_from(first).map_err(|_| Stop::Bad("a negative /First".into()))?;
        let mut header = Lexer {
            bytes: data.get(..first).unwrap_or(&data),
            at: 0,
            end: true,
            need: self.file.span(offset, SECTION_SPAN),
        };
        let mut at = None;
        for _ in 0..count.clamp(0, MAX_ENTRIES as i64) {
            let (found, offset) = (header.int()?, header.int()?);
            if found == i64::from(number) {
                at = usize::try_from(offset).ok();
                break;
            }
        }
        let Some(at) = at.and_then(|at| first.checked_add(at)) else {
            return bad(format!("object {number} is not in object stream {stream}"));
        };
        let mut lexer = Lexer {
            bytes: data.get(at..).unwrap_or_default(),
            at: 0,
            end: true,
            need: self.file.span(offset, SECTION_SPAN),
        };
        lexer.value(0)
    }
}
