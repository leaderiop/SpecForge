//! The PDF reader against files a builder writes with exact offsets: no binary fixture is committed.

use std::collections::BTreeMap;
use std::fmt::Write as _;

use miniz_oxide::deflate::compress_to_vec_zlib;

use crate::pdf::{Failure, HEAD, Opened, Read, Span, TAIL, open};

/// Writes a PDF one object at a time, keeping each object's offset.
struct Builder {
    out: Vec<u8>,
    offsets: BTreeMap<u32, u64>,
}

/// Where a cross-reference stream says an object is.
enum At {
    Offset(u32),
    InStream(u32, u32),
}

impl Builder {
    fn new() -> Self {
        Builder {
            out: b"%PDF-1.7\n%\xe2\xe3\xcf\xd3\n".to_vec(),
            offsets: BTreeMap::new(),
        }
    }

    fn at(&self) -> u64 {
        self.out.len() as u64
    }

    fn object(&mut self, number: u32, body: &str) -> &mut Self {
        self.offsets.insert(number, self.at());
        self.out
            .extend_from_slice(format!("{number} 0 obj\n{body}\nendobj\n").as_bytes());
        self
    }

    fn stream(&mut self, number: u32, dict: &str, data: &[u8]) -> &mut Self {
        self.offsets.insert(number, self.at());
        let head = format!(
            "{number} 0 obj\n<< {dict} /Length {} >>\nstream\n",
            data.len()
        );
        self.out.extend_from_slice(head.as_bytes());
        self.out.extend_from_slice(data);
        self.out.extend_from_slice(b"\nendstream\nendobj\n");
        self
    }

    fn pad(&mut self, bytes: usize) -> &mut Self {
        self.out.extend_from_slice(b"% ");
        self.out.extend(std::iter::repeat_n(b'.', bytes));
        self.out.push(b'\n');
        self
    }

    /// A classic table of `numbers` (each written since `since`, or all) and its trailer; where it starts.
    fn table(&mut self, numbers: &[u32], trailer: &str) -> u64 {
        let at = self.at();
        let mut text = String::from("xref\n0 1\n0000000000 65535 f \n");
        for number in numbers {
            let offset = self.offsets[number];
            write!(text, "{number} 1\n{offset:010} 00000 n \n")
                .expect("a String takes every write");
        }
        let size = self.offsets.keys().max().map_or(1, |n| n + 1);
        write!(text, "trailer\n<< /Size {size} {trailer} >>\n")
            .expect("a String takes every write");
        self.out.extend_from_slice(text.as_bytes());
        at
    }

    /// A cross-reference stream, object `number`, of `rows`, its rows PNG-Up predicted when `predicted`.
    fn xref_stream(
        &mut self,
        number: u32,
        rows: &[(u32, At)],
        trailer: &str,
        predicted: bool,
    ) -> u64 {
        let at = self.at();
        self.offsets.insert(number, at);
        let mut data = Vec::new();
        let mut index = String::new();
        let mut previous = [0u8; 7];
        let mut all: Vec<(u32, [u8; 7])> = rows
            .iter()
            .map(|(n, at)| {
                let row = match at {
                    At::Offset(offset) => {
                        let o = self.offsets[offset].to_be_bytes();
                        [1, o[4], o[5], o[6], o[7], 0, 0]
                    }
                    At::InStream(stream, i) => {
                        let s = stream.to_be_bytes();
                        let i = (*i as u16).to_be_bytes();
                        [2, s[0], s[1], s[2], s[3], i[0], i[1]]
                    }
                };
                (*n, row)
            })
            .collect();
        let o = at.to_be_bytes();
        all.push((number, [1, o[4], o[5], o[6], o[7], 0, 0]));
        all.sort_by_key(|(n, _)| *n);
        for (n, row) in &all {
            write!(index, "{n} 1 ").expect("a String takes every write");
            if predicted {
                data.push(2);
                data.extend(row.iter().zip(previous).map(|(b, p)| b.wrapping_sub(p)));
                previous = *row;
            } else {
                data.extend_from_slice(row);
            }
        }
        let parms = if predicted {
            "/DecodeParms << /Predictor 12 /Columns 7 >>"
        } else {
            ""
        };
        let size = all.iter().map(|(n, _)| n + 1).max().unwrap_or(1);
        let dict = format!(
            "/Type /XRef /Size {size} /W [1 4 2] /Index [{index}] /Filter /FlateDecode {parms} {trailer}"
        );
        let packed = compress_to_vec_zlib(&data, 6);
        self.stream(number, &dict, &packed);
        at
    }

    /// An object stream, object `number`, holding `objects`.
    fn object_stream(&mut self, number: u32, objects: &[(u32, &str)]) -> &mut Self {
        let mut header = String::new();
        let mut body = String::new();
        for (n, text) in objects {
            write!(header, "{n} {} ", body.len()).expect("a String takes every write");
            body.push_str(text);
            body.push('\n');
        }
        let first = header.len();
        let data = compress_to_vec_zlib(format!("{header}{body}").as_bytes(), 6);
        let dict = format!(
            "/Type /ObjStm /N {} /First {first} /Filter /FlateDecode",
            objects.len()
        );
        self.stream(number, &dict, &data)
    }

    fn finish(&mut self, startxref: u64) -> Vec<u8> {
        self.out
            .extend_from_slice(format!("startxref\n{startxref}\n%%EOF\n").as_bytes());
        std::mem::take(&mut self.out)
    }
}

/// The whole file, served in one read.
fn whole(bytes: &[u8]) -> Result<Opened, Failure> {
    open(&[Read { start: 0, bytes }], bytes.len() as u64)
}

/// The file as a first round serves it: its head and its tail.
fn ends(bytes: &[u8]) -> Vec<Read<'_>> {
    let len = bytes.len();
    let tail = len.saturating_sub(TAIL as usize);
    vec![
        Read {
            start: 0,
            bytes: &bytes[..len.min(HEAD as usize)],
        },
        Read {
            start: tail as u64,
            bytes: &bytes[tail..],
        },
    ]
}

/// The file read as the guest reads it: its ends, then each span the reader needs, at most `rounds` rounds in all.
fn rounds(bytes: &[u8], rounds: usize) -> Result<Opened, Failure> {
    let mut reads = ends(bytes);
    let len = bytes.len() as u64;
    for _ in 1..rounds {
        match open(&reads, len) {
            Err(Failure::Need { spans, .. }) => {
                for Span { offset, len } in spans {
                    let start = offset as usize;
                    let end = (start + len as usize).min(bytes.len());
                    reads.push(Read {
                        start: offset,
                        bytes: &bytes[start..end],
                    });
                }
            }
            done => return done,
        }
    }
    open(&reads, len)
}

const CATALOG: &str = "<< /Type /Catalog /Pages 2 0 R >>";

fn pages(count: u32) -> String {
    format!("<< /Type /Pages /Kids [] /Count {count} >>")
}

fn classic() -> Vec<u8> {
    let mut b = Builder::new();
    b.object(1, CATALOG).object(2, &pages(3));
    let xref = b.table(&[1, 2], "/Root 1 0 R");
    b.finish(xref)
}

fn incremental() -> Vec<u8> {
    let mut b = Builder::new();
    b.object(1, CATALOG).object(2, &pages(1));
    let first = b.table(&[1, 2], "/Root 1 0 R");
    b.out
        .extend_from_slice(format!("startxref\n{first}\n%%EOF\n").as_bytes());
    // The update rewrites the page tree only: the catalog is read through /Prev.
    b.object(2, &pages(5));
    let update = b.table(&[2], &format!("/Root 1 0 R /Prev {first}"));
    b.finish(update)
}

fn xref_stream() -> Vec<u8> {
    let mut b = Builder::new();
    b.object(1, CATALOG).object(2, &pages(4));
    let rows = [(1, At::Offset(1)), (2, At::Offset(2))];
    let xref = b.xref_stream(3, &rows, "/Root 1 0 R", true);
    b.finish(xref)
}

fn object_stream(encrypted: bool) -> Vec<u8> {
    let mut b = Builder::new();
    let tree = pages(7);
    b.object_stream(5, &[(1, CATALOG), (2, &tree)]);
    b.object(9, "<< /Filter /Standard /V 2 >>");
    let rows = [
        (1, At::InStream(5, 0)),
        (2, At::InStream(5, 1)),
        (5, At::Offset(5)),
        (9, At::Offset(9)),
    ];
    let trailer = if encrypted {
        "/Root 1 0 R /Encrypt 9 0 R"
    } else {
        "/Root 1 0 R"
    };
    let xref = b.xref_stream(6, &rows, trailer, false);
    b.finish(xref)
}

fn encrypted_plain() -> Vec<u8> {
    let mut b = Builder::new();
    b.object(1, CATALOG).object(2, &pages(2));
    b.object(9, "<< /Filter /Standard /V 2 >>");
    let xref = b.table(&[1, 2, 9], "/Root 1 0 R /Encrypt 9 0 R");
    b.finish(xref)
}

/// A file whose catalog sits mid-file, past the head, before the tail; linearized with `/N 9` when `linearized`.
fn mid_file(linearized: bool) -> Vec<u8> {
    let mut b = Builder::new();
    if linearized {
        b.object(10, "<< /Linearized 1 /L 0 /N 9 >>");
    }
    b.pad(200 * 1024);
    b.object(1, CATALOG).object(2, &pages(9));
    b.pad(200 * 1024);
    let mut numbers = vec![1, 2];
    if linearized {
        numbers.push(10);
    }
    let xref = b.table(&numbers, "/Root 1 0 R");
    b.finish(xref)
}

#[test]
fn every_layout_counts_its_pages() {
    assert_eq!(whole(&classic()), Ok(Opened::Pages(3)));
    assert_eq!(whole(&incremental()), Ok(Opened::Pages(5)));
    assert_eq!(whole(&xref_stream()), Ok(Opened::Pages(4)));
    assert_eq!(whole(&object_stream(false)), Ok(Opened::Pages(7)));
    assert_eq!(whole(&encrypted_plain()), Ok(Opened::Pages(2)));
    assert_eq!(whole(&mid_file(true)), Ok(Opened::Pages(9)));
    // A small file is whole in its two first reads.
    assert_eq!(rounds(&classic(), 1), Ok(Opened::Pages(3)));
}

#[test]
fn a_pdf_page_tree_in_an_encrypted_object_stream_is_undetermined() {
    let Ok(Opened::Undetermined(why)) = whole(&object_stream(true)) else {
        panic!("not undetermined");
    };
    assert!(why.contains("encrypted object stream"), "{why}");
}

#[test]
fn a_catalog_outside_the_served_ranges_is_a_need_for_its_span() {
    let file = mid_file(true);
    let reads = ends(&file);
    let catalog = file.windows(7).position(|w| w == b"1 0 obj").unwrap() as u64;
    let Err(Failure::Need { spans, hint }) = open(&reads, file.len() as u64) else {
        panic!("no need");
    };
    assert_eq!(
        spans,
        [Span {
            offset: catalog,
            len: 64 * 1024
        }]
    );
    assert_eq!(hint, Some(9), "the linearization dictionary's /N");
    // Read in a second round, it counts.
    assert_eq!(rounds(&file, 2), Ok(Opened::Pages(9)));
    let plain = mid_file(false);
    let Err(Failure::Need { hint: None, .. }) = open(&ends(&plain), plain.len() as u64) else {
        panic!("a file with no linearization dictionary has no hint");
    };
}

#[test]
fn a_file_that_is_no_pdf_is_invalid() {
    let Err(Failure::Invalid(why)) = whole(b"GIF89a not a pdf at all") else {
        panic!("read as a pdf");
    };
    assert!(why.contains("%PDF-"), "{why}");
    let mut cut = classic();
    let at = cut.windows(9).rposition(|w| w == b"startxref").unwrap();
    cut.truncate(at);
    let Err(Failure::Invalid(why)) = whole(&cut) else {
        panic!("read with no startxref");
    };
    assert!(why.contains("startxref"), "{why}");
}

#[test]
fn no_truncation_of_any_fixture_panics() {
    for file in [
        classic(),
        incremental(),
        xref_stream(),
        object_stream(false),
        object_stream(true),
        encrypted_plain(),
    ] {
        for cut in 0..=file.len() {
            let _ = whole(&file[..cut]);
            let reads = [
                Read {
                    start: 0,
                    bytes: &file[..cut.min(HEAD as usize)],
                },
                Read {
                    start: cut as u64,
                    bytes: &file[cut..],
                },
            ];
            let _ = open(&reads, file.len() as u64);
        }
        // Every byte, flipped.
        for at in 0..file.len() {
            let mut flipped = file.clone();
            flipped[at] ^= 0x5a;
            let _ = whole(&flipped);
        }
    }
}
