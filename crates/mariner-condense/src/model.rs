use std::collections::{BTreeMap, BTreeSet};
use std::ops::Range;

use serde::{Deserialize, Serialize};

/// HTML `id`s that ended up inside a block.
pub type Ids = BTreeSet<String>;

/// The result of condensing an article.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ArticleDocument {
    /// Content blocks in reading order.
    pub blocks: Vec<Block>,
    /// Present only when a cap was hit and conversion stopped early.
    pub truncation: Option<Truncation>,
    /// HTML `id` -> index into [`Self::blocks`] of the top-level block
    /// containing it, for in-page `#anchor` links.
    pub anchors: BTreeMap<String, usize>,
    /// Every image URL in the document, including images nested inside
    /// lists, quotes and tables.
    pub image_urls: BTreeSet<String>,
}

impl ArticleDocument {
    /// The top-level block containing the element with HTML `id`.
    pub fn block_for_anchor(&self, id: &str) -> Option<&Block> {
        self.anchors
            .get(id)
            .and_then(|&index| self.blocks.get(index))
    }

    pub(crate) fn assemble(blocks: Vec<Block>, truncation: Option<Truncation>) -> Self {
        let mut anchors = BTreeMap::new();
        let mut image_urls = BTreeSet::new();
        for (index, block) in blocks.iter().enumerate() {
            block.visit(&mut |block| {
                for id in block.ids() {
                    anchors.entry(id.clone()).or_insert(index);
                }
                if let Block::Image(image) = block {
                    if let Some(caption) = &image.caption {
                        for id in &caption.ids {
                            anchors.entry(id.clone()).or_insert(index);
                        }
                    }
                    image_urls.extend(image.sources.iter().map(|source| source.url.clone()));
                }
            });
        }
        Self {
            blocks,
            truncation,
            anchors,
            image_urls,
        }
    }
}

/// How much output was kept when a cap stopped conversion.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Truncation {
    /// Blocks kept, nested blocks included.
    pub blocks: usize,
    /// Characters kept across all text blocks.
    pub chars: usize,
}

/// One unit of content. Only list items, quotes and table cells contain
/// other blocks.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Block {
    Text(TextBlock),
    Image(ImageBlock),
    Video(VideoBlock),
    Audio(AudioBlock),
    ListItem(ListItemBlock),
    Quote(QuoteBlock),
    Table(TableBlock),
}

impl Block {
    pub fn ids(&self) -> &Ids {
        match self {
            Block::Text(block) => &block.ids,
            Block::Image(block) => &block.ids,
            Block::Video(block) => &block.ids,
            Block::Audio(block) => &block.ids,
            Block::ListItem(block) => &block.ids,
            Block::Quote(block) => &block.ids,
            Block::Table(block) => &block.ids,
        }
    }

    pub(crate) fn ids_mut(&mut self) -> &mut Ids {
        match self {
            Block::Text(block) => &mut block.ids,
            Block::Image(block) => &mut block.ids,
            Block::Video(block) => &mut block.ids,
            Block::Audio(block) => &mut block.ids,
            Block::ListItem(block) => &mut block.ids,
            Block::Quote(block) => &mut block.ids,
            Block::Table(block) => &mut block.ids,
        }
    }

    /// Calls `f` on this block and every block nested inside it, depth first.
    pub fn visit<'a>(&'a self, f: &mut impl FnMut(&'a Block)) {
        f(self);
        match self {
            Block::ListItem(block) => block.blocks.iter().for_each(|child| child.visit(f)),
            Block::Quote(block) => block.blocks.iter().for_each(|child| child.visit(f)),
            Block::Table(table) => {
                for slot in &table.slots {
                    if let TableSlot::Cell(cell) = slot {
                        cell.blocks.iter().for_each(|child| child.visit(f));
                    }
                }
            }
            Block::Text(_) | Block::Image(_) | Block::Video(_) | Block::Audio(_) => {}
        }
    }
}

/// One paragraph, heading or code block.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TextBlock {
    pub ids: Ids,
    pub text: String,
    /// Sorted by `start`, then longest first. Spans may overlap and nest.
    pub spans: Vec<Span>,
    pub kind: TextKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TextKind {
    /// Normal prose, with whitespace collapsed.
    Prose,
    /// `<pre>` content, whitespace preserved.
    Preformatted,
    /// `<pre><code>` content, whitespace preserved.
    Code,
}

/// A style applied to `text[start..end]`: UTF-8 byte offsets, `start`
/// inclusive and `end` exclusive.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Span {
    pub style: Style,
    pub start: usize,
    pub end: usize,
}

impl Span {
    pub fn range(&self) -> Range<usize> {
        self.start..self.end
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "style", rename_all = "snake_case")]
pub enum Style {
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ImageBlock {
    pub ids: Ids,
    /// Candidate sources, in document order, without duplicates.
    pub sources: Vec<ImageSource>,
    /// From `<figcaption>`, or the image's `alt` text with HTML stripped.
    pub caption: Option<TextBlock>,
    /// The URL of the link wrapping the image, if any.
    pub link: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VideoBlock {
    pub ids: Ids,
    pub sources: Vec<VideoSource>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AudioBlock {
    pub ids: Ids,
    pub sources: Vec<AudioSource>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AudioSource {
    pub url: String,
    pub mime_type: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ListItemBlock {
    pub ids: Ids,
    /// 1-based by default; present for ordered lists only.
    pub number: Option<i32>,
    pub blocks: Vec<Block>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QuoteBlock {
    pub ids: Ids,
    /// The resolved `cite` URL.
    pub cite: Option<String>,
    /// Text only: media inside a quote is emitted outside it.
    pub blocks: Vec<Block>,
}

/// A table with at least two rows and two columns. Column 0 is always the
/// visual left edge: right-to-left tables are mirrored.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TableBlock {
    pub ids: Ids,
    pub rows: usize,
    pub cols: usize,
    /// `rows * cols` slots in row-major order.
    pub slots: Vec<TableSlot>,
}

impl TableBlock {
    pub fn slot(&self, row: usize, col: usize) -> Option<&TableSlot> {
        if row >= self.rows || col >= self.cols {
            return None;
        }
        self.slots.get(row * self.cols + col)
    }

    /// The cell whose top-left corner is at (`row`, `col`). Positions covered
    /// by a spanning cell return `None`; see [`Self::slot`].
    pub fn cell(&self, row: usize, col: usize) -> Option<&TableCell> {
        match self.slot(row, col)? {
            TableSlot::Cell(cell) => Some(cell),
            TableSlot::Covered { .. } | TableSlot::Empty => None,
        }
    }

    /// Every real cell with its grid position and spans, in row-major order:
    /// what a grid layout needs.
    pub fn layout(&self) -> Vec<GridCell<'_>> {
        let mut cells = Vec::new();
        for row in 0..self.rows {
            for col in 0..self.cols {
                if let Some(cell) = self.cell(row, col) {
                    cells.push(GridCell {
                        row,
                        col,
                        row_span: cell.row_span,
                        col_span: cell.col_span,
                        cell,
                    });
                }
            }
        }
        cells
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "slot", rename_all = "snake_case")]
pub enum TableSlot {
    /// The top-left corner of a cell.
    Cell(TableCell),
    /// Covered by the spanning cell whose corner is at (`row`, `col`).
    Covered { row: usize, col: usize },
    /// No cell: the source row was shorter than the table.
    Empty,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TableCell {
    pub kind: CellKind,
    pub row_span: usize,
    pub col_span: usize,
    pub blocks: Vec<Block>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CellKind {
    Header,
    Data,
}

/// A cell placed on the table grid, from [`TableBlock::layout`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GridCell<'a> {
    pub row: usize,
    pub col: usize,
    pub row_span: usize,
    pub col_span: usize,
    pub cell: &'a TableCell,
}
