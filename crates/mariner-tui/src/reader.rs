//! An article as styled terminal lines: a header, then its content
//! condensed into blocks.

use mariner_condense::{Block, Style as SpanStyle, TextBlock, TextKind};
use mariner_core::ArticleContent;
use ratatui::style::{Color, Modifier, Style, Stylize};
use ratatui::text::{Line, Span};

/// The article's header and content, ready for a wrapping paragraph.
pub fn lines(content: &ArticleContent, feed_title: &str) -> Vec<Line<'static>> {
    let article = &content.article;
    let mut lines =
        vec![Line::from(article.title.clone().unwrap_or_else(|| "(Untitled)".into())).bold()];
    let mut byline = vec![feed_title.to_string()];
    if let Some(author) = &article.author {
        byline.push(author.clone());
    }
    byline.push(article.published_at.format("%b %-d, %Y").to_string());
    lines.push(Line::from(byline.join(" · ")).dim());
    if let Some(url) = &article.url {
        lines.push(Line::from(url.clone()).dim().underlined());
    }
    lines.push(Line::default());

    let html = content
        .content_html
        .as_deref()
        .or(article.summary.as_deref())
        .unwrap_or_default();
    let base_url = article.url.as_deref().unwrap_or_default();
    match mariner_condense::condense(html, base_url) {
        Ok(document) => blocks(&document.blocks, "", &mut lines),
        Err(_) => lines.push(Line::from("This article couldn't be read.").italic()),
    }
    lines
}

/// Appends `blocks`, each line starting with `prefix`, separated by blank
/// lines.
fn blocks(blocks: &[Block], prefix: &str, lines: &mut Vec<Line<'static>>) {
    for block in blocks {
        match block {
            Block::Text(text) => text_block(text, prefix, lines),
            Block::Image(image) => {
                let label = image
                    .caption
                    .as_ref()
                    .map(|caption| caption.text.clone())
                    .or_else(|| image.sources.first().map(|source| source.url.clone()))
                    .unwrap_or_default();
                lines.push(prefixed(
                    prefix,
                    vec![Span::from(format!("[image] {label}")).dim()],
                ));
            }
            Block::Video(video) => {
                let url = video
                    .sources
                    .first()
                    .map(|source| source.url.clone())
                    .unwrap_or_default();
                lines.push(prefixed(
                    prefix,
                    vec![Span::from(format!("[video] {url}")).dim()],
                ));
            }
            Block::Audio(audio) => {
                let url = audio
                    .sources
                    .first()
                    .map(|source| source.url.clone())
                    .unwrap_or_default();
                lines.push(prefixed(
                    prefix,
                    vec![Span::from(format!("[audio] {url}")).dim()],
                ));
            }
            Block::ListItem(item) => {
                let marker = match item.number {
                    Some(number) => format!("{number}. "),
                    None => "• ".to_string(),
                };
                let mut inner = Vec::new();
                self::blocks(&item.blocks, "", &mut inner);
                let indent = " ".repeat(marker.chars().count());
                let mut first = true;
                for line in inner.into_iter().filter(|line| !line.spans.is_empty()) {
                    let lead = if first {
                        marker.as_str()
                    } else {
                        indent.as_str()
                    };
                    first = false;
                    let mut spans = vec![Span::from(format!("{prefix}{lead}"))];
                    spans.extend(line.spans);
                    lines.push(Line::from(spans));
                }
                continue;
            }
            Block::Quote(quote) => {
                let mut inner = Vec::new();
                self::blocks(&quote.blocks, "", &mut inner);
                for line in inner {
                    let mut spans = vec![Span::from(format!("{prefix}│ ")).dim()];
                    spans.extend(line.spans.into_iter().map(|span| span.italic()));
                    lines.push(Line::from(spans));
                }
            }
            Block::Table(table) => {
                for row in 0..table.rows {
                    let cells: Vec<String> = (0..table.cols)
                        .filter_map(|col| table.cell(row, col))
                        .map(|cell| {
                            cell.blocks
                                .iter()
                                .filter_map(|block| match block {
                                    Block::Text(text) => Some(text.text.clone()),
                                    _ => None,
                                })
                                .collect::<Vec<_>>()
                                .join(" ")
                        })
                        .collect();
                    lines.push(prefixed(prefix, vec![Span::from(cells.join(" │ "))]));
                }
            }
        }
        lines.push(Line::default());
    }
}

/// A text block's lines, styled by its spans.
fn text_block(block: &TextBlock, prefix: &str, lines: &mut Vec<Line<'static>>) {
    let base = match block.kind {
        TextKind::Prose => Style::default(),
        TextKind::Preformatted | TextKind::Code => Style::default().fg(Color::Gray),
    };
    let mut line = Vec::new();
    for (text, style) in styled_runs(block, base) {
        let mut parts = text.split('\n').peekable();
        while let Some(part) = parts.next() {
            if !part.is_empty() {
                line.push(Span::styled(part.to_string(), style));
            }
            if parts.peek().is_some() {
                lines.push(prefixed(prefix, std::mem::take(&mut line)));
            }
        }
    }
    lines.push(prefixed(prefix, line));
}

/// The block's text cut at every span boundary, each run with the styles
/// of the spans covering it.
fn styled_runs(block: &TextBlock, base: Style) -> Vec<(String, Style)> {
    let text = &block.text;
    let mut boundaries: Vec<usize> = vec![0, text.len()];
    for span in &block.spans {
        boundaries.push(span.start.min(text.len()));
        boundaries.push(span.end.min(text.len()));
    }
    boundaries.sort_unstable();
    boundaries.dedup();
    boundaries
        .windows(2)
        .filter_map(|window| {
            let (start, end) = (window[0], window[1]);
            let run = text.get(start..end)?;
            let style = block
                .spans
                .iter()
                .filter(|span| span.start <= start && end <= span.end)
                .fold(base, |style, span| style.patch(span_style(&span.style)));
            Some((run.to_string(), style))
        })
        .collect()
}

fn span_style(style: &SpanStyle) -> Style {
    match style {
        SpanStyle::Heading { .. } => Style::default().add_modifier(Modifier::BOLD),
        SpanStyle::Bold => Style::default().add_modifier(Modifier::BOLD),
        SpanStyle::Italic => Style::default().add_modifier(Modifier::ITALIC),
        SpanStyle::Underline => Style::default().add_modifier(Modifier::UNDERLINED),
        SpanStyle::Strikethrough => Style::default().add_modifier(Modifier::CROSSED_OUT),
        SpanStyle::Monospace | SpanStyle::Code => Style::default().fg(Color::Gray),
        SpanStyle::Link { .. } => Style::default()
            .fg(Color::Blue)
            .add_modifier(Modifier::UNDERLINED),
        SpanStyle::Superscript | SpanStyle::Subscript | SpanStyle::Font { .. } => Style::default(),
    }
}

fn prefixed(prefix: &str, spans: Vec<Span<'static>>) -> Line<'static> {
    if prefix.is_empty() {
        return Line::from(spans);
    }
    let mut all = vec![Span::from(prefix.to_string())];
    all.extend(spans);
    Line::from(all)
}
