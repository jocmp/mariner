//! Converts article HTML into an ordered list of typed content blocks (text
//! with styled ranges, images, video, audio, lists, quotes and tables) so
//! Jetpack Compose, SwiftUI or a terminal UI can render an article without a
//! web view.
//!
//! ```
//! let document = mariner_condense::condense(
//!     "<h1>Hello</h1><p>A <b>bold</b> start.</p>",
//!     "https://example.com/post",
//! )
//! .unwrap();
//! assert_eq!(document.blocks.len(), 2);
//! ```
//!
//! Text styling is never nested nodes: each [`TextBlock`] is a string plus
//! [`Span`]s over it. Span bounds are UTF-8 byte offsets into
//! [`TextBlock::text`], half-open (`start` inclusive, `end` exclusive), so
//! `&block.text[span.start..span.end]` is the styled text.

mod convert;
mod media;
mod model;
mod style;
mod table;
mod text;

use std::io::Read;

use url::Url;

pub use model::{
    ArticleDocument, AudioBlock, AudioSource, Block, CellKind, GridCell, Ids, ImageBlock,
    ImageSource, ListItemBlock, QuoteBlock, Span, Style, TableBlock, TableCell, TableSlot,
    TextBlock, TextKind, Truncation, VideoBlock, VideoSource,
};

/// Default cap on the number of blocks, nested blocks included.
pub const DEFAULT_MAX_BLOCKS: usize = 2_000;

/// Default cap on the number of characters across all text blocks.
pub const DEFAULT_MAX_CHARS: usize = 100_000;

/// Converts `html` with the default caps. Relative URLs resolve against
/// `base_url`; pass an empty string when there is no base.
pub fn condense(html: &str, base_url: &str) -> Result<ArticleDocument, CondenseError> {
    Condenser::default().condense(html, base_url)
}

/// Like [`condense`], reading the HTML from `reader`. Invalid UTF-8 is
/// replaced rather than rejected.
pub fn condense_reader(
    reader: impl Read,
    base_url: &str,
) -> Result<ArticleDocument, CondenseError> {
    Condenser::default().condense_reader(reader, base_url)
}

/// Converts HTML with configurable output caps.
///
/// When a cap is hit, conversion stops and [`ArticleDocument::truncation`]
/// reports how much was kept. No "truncated" notice is added to the output.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Condenser {
    max_blocks: usize,
    max_chars: usize,
}

impl Default for Condenser {
    fn default() -> Self {
        Self {
            max_blocks: DEFAULT_MAX_BLOCKS,
            max_chars: DEFAULT_MAX_CHARS,
        }
    }
}

impl Condenser {
    pub fn new() -> Self {
        Self::default()
    }

    /// Caps the number of blocks, nested blocks included.
    pub fn max_blocks(mut self, max_blocks: usize) -> Self {
        self.max_blocks = max_blocks;
        self
    }

    /// Caps the number of characters across all text blocks.
    pub fn max_chars(mut self, max_chars: usize) -> Self {
        self.max_chars = max_chars;
        self
    }

    pub fn condense(&self, html: &str, base_url: &str) -> Result<ArticleDocument, CondenseError> {
        let base = parse_base(base_url)?;
        let document = dom_query::Document::from(html);
        let root = document.body().unwrap_or_else(|| document.root());
        Ok(convert::convert(&root, base.as_ref(), self))
    }

    pub fn condense_reader(
        &self,
        mut reader: impl Read,
        base_url: &str,
    ) -> Result<ArticleDocument, CondenseError> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes)?;
        self.condense(&String::from_utf8_lossy(&bytes), base_url)
    }
}

/// Why conversion failed. Malformed HTML is never an error: it's parsed the
/// way a browser would.
#[derive(Debug, thiserror::Error)]
pub enum CondenseError {
    #[error("invalid base URL `{url}`")]
    InvalidBaseUrl {
        url: String,
        #[source]
        source: url::ParseError,
    },
    #[error("failed to read HTML")]
    Read(#[from] std::io::Error),
}

fn parse_base(base_url: &str) -> Result<Option<Url>, CondenseError> {
    let base_url = base_url.trim();
    if base_url.is_empty() {
        return Ok(None);
    }
    Url::parse(base_url)
        .map(Some)
        .map_err(|source| CondenseError::InvalidBaseUrl {
            url: base_url.to_string(),
            source,
        })
}
