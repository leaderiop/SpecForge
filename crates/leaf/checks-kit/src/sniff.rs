//! What a file is, from its first bytes: an image and its size in pixels, or a media container, by the signatures its
//! format starts with. Nothing is decoded past the header, and a header that does not read is no format.

use crate::svg;

/// The image formats [`image`] tells apart, as their short names.
pub const IMAGES: [&str; 10] = [
    "png", "jpeg", "gif", "webp", "avif", "heic", "bmp", "ico", "tiff", "svg",
];

/// The media containers [`container`] tells apart, as their short names.
pub const CONTAINERS: [&str; 11] = [
    "mp4", "mov", "m4a", "webm", "mkv", "avi", "wav", "mp3", "ogg", "flac", "aac",
];

/// An image: its format, one of [`IMAGES`], and its width and height when its header says them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Image {
    pub format: &'static str,
    pub size: Option<(u64, u64)>,
}

impl Image {
    /// Its pixels, when its size is known.
    pub fn pixels(self) -> Option<u64> {
        self.size
            .map(|(width, height)| width.saturating_mul(height))
    }
}

/// The image `head`, the first bytes of a file, starts: a raster format by its signature, else an SVG by its root
/// element (which has no size in pixels).
pub fn image(head: &[u8]) -> Option<Image> {
    use imagesize::{Compression, ImageType};
    let Ok(kind) = imagesize::image_type(head) else {
        return svg::is_svg(head).then_some(Image {
            format: "svg",
            size: None,
        });
    };
    let format = match kind {
        ImageType::Png => "png",
        ImageType::Jpeg => "jpeg",
        ImageType::Gif => "gif",
        ImageType::Webp => "webp",
        ImageType::Heif(Compression::Av1) => "avif",
        ImageType::Heif(_) => "heic",
        ImageType::Bmp => "bmp",
        ImageType::Ico => "ico",
        ImageType::Tiff => "tiff",
        _ => return None,
    };
    let size = imagesize::blob_size(head)
        .ok()
        .map(|size| (size.width as u64, size.height as u64));
    Some(Image { format, size })
}

/// The media container `head`, the first bytes of a file, starts, one of [`CONTAINERS`].
pub fn container(head: &[u8]) -> Option<&'static str> {
    match head {
        [_, _, _, _, b'f', b't', b'y', b'p', brand @ ..] => iso_brand(brand),
        [_, _, _, _, b'm', b'o', b'o', b'v', ..] => Some("mov"),
        [0x1a, 0x45, 0xdf, 0xa3, rest @ ..] => ebml_doc_type(rest),
        [
            b'R',
            b'I',
            b'F',
            b'F',
            _,
            _,
            _,
            _,
            b'A',
            b'V',
            b'I',
            b' ',
            ..,
        ] => Some("avi"),
        [
            b'R',
            b'I',
            b'F',
            b'F',
            _,
            _,
            _,
            _,
            b'W',
            b'A',
            b'V',
            b'E',
            ..,
        ] => Some("wav"),
        [b'O', b'g', b'g', b'S', ..] => Some("ogg"),
        [b'f', b'L', b'a', b'C', ..] => Some("flac"),
        [b'I', b'D', b'3', _, _, flags, a, b, c, d, ..] => {
            // An ID3v2 tag: its size is four 7-bit bytes, and a footer follows it when its flag says so.
            let size = [*a, *b, *c, *d]
                .iter()
                .fold(0usize, |size, byte| (size << 7) | usize::from(byte & 0x7f));
            let footer = if flags & 0x10 == 0 { 0 } else { 10 };
            match head.get(10 + size + footer..) {
                Some(after) if !after.is_empty() => audio_frame(after).or(Some("mp3")),
                _ => Some("mp3"),
            }
        }
        _ => audio_frame(head),
    }
}

/// An ISO base media file's container by its major brand; an image's brand (AVIF, HEIF) is no media container.
fn iso_brand(brand: &[u8]) -> Option<&'static str> {
    match brand.get(..4)? {
        b"qt  " => Some("mov"),
        b"M4A " | b"M4B " | b"M4P " => Some("m4a"),
        b"avif" | b"avis" | b"heic" | b"heix" | b"hevc" | b"hevx" | b"heim" | b"heis" | b"mif1"
        | b"msf1" => None,
        _ => Some("mp4"),
    }
}

/// An EBML variable-length integer at the start of `bytes`: its value (with its length marker kept when `marker`, as an
/// element id is written) and its length.
fn vint(bytes: &[u8], marker: bool) -> Option<(u64, usize)> {
    let first = *bytes.first()?;
    let length = first.leading_zeros() as usize + 1;
    if length > 8 {
        return None;
    }
    let mut value = if marker {
        u64::from(first)
    } else {
        u64::from(first) & ((1u64 << (8 - length)) - 1)
    };
    for byte in bytes.get(1..length)? {
        value = (value << 8) | u64::from(*byte);
    }
    Some((value, length))
}

/// A Matroska file's container by its EBML header's `DocType`: `webm` or `matroska`.
fn ebml_doc_type(after_id: &[u8]) -> Option<&'static str> {
    let (size, length) = vint(after_id, false)?;
    let header = after_id.get(length..)?;
    let header = &header[..header
        .len()
        .min(usize::try_from(size).unwrap_or(usize::MAX))];
    let mut at = 0;
    while at < header.len() {
        let (id, id_length) = vint(&header[at..], true)?;
        let (size, size_length) = vint(&header[at + id_length..], false)?;
        let start = at + id_length + size_length;
        let end = start.checked_add(usize::try_from(size).ok()?)?;
        if id == 0x4282 {
            return match header.get(start..end)? {
                b"webm" => Some("webm"),
                b"matroska" => Some("mkv"),
                _ => None,
            };
        }
        at = end;
    }
    None
}

/// An MPEG audio frame header: ADTS AAC (layer bits 00), or an MPEG layer III frame with a bitrate and a sample rate.
fn audio_frame(head: &[u8]) -> Option<&'static str> {
    let [0xff, second, third, ..] = *head else {
        return None;
    };
    if second & 0xf6 == 0xf0 {
        return Some("aac");
    }
    let version = (second >> 3) & 0b11;
    let layer = (second >> 1) & 0b11;
    let bitrate = third >> 4;
    let rate = (third >> 2) & 0b11;
    (second & 0xe0 == 0xe0 && version != 0b01 && layer == 0b01 && bitrate != 0b1111 && rate != 0b11)
        .then_some("mp3")
}
