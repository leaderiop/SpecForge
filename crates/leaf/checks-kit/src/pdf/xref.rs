//! Cross-reference sections, tables and streams, and the stream filters they need.

use miniz_oxide::inflate::{TINFLStatus, decompress_to_vec_zlib_with_limit};

use super::lexer::{Dict, Lexer, Obj};
use super::{Budget, Entry, MAX_ENTRIES, Stop, bad};

/// A classic cross-reference table after its `xref` keyword: what it lists first for object `want`, and its trailer.
pub(super) fn table(
    lexer: &mut Lexer<'_>,
    want: Option<u32>,
    budget: &mut Budget,
) -> Result<(Option<Entry>, Dict), Stop> {
    let mut found = None;
    let mut held = 0u64;
    loop {
        if lexer.keyword(b"trailer")? {
            let Obj::Dict(trailer) = lexer.value(0)? else {
                return bad("its trailer is not a dictionary");
            };
            return Ok((found, trailer));
        }
        let first = lexer.int()?;
        let count = lexer.int()?;
        let (Ok(first), Ok(count)) = (u32::try_from(first), u64::try_from(count)) else {
            return bad("a cross-reference subsection is negative");
        };
        held += count;
        if held > MAX_ENTRIES {
            return bad(format!(
                "its cross-reference holds more than {MAX_ENTRIES} entries"
            ));
        }
        budget.scan(count)?;
        for n in 0..count {
            let offset = lexer.int()?;
            let _generation = lexer.int()?;
            lexer.white();
            let kind = lexer.word()?;
            let number = u32::try_from(u64::from(first) + n)
                .map_err(|_| Stop::Bad("an object number is too large".into()))?;
            let entry = match (kind, u64::try_from(offset)) {
                (b"n", Ok(offset)) => Entry::At(offset),
                (b"f", _) => Entry::Free,
                _ => return bad("a cross-reference entry is neither n nor f"),
            };
            if want == Some(number) {
                found.get_or_insert(entry);
            }
        }
    }
}

/// What a cross-reference stream lists first for object `want`.
pub(super) fn xref_stream(
    dict: &Dict,
    data: &[u8],
    want: Option<u32>,
    budget: &mut Budget,
) -> Result<Option<Entry>, Stop> {
    let widths: Vec<usize> = match dict.get(b"W") {
        Some(Obj::Array(items)) if items.len() == 3 => items
            .iter()
            .map(|item| match item {
                Obj::Int(n) if (0..=8).contains(n) => Ok(*n as usize),
                _ => bad("its cross-reference stream's /W is not three widths of 0 to 8"),
            })
            .collect::<Result<_, _>>()?,
        _ => return bad("its cross-reference stream has no /W of three widths"),
    };
    let size = dict.int(b"Size").unwrap_or(0);
    let index: Vec<i64> = match dict.get(b"Index") {
        Some(Obj::Array(items)) => items
            .iter()
            .map(|item| match item {
                Obj::Int(n) => Ok(*n),
                _ => bad("its cross-reference stream's /Index is not numbers"),
            })
            .collect::<Result<_, _>>()?,
        _ => vec![0, size],
    };
    let data = decoded(dict, data, budget)?;
    let row: usize = widths.iter().sum();
    if row == 0 {
        return bad("its cross-reference stream's rows are empty");
    }
    let mut rows = data.chunks_exact(row);
    let mut found = None;
    let mut held = 0u64;
    for pair in index.chunks(2) {
        let [first, count] = pair else {
            return bad("its cross-reference stream's /Index is not pairs");
        };
        let (Ok(first), Ok(count)) = (u32::try_from(*first), u64::try_from(*count)) else {
            return bad("its cross-reference stream's /Index is negative");
        };
        held += count;
        if held > MAX_ENTRIES {
            return bad(format!(
                "its cross-reference holds more than {MAX_ENTRIES} entries"
            ));
        }
        budget.scan(count)?;
        for n in 0..count {
            let Some(bytes) = rows.next() else {
                return bad("its cross-reference stream holds fewer rows than its /Index");
            };
            let mut fields = [0u64; 3];
            let mut at = 0;
            for (field, width) in fields.iter_mut().zip(&widths) {
                *field = bytes[at..at + width]
                    .iter()
                    .fold(0, |n, b| (n << 8) | u64::from(*b));
                at += width;
            }
            let kind = if widths[0] == 0 { 1 } else { fields[0] };
            let entry = match kind {
                0 => Entry::Free,
                1 => Entry::At(fields[1]),
                2 => Entry::InStream {
                    stream: u32::try_from(fields[1]).unwrap_or(u32::MAX),
                },
                // Another type is a reference to null (ISO 32000-1, 7.5.8.3).
                _ => Entry::Free,
            };
            let number = u32::try_from(u64::from(first) + n)
                .map_err(|_| Stop::Bad("an object number is too large".into()))?;
            if want == Some(number) {
                found.get_or_insert(entry);
            }
        }
    }
    Ok(found)
}

/// A stream's data with its filter and predictor undone: none or `FlateDecode`, PNG predictors. What it inflates to is
/// charged to `budget`.
pub(super) fn decoded(dict: &Dict, data: &[u8], budget: &mut Budget) -> Result<Vec<u8>, Stop> {
    let filter = match dict.get(b"Filter") {
        None => None,
        Some(Obj::Name(name)) => Some(name.clone()),
        Some(Obj::Array(items)) if items.len() <= 1 => match items.first() {
            Some(Obj::Name(name)) => Some(name.clone()),
            None => None,
            _ => return bad("a stream's /Filter is not a name"),
        },
        _ => return bad("a stream has more than one filter"),
    };
    let inflated = match filter.as_deref() {
        None => data.to_vec(),
        Some(b"FlateDecode") => {
            decompress_to_vec_zlib_with_limit(data, budget.room()).map_err(|error| {
                let past_room = error.status == TINFLStatus::HasMoreOutput;
                past_room
                    .then(|| budget.exhausted())
                    .flatten()
                    .unwrap_or_else(|| {
                        Stop::Bad("a stream does not inflate, or inflates past 8 MiB".into())
                    })
            })?
        }
        Some(other) => {
            return bad(format!(
                "a stream's filter /{} is not read",
                String::from_utf8_lossy(other)
            ));
        }
    };
    let parms = match dict.get(b"DecodeParms") {
        Some(Obj::Dict(parms)) => parms.clone(),
        Some(Obj::Array(items)) => match items.first() {
            Some(Obj::Dict(parms)) => parms.clone(),
            _ => Dict::default(),
        },
        _ => Dict::default(),
    };
    budget.inflate(inflated.len());
    match parms.int(b"Predictor").unwrap_or(1) {
        1 => Ok(inflated),
        10..=15 => {
            let columns = parms.int(b"Columns").unwrap_or(1);
            let columns = usize::try_from(columns)
                .ok()
                .filter(|c| (1..=1 << 16).contains(c))
                .map_or_else(|| bad("a stream's /Columns is out of range"), Ok)?;
            Ok(unpredicted(&inflated, columns))
        }
        _ => bad("a stream's predictor is not read"),
    }
}

/// PNG rows of `columns` bytes, each after its filter byte, unfiltered (one byte per pixel).
pub(super) fn unpredicted(data: &[u8], columns: usize) -> Vec<u8> {
    let mut out = Vec::with_capacity(data.len());
    let mut previous = vec![0u8; columns];
    for row in data.chunks(columns + 1) {
        let (Some(&filter), bytes) = (row.first(), &row[1.min(row.len())..]) else {
            break;
        };
        let mut current = vec![0u8; columns];
        for (i, &byte) in bytes.iter().enumerate() {
            let left = if i > 0 { current[i - 1] } else { 0 };
            let up = previous[i];
            let corner = if i > 0 { previous[i - 1] } else { 0 };
            current[i] = match filter {
                1 => byte.wrapping_add(left),
                2 => byte.wrapping_add(up),
                3 => byte.wrapping_add(((u16::from(left) + u16::from(up)) / 2) as u8),
                4 => byte.wrapping_add(paeth(left, up, corner)),
                _ => byte,
            };
        }
        out.extend_from_slice(&current[..bytes.len()]);
        previous = current;
    }
    out
}

pub(super) fn paeth(left: u8, up: u8, corner: u8) -> u8 {
    let p = i16::from(left) + i16::from(up) - i16::from(corner);
    let (pa, pb, pc) = (
        (p - i16::from(left)).abs(),
        (p - i16::from(up)).abs(),
        (p - i16::from(corner)).abs(),
    );
    if pa <= pb && pa <= pc {
        left
    } else if pb <= pc {
        up
    } else {
        corner
    }
}
