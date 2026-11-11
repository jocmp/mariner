//! An article as terminal content: a header, then its content condensed
//! into blocks. Images become previews once fetched.

use mariner_condense::{Block, Style as SpanStyle, TextBlock, TextKind};
use mariner_core::ArticleContent;
use ratatui::style::{Color, Modifier, Style, Stylize};
use ratatui::text::{Line, Span};

use crate::halfblock;
use crate::images::{Image, Images};

/// Images are previewed at most this many columns wide.
const IMAGE_WIDTH: u16 = 80;

/// A piece of an article: a line of text, or an image placed between lines.
#[derive(Debug, Clone)]
pub enum Part {
    Line(Line<'static>),
    Image { url: String, label: String },
}

/// The article's header and content.
pub fn parts(content: &ArticleContent, feed_title: &str) -> Vec<Part> {
    let article = &content.article;
    let mut header =
        vec![Line::from(article.title.clone().unwrap_or_else(|| "(Untitled)".into())).bold()];
    let mut byline = vec![feed_title.to_string()];
    if let Some(author) = &article.author {
        byline.push(author.clone());
    }
    byline.push(article.published_at.format("%b %-d, %Y").to_string());
    header.push(Line::from(byline.join(" · ")).dim());
    if let Some(url) = &article.url {
        header.push(Line::from(url.clone()).dim().underlined());
    }
    header.push(Line::default());
    let mut parts: Vec<Part> = header.into_iter().map(Part::Line).collect();

    let html = content
        .content_html
        .as_deref()
        .or(article.summary.as_deref())
        .unwrap_or_default();
    let base_url = article.url.as_deref().unwrap_or_default();
    match mariner_condense::condense(html, base_url) {
        Ok(document) => blocks(&document.blocks, "", &mut parts),
        Err(_) => parts.push(Part::Line(
            Line::from("This article couldn't be read.").italic(),
        )),
    }
    parts
}

/// The URLs of the article's images, to fetch.
pub fn image_urls(parts: &[Part]) -> Vec<String> {
    parts
        .iter()
        .filter_map(|part| match part {
            Part::Image { url, .. } => Some(url.clone()),
            Part::Line(_) => None,
        })
        .collect()
}

/// Lines for a pane `width` columns wide: each fetched image as a
/// half-block preview, others as their label.
pub fn render(parts: &[Part], width: u16, images: &Images, true_color: bool) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    for part in parts {
        match part {
            Part::Line(line) => lines.push(line.clone()),
            Part::Image { url, label } => match images.get(url) {
                Some(Image::Ready(image)) => {
                    lines.extend(halfblock::lines(&image, width.min(IMAGE_WIDTH), true_color));
                    if !label.is_empty() {
                        lines.push(Line::from(label.clone()).dim().italic());
                    }
                }
                Some(Image::Loading) => {
                    lines.push(Line::from(format!("[image loading] {label}")).dim())
                }
                Some(Image::Failed) | None => {
                    lines.push(Line::from(format!("[image] {label}")).dim())
                }
            },
        }
    }
    lines
}

/// Appends `blocks`, each line starting with `prefix`, separated by blank
/// lines.
fn blocks(blocks: &[Block], prefix: &str, parts: &mut Vec<Part>) {
    for block in blocks {
        match block {
            Block::Text(text) => {
                let mut lines = Vec::new();
                text_block(text, prefix, &mut lines);
                parts.extend(lines.into_iter().map(Part::Line));
            }
            Block::Image(image) => match image.sources.first() {
                Some(source) => parts.push(Part::Image {
                    url: source.url.clone(),
                    label: image
                        .caption
                        .as_ref()
                        .map(|caption| caption.text.clone())
                        .unwrap_or_default(),
                }),
                None => continue,
            },
            Block::Video(video) => {
                let url = video
                    .sources
                    .first()
                    .map(|source| source.url.clone())
                    .unwrap_or_default();
                parts.push(Part::Line(prefixed(
                    prefix,
                    vec![Span::from(format!("[video] {url}")).dim()],
                )));
            }
            Block::Audio(audio) => {
                let url = audio
                    .sources
                    .first()
                    .map(|source| source.url.clone())
                    .unwrap_or_default();
                parts.push(Part::Line(prefixed(
                    prefix,
                    vec![Span::from(format!("[audio] {url}")).dim()],
                )));
            }
            Block::ListItem(item) => {
                let marker = match item.number {
                    Some(number) => format!("{number}. "),
                    None => "• ".to_string(),
                };
                let indent = " ".repeat(marker.chars().count());
                let mut inner = Vec::new();
                self::blocks(&item.blocks, "", &mut inner);
                let mut first = true;
                for part in inner {
                    match part {
                        Part::Line(line) if line.spans.is_empty() => {}
                        Part::Line(line) => {
                            let lead = if first {
                                marker.as_str()
                            } else {
                                indent.as_str()
                            };
                            first = false;
                            let mut spans = vec![Span::from(format!("{prefix}{lead}"))];
                            spans.extend(line.spans);
                            parts.push(Part::Line(Line::from(spans)));
                        }
                        image => parts.push(image),
                    }
                }
                continue;
            }
            Block::Quote(quote) => {
                let mut inner = Vec::new();
                self::blocks(&quote.blocks, "", &mut inner);
                for part in inner {
                    match part {
                        Part::Line(line) => {
                            let mut spans = vec![Span::from(format!("{prefix}│ ")).dim()];
                            spans.extend(line.spans.into_iter().map(|span| span.italic()));
                            parts.push(Part::Line(Line::from(spans)));
                        }
                        image => parts.push(image),
                    }
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
                    parts.push(Part::Line(prefixed(
                        prefix,
                        vec![Span::from(cells.join(" │ "))],
                    )));
                }
            }
        }
        parts.push(Part::Line(Line::default()));
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

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use mariner_core::{Article, ArticleContent};
    use mariner_feeds::fetch::HttpFetcher;

    use super::*;

    fn content(html: &str) -> ArticleContent {
        ArticleContent {
            article: Article {
                id: "a".into(),
                feed_id: "f".into(),
                title: Some("Title".into()),
                author: None,
                url: Some("https://example.com/post".into()),
                summary: None,
                image_url: None,
                published_at: chrono::DateTime::from_timestamp(1_700_000_000, 0).unwrap(),
                read: false,
                starred: false,
            },
            content_html: Some(html.into()),
        }
    }

    fn text(lines: &[Line]) -> Vec<String> {
        lines.iter().map(|line| line.to_string()).collect()
    }

    #[test]
    fn previews_fetched_images_and_labels_the_rest() {
        let parts = parts(
            &content(
                r#"<p>Before</p><figure><img src="/a.png"><figcaption>A caption</figcaption></figure>
                   <p>Middle</p><img src="https://example.com/b.png"><p>After</p>"#,
            ),
            "Feed",
        );
        assert_eq!(
            image_urls(&parts),
            ["https://example.com/a.png", "https://example.com/b.png"]
        );

        let images = Images::new(HttpFetcher::new().unwrap());
        let red = image::RgbaImage::from_pixel(40, 20, image::Rgba([255, 0, 0, 255]));
        images.insert("https://example.com/a.png", Image::Ready(Arc::new(red)));
        images.insert("https://example.com/b.png", Image::Loading);

        let lines = render(&parts, 20, &images, true);
        let rendered = text(&lines);
        let preview: Vec<_> = rendered
            .iter()
            .filter(|line| line.starts_with('▀'))
            .collect();
        assert_eq!(
            preview.len(),
            5,
            "40x20 pixels at 20 columns: 10 pixel rows, 5 lines"
        );
        assert_eq!(preview[0].chars().count(), 20);
        assert!(rendered.contains(&"A caption".to_string()));
        assert!(
            rendered
                .iter()
                .any(|line| line.starts_with("[image loading]"))
        );
        let order: Vec<_> = rendered
            .iter()
            .filter(|line| ["Before", "Middle", "After"].contains(&line.as_str()))
            .collect();
        assert_eq!(order, ["Before", "Middle", "After"]);
    }
}
