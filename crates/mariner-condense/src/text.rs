use crate::model::{Ids, Span, Style, TextBlock, TextKind};

/// Accumulates one text block's string and styled spans.
///
/// Styles still open when the block is taken carry over to the next block,
/// so inline styling that spans a block boundary styles both.
pub(crate) struct TextBuilder {
    text: String,
    spans: Vec<Span>,
    open: Vec<OpenStyle>,
    kind: TextKind,
    next_handle: usize,
}

struct OpenStyle {
    handle: usize,
    style: Style,
    start: usize,
}

impl TextBuilder {
    pub(crate) fn new() -> Self {
        Self {
            text: String::new(),
            spans: Vec::new(),
            open: Vec::new(),
            kind: TextKind::Prose,
            next_handle: 0,
        }
    }

    /// Sets the kind for blocks taken from now on, returning the previous one.
    pub(crate) fn set_kind(&mut self, kind: TextKind) -> TextKind {
        std::mem::replace(&mut self.kind, kind)
    }

    /// Starts `style` at the current position; close it with [`Self::pop_style`].
    pub(crate) fn push_style(&mut self, style: Style) -> usize {
        let handle = self.next_handle;
        self.next_handle += 1;
        self.open.push(OpenStyle {
            handle,
            style,
            start: self.text.len(),
        });
        handle
    }

    pub(crate) fn pop_style(&mut self, handle: usize) {
        let Some(index) = self.open.iter().position(|open| open.handle == handle) else {
            return;
        };
        let open = self.open.remove(index);
        if open.start < self.text.len() {
            self.spans.push(Span {
                style: open.style,
                start: open.start,
                end: self.text.len(),
            });
        }
    }

    /// Appends text with whitespace collapsed the way a browser renders it.
    pub(crate) fn append_collapsed(&mut self, text: &str) {
        for c in text.chars() {
            if is_html_whitespace(c) {
                if !self.text.is_empty() && !self.text.ends_with([' ', '\n']) {
                    self.text.push(' ');
                }
            } else {
                self.text.push(c);
            }
        }
    }

    /// Appends text as-is, for preformatted content.
    pub(crate) fn append_raw(&mut self, text: &str) {
        self.text.push_str(text);
    }

    /// A `<br>`: a line break within the current block.
    pub(crate) fn line_break(&mut self) {
        if self.text.is_empty() {
            return;
        }
        let trimmed = self.text.trim_end_matches(' ').len();
        self.text.truncate(trimmed);
        self.text.push('\n');
    }

    /// Finishes the current block, trimming trailing whitespace. Returns
    /// `None` when there's no text. Open styles stay open for the next block.
    pub(crate) fn take(&mut self) -> Option<TextBlock> {
        let trimmed = self.text.trim_end_matches(is_html_whitespace).len();
        self.text.truncate(trimmed);
        let len = self.text.len();

        let mut spans = std::mem::take(&mut self.spans);
        for open in &mut self.open {
            if open.start < len {
                spans.push(Span {
                    style: open.style.clone(),
                    start: open.start,
                    end: len,
                });
            }
            open.start = 0;
        }

        let text = std::mem::take(&mut self.text);
        if text.is_empty() {
            return None;
        }
        for span in &mut spans {
            span.end = span.end.min(len);
        }
        spans.retain(|span| span.start < span.end);
        spans.sort_by(|a, b| a.start.cmp(&b.start).then(b.end.cmp(&a.end)));

        Some(TextBlock {
            ids: Ids::new(),
            text,
            spans,
            kind: self.kind,
        })
    }
}

pub(crate) fn is_html_whitespace(c: char) -> bool {
    matches!(c, ' ' | '\t' | '\n' | '\r' | '\u{0C}')
}
