//! Turns a document's bytes into text.

use encoding_rs::{Encoding, UTF_8};

pub(crate) struct Decoded {
    pub text: String,
    /// Whether the XML declaration names an encoding. Affects how XHTML
    /// content is serialized (see `xhtml`).
    pub declared_encoding: bool,
}

/// Decodes by byte order mark, then the XML declaration's encoding, then
/// the `charset` hint, then UTF-8. Bytes invalid in the chosen encoding
/// become U+FFFD.
pub(crate) fn decode(bytes: &[u8], charset: Option<&str>) -> Decoded {
    let declared = declared_encoding(bytes);
    let declared_encoding = declared.is_some();
    let encoding = Encoding::for_bom(bytes)
        .map(|(encoding, _)| encoding)
        .or_else(|| declared.and_then(|label| Encoding::for_label(label.as_bytes())))
        .or_else(|| {
            if declared_encoding {
                return None;
            }
            charset.and_then(|label| Encoding::for_label(label.trim().as_bytes()))
        })
        .unwrap_or(UTF_8);
    // A declaration that claims UTF-16 in a document read as bytes without
    // a BOM is wrong about itself; ASCII-compatible decoding still works.
    let encoding = if encoding.is_single_byte() || encoding == UTF_8 || has_bom(bytes) {
        encoding
    } else {
        UTF_8
    };
    let (text, _, _) = encoding.decode(bytes);
    Decoded {
        text: text.into_owned(),
        declared_encoding,
    }
}

fn has_bom(bytes: &[u8]) -> bool {
    Encoding::for_bom(bytes).is_some()
}

/// The `encoding` pseudo-attribute of a leading `<?xml ... ?>` declaration.
fn declared_encoding(bytes: &[u8]) -> Option<String> {
    let start = bytes.iter().position(|byte| !byte.is_ascii_whitespace())?;
    let rest = &bytes[start..];
    let rest = rest.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(rest);
    if !rest.starts_with(b"<?xml") {
        return None;
    }
    let end = rest.windows(2).position(|pair| pair == b"?>")?;
    let declaration = std::str::from_utf8(&rest[..end]).ok()?;
    let at = declaration.find("encoding")?;
    let value = declaration[at + "encoding".len()..].trim_start();
    let value = value.strip_prefix('=')?.trim_start();
    let quote = value.chars().next().filter(|c| *c == '"' || *c == '\'')?;
    let value = &value[1..];
    let close = value.find(quote)?;
    Some(value[..close].trim().to_string())
}
