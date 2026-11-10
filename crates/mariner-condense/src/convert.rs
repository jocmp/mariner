//! Walks the parsed DOM and emits blocks.

use dom_query::NodeRef;
use url::Url;

use crate::model::{
    ArticleDocument, AudioBlock, Block, CellKind, Ids, ImageBlock, ListItemBlock, QuoteBlock,
    Style, TextBlock, TextKind, Truncation, VideoBlock,
};
use crate::table::{GridBuilder, TableOutput};
use crate::text::TextBuilder;
use crate::{Condenser, media, style};

/// Elements whose content is never shown. A `<figcaption>` is read by its
/// figure; anywhere else it's dropped.
const DROPPED: [&str; 24] = [
    "area",
    "button",
    "canvas",
    "caption",
    "embed",
    "figcaption",
    "head",
    "input",
    "link",
    "map",
    "math",
    "meta",
    "noscript",
    "object",
    "option",
    "param",
    "rp",
    "rt",
    "script",
    "select",
    "style",
    "svg",
    "template",
    "textarea",
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
    /// Walking a figure's content for its caption: media and nested
    /// figures are skipped, and its figcaption is read.
    figure_content: bool,
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
            figure_content: false,
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
        if style::is_hidden(node) {
            return;
        }
        if self.figure_content {
            match name.as_str() {
                "figcaption" => return self.walk_inline(node, &name),
                "figure" | "img" | "picture" | "video" | "audio" | "iframe" => return,
                _ => {}
            }
        }
        if DROPPED.contains(&name.as_str()) {
            return;
        }

        match name.as_str() {
            "br" => {
                self.note_id(node);
                self.text.line_break();
            }
            "p" => self.walk_paragraph(node),
            "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => self.walk_heading(node, &name),
            "pre" => self.walk_pre(node),
            "ul" | "menu" => self.walk_list(node, false),
            "ol" => self.walk_list(node, true),
            "blockquote" => self.walk_quote(node),
            "figure" => self.walk_figure(node),
            "img" => self.walk_img(node),
            "video" => self.walk_video(node),
            "audio" => self.walk_audio(node),
            "iframe" => self.walk_iframe(node),
            "table" => self.walk_table(node),
            "source" | "track" => {}
            // Other containers (div, section, li outside a list, ...) flow
            // into the surrounding text.
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

    fn walk_paragraph(&mut self, node: &NodeRef) {
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
        let kind = if has_code_child(node) {
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
            .and_then(|href| media::resolve_link(self.base, &href));
        let styles = inline_styles(node, name, link.as_deref());
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

    /// Only `<li>` children count; anything else directly inside a list is
    /// dropped. Ordered items are numbered by position.
    fn walk_list(&mut self, node: &NodeRef, ordered: bool) {
        self.flush_text();
        self.note_id(node);
        let items = node
            .element_children()
            .into_iter()
            .filter(|child| element_name(child) == "li");
        for (index, item) in items.enumerate() {
            if self.stopped {
                break;
            }
            let number = ordered.then(|| i32::try_from(index + 1).unwrap_or(i32::MAX));
            self.open(ContainerKind::ListItem { number }, &item);
            self.walk_children(&item);
            self.close_container();
        }
    }

    fn walk_quote(&mut self, node: &NodeRef) {
        let cite = node
            .attr("cite")
            .map(|cite| cite.trim().to_string())
            .filter(|cite| !cite.is_empty());
        self.open(ContainerKind::Quote { cite }, node);
        self.walk_children(node);
        self.close_container();
    }

    /// A figure is a single image built from every image inside it. Its
    /// caption is the first text in the figure's other content, figcaption
    /// included; alt text isn't used. Without images, a YouTube embed inside
    /// becomes a video; otherwise the figure is dropped.
    fn walk_figure(&mut self, node: &NodeRef) {
        self.flush_text();
        let images: Vec<NodeRef> = node
            .descendants_it()
            .filter(|child| child.is_element() && element_name(child) == "img")
            .collect();
        let sources = media::dedupe(
            images
                .iter()
                .flat_map(|image| media::img_sources(image, self.base))
                .collect(),
        );

        if sources.is_empty() {
            let embed = node
                .descendants_it()
                .find(|child| child.is_element() && element_name(child) == "iframe");
            match embed {
                Some(iframe) => {
                    self.note_id(node);
                    self.walk_iframe(&iframe);
                }
                None => self.note_descendant_ids(node),
            }
            return;
        }

        let caption = self.capture_figure_caption(node);
        self.note_descendant_ids(node);
        let ids = self.drain_pending_ids();
        let link = self.links.last().cloned();
        self.emit_media(Block::Image(ImageBlock {
            ids,
            sources,
            caption,
            link,
        }));
    }

    fn walk_img(&mut self, node: &NodeRef) {
        let sources = media::img_sources(node, self.base);
        if sources.is_empty() {
            self.note_id(node);
            return;
        }
        self.flush_text();
        let ids = self.take_ids(node);
        let link = self.links.last().cloned();
        self.emit_media(Block::Image(ImageBlock {
            ids,
            sources,
            caption: media::alt_caption(node),
            link,
        }));
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
        let Some(source) = node
            .attr("src")
            .and_then(|src| media::youtube(&src, node, self.base))
        else {
            self.note_id(node);
            return;
        };
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
                    self.place(block);
                }
            }
        }
    }

    /// The first text block of a figure's content, links inside it ignored.
    fn capture_figure_caption(&mut self, figure: &NodeRef) -> Option<TextBlock> {
        let outer_links = std::mem::take(&mut self.links);
        let outer_figure = std::mem::replace(&mut self.figure_content, true);
        let blocks = self.capture(|this| this.walk_children(figure));
        self.figure_content = outer_figure;
        self.links = outer_links;
        blocks.into_iter().find_map(|block| match block {
            Block::Text(text) => Some(text),
            _ => None,
        })
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

    fn emit_media(&mut self, block: Block) {
        self.flush_text();
        if self.admit(0) {
            self.place(block);
        }
    }

    /// Places a block in the current container. Quotes hold text only:
    /// anything else, nested quotes included, goes outside the outermost
    /// enclosing quote, which is split around it.
    fn place(&mut self, block: Block) {
        let outermost_quote = match block {
            Block::Text(_) => None,
            _ => self.outermost_quote(),
        };
        let Some(quote) = outermost_quote else {
            self.top().blocks.push(block);
            return;
        };

        let mut reopen = Vec::new();
        while self.stack.len() > quote {
            if let Some(container) = self.stack.last() {
                reopen.push(container.kind.clone());
            }
            self.close_container();
        }
        self.top().blocks.push(block);
        for kind in reopen.into_iter().rev() {
            self.stack.push(Container {
                kind,
                ids: Ids::new(),
                blocks: Vec::new(),
            });
        }
    }

    /// When the current container is a quote, the index of the outermost
    /// quote in the run of quotes nested directly inside each other.
    fn outermost_quote(&self) -> Option<usize> {
        let mut outermost = None;
        for (index, container) in self.stack.iter().enumerate().rev() {
            match container.kind {
                ContainerKind::Quote { .. } => outermost = Some(index),
                _ => break,
            }
        }
        outermost
    }

    /// Counts a new block with `chars` characters against the caps. The
    /// block that goes over a cap is dropped, conversion stops, and the
    /// reported counts include it.
    fn admit(&mut self, chars: usize) -> bool {
        if self.stopped {
            return false;
        }
        self.block_count += 1;
        self.char_count += chars;
        if self.block_count > self.limits.max_blocks || self.char_count > self.limits.max_chars {
            self.stopped = true;
            return false;
        }
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

    /// Notes the IDs of `node` and every element inside it.
    fn note_descendant_ids(&mut self, node: &NodeRef) {
        self.note_id(node);
        for child in node.descendants_it() {
            if child.is_element() {
                self.note_id(&child);
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

fn inline_styles(node: &NodeRef, name: &str, link: Option<&str>) -> Vec<Style> {
    let mut styles = Vec::new();
    match name {
        "b" | "strong" => styles.push(Style::Bold),
        "i" | "em" | "cite" | "dfn" => styles.push(Style::Italic),
        "u" => styles.push(Style::Underline),
        "s" => styles.push(Style::Strikethrough),
        "sup" => styles.push(Style::Superscript),
        "sub" => styles.push(Style::Subscript),
        "tt" => styles.push(Style::Monospace),
        "code" => styles.push(Style::Code),
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

fn element_name(node: &NodeRef) -> String {
    node.node_name()
        .map(|name| name.to_ascii_lowercase())
        .unwrap_or_default()
}

fn parse_int(node: &NodeRef, name: &str) -> Option<i32> {
    node.attr(name)?.trim().parse().ok()
}

fn has_code_child(pre: &NodeRef) -> bool {
    pre.element_children()
        .iter()
        .any(|child| element_name(child) == "code")
}

/// The nearest `dir` attribute on the table or its ancestors decides.
fn is_rtl(table: &NodeRef) -> bool {
    std::iter::once(*table)
        .chain(table.ancestors_it(None))
        .find_map(|node| node.attr("dir"))
        .is_some_and(|dir| dir.trim().eq_ignore_ascii_case("rtl"))
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
