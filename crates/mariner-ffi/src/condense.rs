//! Condense's document, for drawing articles with native views.
//!
//! The shapes follow `mariner_condense`, with two changes for the apps:
//! - Span offsets are UTF-16 code units, which Kotlin strings and
//!   `NSRange` use, not UTF-8 bytes.
//! - Tables come as their cells placed on the grid, which is what a grid
//!   layout needs, rather than a slot per position.

use std::collections::HashMap;

use mariner_condense as condense;

use crate::error::MarinerError;

/// Converts article HTML into blocks. Relative URLs resolve against
/// `base_url`; pass an empty string when there is none. It's CPU work, so
/// call it off the main thread.
#[uniffi::export]
pub fn condense(html: String, base_url: String) -> Result<ArticleDocument, MarinerError> {
    let document =
        condense::condense(&html, &base_url).map_err(|error| MarinerError::Condense {
            reason: error.to_string(),
        })?;
    Ok(document.into())
}

#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct ArticleDocument {
    /// Content blocks in reading order.
    pub blocks: Vec<Block>,
    /// Present only when a cap was hit and conversion stopped early.
    pub truncation: Option<Truncation>,
    /// HTML `id` -> index into `blocks` of the top-level block containing
    /// it, for in-page `#anchor` links.
    pub anchors: HashMap<String, u32>,
    /// Every image URL, including images nested in lists, quotes and tables.
    pub image_urls: Vec<String>,
}

/// How much was kept when a cap stopped conversion.
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Record)]
pub struct Truncation {
    pub blocks: u32,
    pub chars: u32,
}

/// One unit of content. Only list items, quotes and table cells contain
/// other blocks.
#[derive(Debug, Clone, PartialEq, uniffi::Enum)]
pub enum Block {
    Text { block: TextBlock },
    Image { block: ImageBlock },
    Video { block: VideoBlock },
    Audio { block: AudioBlock },
    ListItem { block: ListItemBlock },
    Quote { block: QuoteBlock },
    Table { block: TableBlock },
}

/// One paragraph, heading or code block.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct TextBlock {
    /// HTML `id`s inside the block.
    pub ids: Vec<String>,
    pub text: String,
    /// Sorted by `start`, then longest first. Spans may overlap and nest.
    pub spans: Vec<TextSpan>,
    pub kind: TextKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum TextKind {
    /// Normal prose, with whitespace collapsed.
    Prose,
    /// `<pre>` content, whitespace preserved.
    Preformatted,
    /// `<pre><code>` content, whitespace preserved.
    Code,
}

/// A style over `text`, from `start` (inclusive) to `end` (exclusive), in
/// UTF-16 code units.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct TextSpan {
    pub style: TextStyle,
    pub start: u32,
    pub end: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Enum)]
pub enum TextStyle {
    /// Levels 1 through 6.
    Heading {
        level: u8,
    },
    Bold,
    Italic,
    Underline,
    Strikethrough,
    Superscript,
    Subscript,
    Monospace,
    Code,
    /// From legacy `<font face>`.
    Font {
        face: String,
    },
    /// The resolved URL, or the raw `href` when it can't be resolved.
    Link {
        url: String,
    },
}

#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct ImageBlock {
    pub ids: Vec<String>,
    /// Candidate sources, in document order, without duplicates.
    pub sources: Vec<ImageSource>,
    /// From `<figcaption>`, or the image's `alt` text.
    pub caption: Option<TextBlock>,
    /// The URL of the link wrapping the image.
    pub link: Option<String>,
}

#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct ImageSource {
    pub url: String,
    /// From the `width` attribute.
    pub width: Option<u32>,
    /// From the `height` attribute.
    pub height: Option<u32>,
    /// From a `srcset` width descriptor (`640w`).
    pub srcset_width: Option<u32>,
    /// From a `srcset` pixel-density descriptor (`2x`).
    pub density: Option<f32>,
}

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct VideoBlock {
    pub ids: Vec<String>,
    pub sources: Vec<VideoSource>,
}

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct VideoSource {
    /// A playable file or an embed URL.
    pub url: String,
    /// A page to open externally, such as a YouTube watch page.
    pub link: Option<String>,
    pub thumbnail: Option<String>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub mime_type: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct AudioBlock {
    pub ids: Vec<String>,
    pub sources: Vec<AudioSource>,
}

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct AudioSource {
    pub url: String,
    pub mime_type: Option<String>,
}

#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct ListItemBlock {
    pub ids: Vec<String>,
    /// Present for ordered lists only.
    pub number: Option<i32>,
    pub blocks: Vec<Block>,
}

#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct QuoteBlock {
    pub ids: Vec<String>,
    /// The resolved `cite` URL.
    pub cite: Option<String>,
    pub blocks: Vec<Block>,
}

/// A table of at least two rows and two columns. Column 0 is the visual
/// left edge: right-to-left tables are mirrored.
#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct TableBlock {
    pub ids: Vec<String>,
    pub rows: u32,
    pub cols: u32,
    /// Each cell at its top-left grid position, in row-major order.
    /// Positions a cell spans over, or that no cell fills, are absent.
    pub cells: Vec<TableCell>,
}

#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct TableCell {
    pub row: u32,
    pub col: u32,
    pub row_span: u32,
    pub col_span: u32,
    pub kind: CellKind,
    pub blocks: Vec<Block>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum CellKind {
    Header,
    Data,
}

impl From<condense::ArticleDocument> for ArticleDocument {
    fn from(document: condense::ArticleDocument) -> Self {
        ArticleDocument {
            blocks: blocks(document.blocks),
            truncation: document.truncation.map(|truncation| Truncation {
                blocks: count(truncation.blocks),
                chars: count(truncation.chars),
            }),
            anchors: document
                .anchors
                .into_iter()
                .map(|(id, index)| (id, count(index)))
                .collect(),
            image_urls: document.image_urls.into_iter().collect(),
        }
    }
}

fn blocks(blocks: Vec<condense::Block>) -> Vec<Block> {
    blocks.into_iter().map(Block::from).collect()
}

impl From<condense::Block> for Block {
    fn from(block: condense::Block) -> Self {
        match block {
            condense::Block::Text(block) => Block::Text {
                block: block.into(),
            },
            condense::Block::Image(image) => Block::Image {
                block: ImageBlock {
                    ids: image.ids.into_iter().collect(),
                    sources: image
                        .sources
                        .into_iter()
                        .map(|source| ImageSource {
                            url: source.url,
                            width: source.width,
                            height: source.height,
                            srcset_width: source.srcset_width,
                            density: source.density,
                        })
                        .collect(),
                    caption: image.caption.map(Into::into),
                    link: image.link,
                },
            },
            condense::Block::Video(video) => Block::Video {
                block: VideoBlock {
                    ids: video.ids.into_iter().collect(),
                    sources: video
                        .sources
                        .into_iter()
                        .map(|source| VideoSource {
                            url: source.url,
                            link: source.link,
                            thumbnail: source.thumbnail,
                            width: source.width,
                            height: source.height,
                            mime_type: source.mime_type,
                        })
                        .collect(),
                },
            },
            condense::Block::Audio(audio) => Block::Audio {
                block: AudioBlock {
                    ids: audio.ids.into_iter().collect(),
                    sources: audio
                        .sources
                        .into_iter()
                        .map(|source| AudioSource {
                            url: source.url,
                            mime_type: source.mime_type,
                        })
                        .collect(),
                },
            },
            condense::Block::ListItem(item) => Block::ListItem {
                block: ListItemBlock {
                    ids: item.ids.into_iter().collect(),
                    number: item.number,
                    blocks: blocks(item.blocks),
                },
            },
            condense::Block::Quote(quote) => Block::Quote {
                block: QuoteBlock {
                    ids: quote.ids.into_iter().collect(),
                    cite: quote.cite,
                    blocks: blocks(quote.blocks),
                },
            },
            condense::Block::Table(table) => Block::Table {
                block: table.into(),
            },
        }
    }
}

impl From<condense::TextBlock> for TextBlock {
    fn from(block: condense::TextBlock) -> Self {
        let spans = block
            .spans
            .into_iter()
            .map(|span| TextSpan {
                start: utf16_offset(&block.text, span.start),
                end: utf16_offset(&block.text, span.end),
                style: span.style.into(),
            })
            .collect();
        TextBlock {
            ids: block.ids.into_iter().collect(),
            text: block.text,
            spans,
            kind: match block.kind {
                condense::TextKind::Prose => TextKind::Prose,
                condense::TextKind::Preformatted => TextKind::Preformatted,
                condense::TextKind::Code => TextKind::Code,
            },
        }
    }
}

/// The UTF-16 offset of the UTF-8 byte offset `byte` in `text`.
fn utf16_offset(text: &str, byte: usize) -> u32 {
    let prefix = text.get(..byte).unwrap_or(text);
    count(prefix.encode_utf16().count())
}

impl From<condense::Style> for TextStyle {
    fn from(style: condense::Style) -> Self {
        match style {
            condense::Style::Heading { level } => TextStyle::Heading { level },
            condense::Style::Bold => TextStyle::Bold,
            condense::Style::Italic => TextStyle::Italic,
            condense::Style::Underline => TextStyle::Underline,
            condense::Style::Strikethrough => TextStyle::Strikethrough,
            condense::Style::Superscript => TextStyle::Superscript,
            condense::Style::Subscript => TextStyle::Subscript,
            condense::Style::Monospace => TextStyle::Monospace,
            condense::Style::Code => TextStyle::Code,
            condense::Style::Font { face } => TextStyle::Font { face },
            condense::Style::Link { url } => TextStyle::Link { url },
        }
    }
}

impl From<condense::TableBlock> for TableBlock {
    fn from(table: condense::TableBlock) -> Self {
        let cells = table
            .layout()
            .into_iter()
            .map(|placed| TableCell {
                row: count(placed.row),
                col: count(placed.col),
                row_span: count(placed.row_span),
                col_span: count(placed.col_span),
                kind: match placed.cell.kind {
                    condense::CellKind::Header => CellKind::Header,
                    condense::CellKind::Data => CellKind::Data,
                },
                blocks: blocks(placed.cell.blocks.clone()),
            })
            .collect();
        TableBlock {
            ids: table.ids.into_iter().collect(),
            rows: count(table.rows),
            cols: count(table.cols),
            cells,
        }
    }
}

/// Counts are capped well below `u32::MAX` by condense's limits.
fn count(value: usize) -> u32 {
    u32::try_from(value).unwrap_or(u32::MAX)
}
