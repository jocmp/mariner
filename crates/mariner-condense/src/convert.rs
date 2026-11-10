//! Walks the parsed DOM and emits blocks.

use dom_query::NodeRef;
use url::Url;

use crate::model::{
    ArticleDocument, AudioBlock, Block, CellKind, Ids, ImageBlock, ImageSource, ListItemBlock,
    QuoteBlock, Style, TextBlock, TextKind, Truncation, VideoBlock,
};
use crate::table::{GridBuilder, TableOutput};
use crate::text::TextBuilder;
use crate::{Condenser, media, style};

/// Elements whose content is never shown.
const DROPPED: [&str; 22] = [
    "area", "button", "canvas", "embed", "head", "input", "link", "map", "math", "meta",
    "noscript", "object", "option", "param", "rp", "rt", "script", "select", "style", "svg",
    "template", "textarea",
];

/// Containers that separate text but don't become blocks themselves.
const BOUNDARIES: [&str; 22] = [
    "address",
    "article",
    "aside",
    "center",
    "dd",
    "details",
    "dialog",
    "div",
    "dl",
    "dt",
    "fieldset",
    "figcaption",
    "footer",
    "form",
    "header",
    "hgroup",
    "hr",
    "main",
    "nav",
    "p",
    "section",
    "summary",
];

pub(crate) fn convert(root: &NodeRef, base: Option<&Url>, limits: &Condenser) -> ArticleDocument {
    let mut converter = Converter::new(base, limits);
    converter.walk(root);
    converter.finish()
}

struct Converter<'a> {
    base: Option<&'a Url>,
    limits: &'a Condenser,
    text: TextBuilder,
    /// `stack[0]` is the document; list items, quotes and captures sit above.
    stack: Vec<Container>,
    /// IDs seen but not yet attached to a block: the next block takes them.
    pending_ids: Vec<String>,
    /// Resolved `href`s of the links enclosing the current node.
    links: Vec<String>,
    /// One entry per enclosing `<figure>`: its caption until an image uses it.
    captions: Vec<Option<TextBlock>>,
    /// `<source srcset>` candidates from the enclosing `<picture>`.
    picture_sources: Vec<ImageSource>,
    pre_depth: usize,
    block_count: usize,
    char_count: usize,
    stopped: bool,
}

#[derive(Clone)]
enum ContainerKind {
    Root,
    ListItem {
        number: Option<i32>,
    },
    Quote {
        cite: Option<String>,
    },
    /// Collects blocks for a table cell or caption, then hands them back.
    Capture,
}

impl ContainerKind {
    /// The kind to reopen after splitting this container around media.
    fn continuation(&self) -> Self {
        match self {
            ContainerKind::ListItem { .. } => ContainerKind::ListItem { number: None },
            other => other.clone(),
        }
    }
}

struct Container {
    kind: ContainerKind,
    ids: Ids,
    blocks: Vec<Block>,
}

impl<'a> Converter<'a> {
    fn new(base: Option<&'a Url>, limits: &'a Condenser) -> Self {
        Self {
            base,
            limits,
            text: TextBuilder::new(),
            stack: vec![Container {
                kind: ContainerKind::Root,
                ids: Ids::new(),
                blocks: Vec::new(),
            }],
            pending_ids: Vec::new(),
            links: Vec::new(),
            captions: Vec::new(),
            picture_sources: Vec::new(),
            pre_depth: 0,
            block_count: 0,
            char_count: 0,
            stopped: false,
        }
    }

    fn finish(mut self) -> ArticleDocument {
        self.flush_text();
        while self.stack.len() > 1 {
            self.close_container();
        }
        let mut blocks = self.stack.pop().map(|root| root.blocks).unwrap_or_default();
        if let Some(last) = blocks.last_mut() {
            last.ids_mut().extend(self.pending_ids.drain(..));
        }
        let truncation = self.stopped.then_some(Truncation {
            blocks: self.block_count,
            chars: self.char_count,
        });
        ArticleDocument::assemble(blocks, truncation)
    }

    fn walk(&mut self, node: &NodeRef) {
        if self.stopped {
            return;
        }
        if node.is_text() {
            let text = node.text();
            if self.pre_depth > 0 {
                self.text.append_raw(&text);
            } else {
                self.text.append_collapsed(&text);
            }
            return;
        }
        if node.is_document() || node.is_fragment() {
            self.walk_children(node);
            return;
        }
        if !node.is_element() {
            return;
        }
        let name = element_name(node);
        if DROPPED.contains(&name.as_str()) || style::is_hidden(node) {
            return;
        }
        if let Some(source) =
            style::background_image(node).and_then(|url| media::background_source(&url, self.base))
        {
            self.emit_image(vec![source], None, Ids::new());
        }

        match name.as_str() {
            "br" => {
                self.note_id(node);
                self.text.line_break();
            }
            "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => self.walk_heading(node, &name),
            "pre" => self.walk_pre(node),
            "ul" | "menu" => self.walk_list(node, false),
            "ol" => self.walk_list(node, true),
            "li" => self.walk_list_item(node, None),
            "blockquote" => self.walk_quote(node),
            "figure" => self.walk_figure(node),
            // A figure captures its own caption; a stray one is plain text.
            "figcaption" if !self.captions.is_empty() => {}
            "picture" => self.walk_picture(node),
            "img" => self.walk_img(node),
            "video" => self.walk_video(node),
            "audio" => self.walk_audio(node),
            "iframe" => self.walk_iframe(node),
            "table" => self.walk_table(node),
            "source" | "track" => {}
            _ if BOUNDARIES.contains(&name.as_str()) => self.walk_boundary(node),
            _ => self.walk_inline(node, &name),
        }
    }

    fn walk_children(&mut self, node: &NodeRef) {
        for child in node.children_it(false) {
            if self.stopped {
                return;
            }
            self.walk(&child);
        }
    }

    fn walk_boundary(&mut self, node: &NodeRef) {
        self.flush_text();
        self.note_id(node);
        self.walk_children(node);
        self.flush_text();
    }

    fn walk_heading(&mut self, node: &NodeRef, name: &str) {
        let level = name[1..].parse().unwrap_or(1);
        self.flush_text();
        self.note_id(node);
        let handle = self.text.push_style(Style::Heading { level });
        self.walk_children(node);
        self.text.pop_style(handle);
        self.flush_text();
    }

    fn walk_pre(&mut self, node: &NodeRef) {
        self.flush_text();
        self.note_id(node);
        let kind = if is_code_block(node) {
            TextKind::Code
        } else {
            TextKind::Preformatted
        };
        let previous = self.text.set_kind(kind);
        self.pre_depth += 1;
        self.walk_children(node);
        self.flush_text();
        self.pre_depth -= 1;
        self.text.set_kind(previous);
    }

    fn walk_inline(&mut self, node: &NodeRef, name: &str) {
        self.note_id(node);
        let link = (name == "a")
            .then(|| node.attr("href"))
            .flatten()
            .and_then(|href| media::resolve(self.base, &href));
        let styles = self.inline_styles(node, name, link.as_deref());
        let handles: Vec<usize> = styles
            .into_iter()
            .map(|style| self.text.push_style(style))
            .collect();
        if let Some(link) = &link {
            self.links.push(link.clone());
        }

        self.walk_children(node);

        if link.is_some() {
            self.links.pop();
        }
        for handle in handles.into_iter().rev() {
            self.text.pop_style(handle);
        }
    }

    fn inline_styles(&self, node: &NodeRef, name: &str, link: Option<&str>) -> Vec<Style> {
        let mut styles = Vec::new();
        match name {
            "b" | "strong" => styles.push(Style::Bold),
            "i" | "em" | "cite" | "dfn" | "var" => styles.push(Style::Italic),
            "u" | "ins" => styles.push(Style::Underline),
            "s" | "strike" | "del" => styles.push(Style::Strikethrough),
            "sup" => styles.push(Style::Superscript),
            "sub" => styles.push(Style::Subscript),
            "tt" | "kbd" | "samp" => styles.push(Style::Monospace),
            "code" if self.pre_depth == 0 => styles.push(Style::Code),
            "font" => {
                if let Some(face) = node.attr("face").filter(|face| !face.trim().is_empty()) {
                    styles.push(Style::Font {
                        face: face.trim().to_string(),
                    });
                }
            }
            "span" => {
                if style::is_bold(node) {
                    styles.push(Style::Bold);
                }
                if style::is_italic(node) {
                    styles.push(Style::Italic);
                }
            }
            _ => {}
        }
        if let Some(url) = link {
            styles.push(Style::Link {
                url: url.to_string(),
            });
        }
        styles
    }

    fn walk_list(&mut self, node: &NodeRef, ordered: bool) {
        self.flush_text();
        self.note_id(node);
        let items = node
            .element_children()
            .iter()
            .filter(|child| element_name(child) == "li")
            .count();
        let reversed = ordered && node.has_attr("reversed");
        let mut number =
            parse_int(node, "start").unwrap_or(if reversed { items as i32 } else { 1 });

        for child in node.children_it(false) {
            if self.stopped {
                break;
            }
            if child.is_element() && element_name(&child) == "li" {
                let item_number = ordered.then(|| {
                    if let Some(value) = parse_int(&child, "value") {
                        number = value;
                    }
                    number
                });
                self.walk_list_item(&child, item_number);
                number = if reversed { number - 1 } else { number + 1 };
            } else {
                self.walk(&child);
            }
        }
        self.flush_text();
    }

    fn walk_list_item(&mut self, node: &NodeRef, number: Option<i32>) {
        self.open(ContainerKind::ListItem { number }, node);
        self.walk_children(node);
        self.close_container();
    }

    fn walk_quote(&mut self, node: &NodeRef) {
        let cite = node
            .attr("cite")
            .and_then(|cite| media::resolve(self.base, &cite));
        self.open(ContainerKind::Quote { cite }, node);
        self.walk_children(node);
        self.close_container();
    }

    fn walk_figure(&mut self, node: &NodeRef) {
        self.flush_text();
        self.note_id(node);
        let caption = figure_caption(node).and_then(|caption| self.capture_caption(&caption));
        self.captions.push(caption);
        self.walk_children(node);
        let unused = self.captions.pop().flatten();
        self.flush_text();

        // The figure's image may sit just before the figure element.
        if let Some(caption) = unused {
            match self.top().blocks.last_mut() {
                Some(Block::Image(image)) => image.caption = Some(caption),
                _ => self.place_text(caption),
            }
        }
    }

    fn walk_picture(&mut self, node: &NodeRef) {
        let outer = std::mem::replace(
            &mut self.picture_sources,
            media::picture_sources(node, self.base),
        );
        self.walk_children(node);
        // A picture with sources but no <img> is still an image.
        let unused = std::mem::replace(&mut self.picture_sources, outer);
        if !unused.is_empty() {
            self.flush_text();
            let ids = self.take_ids(node);
            self.emit_image(media::dedupe(unused), None, ids);
        }
    }

    fn walk_img(&mut self, node: &NodeRef) {
        let mut sources = media::img_sources(node, self.base);
        sources.append(&mut self.picture_sources);
        let sources = media::dedupe(sources);
        if sources.is_empty() {
            self.note_id(node);
            return;
        }
        self.flush_text();
        let ids = self.take_ids(node);
        let caption = match self.captions.last_mut() {
            Some(slot @ Some(_)) => slot.take(),
            _ => media::alt_caption(node),
        };
        self.emit_image(sources, caption, ids);
    }

    fn walk_video(&mut self, node: &NodeRef) {
        let sources = media::video_sources(node, self.base);
        if sources.is_empty() {
            self.note_id(node);
            return;
        }
        self.flush_text();
        let ids = self.take_ids(node);
        self.emit_media(Block::Video(VideoBlock { ids, sources }));
    }

    fn walk_audio(&mut self, node: &NodeRef) {
        let sources = media::audio_sources(node, self.base);
        if sources.is_empty() {
            self.note_id(node);
            return;
        }
        self.flush_text();
        let ids = self.take_ids(node);
        self.emit_media(Block::Audio(AudioBlock { ids, sources }));
    }

    /// Recognized video embeds become videos; other iframes are dropped.
    fn walk_iframe(&mut self, node: &NodeRef) {
        let src = node.attr("src").or_else(|| node.attr("data-src"));
        let Some(mut source) = src.and_then(|src| media::youtube(&src, self.base)) else {
            return;
        };
        source.width = media::dimension(node, "width");
        source.height = media::dimension(node, "height");
        self.flush_text();
        let ids = self.take_ids(node);
        self.emit_media(Block::Video(VideoBlock {
            ids,
            sources: vec![source],
        }));
    }

    fn walk_table(&mut self, node: &NodeRef) {
        self.flush_text();
        self.note_id(node);
        let rtl = is_rtl(node);

        let mut grid = GridBuilder::new();
        for child in node.element_children() {
            if element_name(&child) == "caption" {
                self.walk_boundary(&child);
            }
        }
        for row in table_rows(node) {
            if self.stopped {
                break;
            }
            grid.start_row();
            for cell in row.element_children() {
                let kind = match element_name(&cell).as_str() {
                    "th" => CellKind::Header,
                    "td" => CellKind::Data,
                    _ => continue,
                };
                let row_span = parse_int(&cell, "rowspan").map_or(1, |n| n.max(0) as usize);
                let col_span = parse_int(&cell, "colspan").map_or(1, |n| n.max(1) as usize);
                let blocks = self.capture(|this| {
                    this.note_id(&cell);
                    this.walk_children(&cell);
                });
                grid.add(kind, row_span, col_span, blocks);
            }
        }

        let ids = self.drain_pending_ids();
        match grid.finish(ids, rtl) {
            TableOutput::Table(table) => {
                self.block_count += 1;
                self.place(Block::Table(table));
            }
            TableOutput::Flat(blocks) => {
                for block in blocks {
                    self.place_captured(block);
                }
            }
        }
    }

    /// Walks a figcaption into a single text block.
    fn capture_caption(&mut self, node: &NodeRef) -> Option<TextBlock> {
        let blocks = self.capture(|this| {
            this.note_id(node);
            this.walk_children(node);
        });
        let mut caption: Option<TextBlock> = None;
        for block in blocks {
            match block {
                Block::Text(text) => match &mut caption {
                    None => caption = Some(text),
                    Some(caption) => append_line(caption, text),
                },
                other => self.place_captured(other),
            }
        }
        caption
    }

    /// Runs `f` with a fresh text builder and container, returning the
    /// blocks it produced instead of placing them.
    fn capture(&mut self, f: impl FnOnce(&mut Self)) -> Vec<Block> {
        self.flush_text();
        let saved_text = std::mem::replace(&mut self.text, TextBuilder::new());
        let saved_ids = std::mem::take(&mut self.pending_ids);
        self.stack.push(Container {
            kind: ContainerKind::Capture,
            ids: Ids::new(),
            blocks: Vec::new(),
        });
        let depth = self.stack.len();

        f(self);

        self.flush_text();
        while self.stack.len() > depth {
            self.close_container();
        }
        let mut blocks = self.stack.pop().map(|c| c.blocks).unwrap_or_default();
        if let Some(last) = blocks.last_mut() {
            last.ids_mut().extend(self.pending_ids.drain(..));
        }
        self.text = saved_text;
        let leftover = std::mem::replace(&mut self.pending_ids, saved_ids);
        self.pending_ids.extend(leftover);
        blocks
    }

    /// Places a block produced by a capture, keeping quotes text-only.
    fn place_captured(&mut self, block: Block) {
        match block {
            Block::Image(_) | Block::Video(_) | Block::Audio(_) => self.place_media(block),
            other => self.place(other),
        }
    }

    fn open(&mut self, kind: ContainerKind, node: &NodeRef) {
        self.flush_text();
        let ids = self.take_ids(node);
        self.stack.push(Container {
            kind,
            ids,
            blocks: Vec::new(),
        });
    }

    /// Closes the top container, placing its block in the parent. Empty
    /// containers vanish and pass their IDs on to the next block.
    fn close_container(&mut self) {
        self.flush_text();
        let Some(container) = self.stack.pop() else {
            return;
        };
        if container.blocks.is_empty() {
            self.pending_ids.extend(container.ids);
            return;
        }
        let block = match container.kind {
            ContainerKind::ListItem { number } => Block::ListItem(ListItemBlock {
                ids: container.ids,
                number,
                blocks: container.blocks,
            }),
            ContainerKind::Quote { cite } => Block::Quote(QuoteBlock {
                ids: container.ids,
                cite,
                blocks: container.blocks,
            }),
            ContainerKind::Root | ContainerKind::Capture => {
                // Never closed here; put it back.
                self.stack.push(container);
                return;
            }
        };
        self.block_count += 1;
        self.place(block);
    }

    fn flush_text(&mut self) {
        let Some(mut block) = self.text.take() else {
            return;
        };
        let chars = block.text.chars().count();
        if !self.admit(chars) {
            return;
        }
        block.ids = self.drain_pending_ids();
        self.place(Block::Text(block));
    }

    fn place_text(&mut self, block: TextBlock) {
        let chars = block.text.chars().count();
        if self.admit(chars) {
            self.place(Block::Text(block));
        }
    }

    fn emit_image(&mut self, sources: Vec<ImageSource>, caption: Option<TextBlock>, ids: Ids) {
        let link = self.links.last().cloned();
        self.emit_media(Block::Image(ImageBlock {
            ids,
            sources,
            caption,
            link,
        }));
    }

    fn emit_media(&mut self, block: Block) {
        self.flush_text();
        if self.admit(0) {
            self.place_media(block);
        }
    }

    /// Places media outside any enclosing quotes, splitting them around it.
    fn place_media(&mut self, block: Block) {
        let mut outermost_quote = None;
        for (index, container) in self.stack.iter().enumerate().rev() {
            match container.kind {
                ContainerKind::Quote { .. } => outermost_quote = Some(index),
                ContainerKind::Root | ContainerKind::Capture => break,
                ContainerKind::ListItem { .. } => {}
            }
        }
        let Some(quote) = outermost_quote else {
            self.place(block);
            return;
        };

        let mut reopen = Vec::new();
        while self.stack.len() > quote {
            if let Some(container) = self.stack.last() {
                reopen.push(container.kind.continuation());
            }
            self.close_container();
        }
        self.place(block);
        for kind in reopen.into_iter().rev() {
            self.stack.push(Container {
                kind,
                ids: Ids::new(),
                blocks: Vec::new(),
            });
        }
    }

    fn place(&mut self, block: Block) {
        self.top().blocks.push(block);
    }

    /// Counts a new block with `chars` characters against the caps. Once a
    /// cap would be exceeded, conversion stops and the block is dropped.
    fn admit(&mut self, chars: usize) -> bool {
        if self.stopped
            || self.block_count >= self.limits.max_blocks
            || self.char_count + chars > self.limits.max_chars
        {
            self.stopped = true;
            return false;
        }
        self.block_count += 1;
        self.char_count += chars;
        true
    }

    fn top(&mut self) -> &mut Container {
        self.stack
            .last_mut()
            .expect("the root container is never popped")
    }

    fn note_id(&mut self, node: &NodeRef) {
        if let Some(id) = node.id_attr() {
            let id = id.trim();
            if !id.is_empty() {
                self.pending_ids.push(id.to_string());
            }
        }
    }

    fn take_ids(&mut self, node: &NodeRef) -> Ids {
        self.note_id(node);
        self.drain_pending_ids()
    }

    fn drain_pending_ids(&mut self) -> Ids {
        self.pending_ids.drain(..).collect()
    }
}

fn element_name(node: &NodeRef) -> String {
    node.node_name()
        .map(|name| name.to_ascii_lowercase())
        .unwrap_or_default()
}

fn parse_int(node: &NodeRef, name: &str) -> Option<i32> {
    node.attr(name)?.trim().parse().ok()
}

/// `<pre>` whose only content is a single `<code>`.
fn is_code_block(pre: &NodeRef) -> bool {
    let mut code = false;
    for child in pre.children_it(false) {
        if child.is_element() {
            if code || element_name(&child) != "code" {
                return false;
            }
            code = true;
        } else if child.is_text() && !child.text().trim().is_empty() {
            return false;
        }
    }
    code
}

/// The nearest `dir` attribute on the table or its ancestors decides.
fn is_rtl(table: &NodeRef) -> bool {
    std::iter::once(*table)
        .chain(table.ancestors_it(None))
        .find_map(|node| node.attr("dir"))
        .is_some_and(|dir| dir.trim().eq_ignore_ascii_case("rtl"))
}

/// The figure's own figcaption, not one belonging to a nested figure.
fn figure_caption<'a>(figure: &NodeRef<'a>) -> Option<NodeRef<'a>> {
    figure.descendants_it().find(|node| {
        node.is_element()
            && element_name(node) == "figcaption"
            && node
                .ancestors_it(None)
                .find(|ancestor| element_name(ancestor) == "figure")
                .is_some_and(|nearest| nearest.id == figure.id)
    })
}

/// Rows in display order: `<thead>`, then bodies and bare rows, then `<tfoot>`.
fn table_rows<'a>(table: &NodeRef<'a>) -> Vec<NodeRef<'a>> {
    let mut head = Vec::new();
    let mut body = Vec::new();
    let mut foot = Vec::new();
    for child in table.element_children() {
        let rows = |section: &NodeRef<'a>| {
            section
                .element_children()
                .into_iter()
                .filter(|row| element_name(row) == "tr")
                .collect::<Vec<_>>()
        };
        match element_name(&child).as_str() {
            "thead" => head.extend(rows(&child)),
            "tbody" => body.extend(rows(&child)),
            "tfoot" => foot.extend(rows(&child)),
            "tr" => body.push(child),
            _ => {}
        }
    }
    head.into_iter().chain(body).chain(foot).collect()
}

/// Joins `next` onto `caption` as a new line, shifting its spans.
fn append_line(caption: &mut TextBlock, next: TextBlock) {
    caption.text.push('\n');
    let offset = caption.text.len();
    caption.text.push_str(&next.text);
    caption.ids.extend(next.ids);
    caption.spans.extend(next.spans.into_iter().map(|mut span| {
        span.start += offset;
        span.end += offset;
        span
    }));
}
