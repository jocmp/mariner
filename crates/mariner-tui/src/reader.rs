//! An article as terminal content: a header, then its content condensed
//! into blocks. Text is wrapped here rather than by ratatui, so each link's
//! place on screen is known and can be clicked. Images become previews once
//! fetched.

use std::sync::Arc;

use mariner_condense::{Block, Style as SpanStyle, TextBlock, TextKind};
use mariner_core::ArticleContent;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use unicode_width::UnicodeWidthStr;

use crate::halfblock;
use crate::images::{Image, Images};

/// Images are previewed at most this many columns wide.
const IMAGE_WIDTH: u16 = 80;

/// A run of text with one style, and the link it belongs to.
#[derive(Debug, Clone, PartialEq)]
pub struct Segment {
    pub text: String,
    pub style: Style,
    pub link: Option<Arc<str>>,
}

impl Segment {
    fn plain(text: impl Into<String>, style: Style) -> Self {
        Segment {
            text: text.into(),
            style,
            link: None,
        }
    }
}

/// A piece of an article: a line of text (wrapped when laid out), or an
/// image placed between lines.
#[derive(Debug, Clone)]
pub enum Part {
    Text(Vec<Segment>),
    Image {
        url: String,
        label: String,
        /// Where the image links to, if anywhere.
        link: Option<Arc<str>>,
    },
}

/// The article laid out for a width: its lines, and the links on each line
/// as column ranges.
pub struct Screen {
    pub lines: Vec<Line<'static>>,
    pub links: Vec<Vec<LinkRange>>,
}

/// Columns `start..end` of a line link to `url`.
#[derive(Debug, Clone, PartialEq)]
pub struct LinkRange {
    pub start: u16,
    pub end: u16,
    pub url: Arc<str>,
}

/// The article's header and content.
pub fn parts(content: &ArticleContent, feed_title: &str) -> Vec<Part> {
    let article = &content.article;
    let bold = Style::default().add_modifier(Modifier::BOLD);
    let dim = Style::default().add_modifier(Modifier::DIM);
    let title = article.title.clone().unwrap_or_else(|| "(Untitled)".into());
    let mut byline = vec![feed_title.to_string()];
    if let Some(author) = &article.author {
        byline.push(author.clone());
    }
    byline.push(article.published_at.format("%b %-d, %Y").to_string());
    let mut parts = vec![
        Part::Text(vec![Segment::plain(title, bold)]),
        Part::Text(vec![Segment::plain(byline.join(" · "), dim)]),
    ];
    if let Some(url) = &article.url {
        parts.push(Part::Text(vec![Segment {
            text: url.clone(),
            style: link_style().add_modifier(Modifier::DIM),
            link: Some(url.as_str().into()),
        }]));
    }
    parts.push(Part::Text(Vec::new()));

    let html = content
        .content_html
        .as_deref()
        .or(article.summary.as_deref())
        .unwrap_or_default();
    let base_url = article.url.as_deref().unwrap_or_default();
    match mariner_condense::condense(html, base_url) {
        Ok(document) => blocks(&document.blocks, "", &mut parts),
        Err(_) => parts.push(Part::Text(vec![Segment::plain(
            "This article couldn't be read.",
            Style::default().add_modifier(Modifier::ITALIC),
        )])),
    }
    parts
}

/// The URLs of the article's images, to fetch.
pub fn image_urls(parts: &[Part]) -> Vec<String> {
    parts
        .iter()
        .filter_map(|part| match part {
            Part::Image { url, .. } => Some(url.clone()),
            Part::Text(_) => None,
        })
        .collect()
}

/// The article's links in order, once each, with the text of their first
/// mention.
pub fn links(parts: &[Part]) -> Vec<(String, String)> {
    let mut links: Vec<(String, String)> = Vec::new();
    for part in parts {
        let mentions: Vec<(String, &str)> = match part {
            Part::Text(segments) => {
                let mut mentions: Vec<(String, &str)> = Vec::new();
                for segment in segments {
                    let Some(url) = segment.link.as_deref() else {
                        continue;
                    };
                    match mentions.last_mut() {
                        Some((text, last)) if *last == url => text.push_str(&segment.text),
                        _ => mentions.push((segment.text.clone(), url)),
                    }
                }
                mentions
            }
            Part::Image {
                label,
                link: Some(url),
                ..
            } => vec![(label.clone(), url)],
            Part::Image { .. } => Vec::new(),
        };
        for (text, url) in mentions {
            if !links.iter().any(|(_, known)| known == url) {
                let text = text.trim();
                let text = if text.is_empty() { url } else { text };
                links.push((text.to_string(), url.to_string()));
            }
        }
    }
    links
}

/// Lays the article out `width` columns wide: text wrapped at word
/// boundaries, each fetched image as a half-block preview, and the rest as
/// their label.
pub fn layout(parts: &[Part], width: u16, images: &Images, true_color: bool) -> Screen {
    let mut screen = Screen {
        lines: Vec::new(),
        links: Vec::new(),
    };
    for part in parts {
        match part {
            Part::Text(segments) => {
                for row in wrap(segments, width) {
                    push_row(&mut screen, row);
                }
            }
            Part::Image { url, label, link } => match images.get(url) {
                Some(Image::Ready(image)) => {
                    for line in halfblock::lines(&image, width.min(IMAGE_WIDTH), true_color) {
                        let line_width = line.width() as u16;
                        screen.links.push(
                            link.iter()
                                .map(|url| LinkRange {
                                    start: 0,
                                    end: line_width,
                                    url: Arc::clone(url),
                                })
                                .collect(),
                        );
                        screen.lines.push(line);
                    }
                    if !label.is_empty() {
                        let style = Style::default().add_modifier(Modifier::DIM | Modifier::ITALIC);
                        for row in wrap(&[Segment::plain(label.clone(), style)], width) {
                            push_row(&mut screen, row);
                        }
                    }
                }
                state => {
                    let loading = matches!(state, Some(Image::Loading));
                    let text = match (loading, label.is_empty()) {
                        (true, true) => "[image loading]".to_string(),
                        (true, false) => format!("[image loading] {label}"),
                        (false, true) => format!("[image] {url}"),
                        (false, false) => format!("[image] {label}"),
                    };
                    let segment = Segment {
                        text,
                        style: Style::default().add_modifier(Modifier::DIM),
                        link: link.clone().or_else(|| Some(url.as_str().into())),
                    };
                    for row in wrap(&[segment], width) {
                        push_row(&mut screen, row);
                    }
                }
            },
        }
    }
    screen
}

/// Adds a wrapped row as a line, noting where its links fall.
fn push_row(screen: &mut Screen, row: Vec<Segment>) {
    let mut links: Vec<LinkRange> = Vec::new();
    let mut column = 0u16;
    let mut spans = Vec::with_capacity(row.len());
    for segment in row {
        let width = segment.text.width() as u16;
        if let Some(url) = &segment.link {
            match links.last_mut() {
                Some(last) if last.end == column && last.url == *url => last.end += width,
                _ => links.push(LinkRange {
                    start: column,
                    end: column + width,
                    url: Arc::clone(url),
                }),
            }
        }
        column += width;
        spans.push(Span::styled(segment.text, segment.style));
    }
    screen.lines.push(Line::from(spans));
    screen.links.push(links);
}

/// Wraps a line of segments to `width` columns at word boundaries, breaking
/// words longer than a line. An empty line stays one empty row.
pub fn wrap(segments: &[Segment], width: u16) -> Vec<Vec<Segment>> {
    let width = usize::from(width.max(1));
    let mut rows: Vec<Vec<Segment>> = Vec::new();
    let mut row: Vec<Segment> = Vec::new();
    let mut row_width = 0;
    for (token, segment) in tokens(segments) {
        let token_width = token.width();
        let is_space = token.chars().all(char::is_whitespace);
        if is_space {
            if row_width + token_width <= width && !(row_width == 0 && !rows.is_empty()) {
                append(&mut row, token, segment);
                row_width += token_width;
            } else if row_width + token_width > width {
                rows.push(std::mem::take(&mut row));
                row_width = 0;
            }
            continue;
        }
        if row_width + token_width <= width {
            append(&mut row, token, segment);
            row_width += token_width;
            continue;
        }
        if token_width <= width {
            trim_end(&mut row);
            rows.push(std::mem::take(&mut row));
            append(&mut row, token, segment);
            row_width = token_width;
            continue;
        }
        if row_width > 0 {
            trim_end(&mut row);
            rows.push(std::mem::take(&mut row));
            row_width = 0;
        }
        for c in token.chars() {
            let char_width = c.to_string().width();
            if row_width + char_width > width && row_width > 0 {
                rows.push(std::mem::take(&mut row));
                row_width = 0;
            }
            append(&mut row, &c.to_string(), segment);
            row_width += char_width;
        }
    }
    trim_end(&mut row);
    rows.push(row);
    rows
}

/// Each segment's text cut into runs of whitespace and of everything else.
fn tokens(segments: &[Segment]) -> Vec<(&str, &Segment)> {
    let mut tokens = Vec::new();
    for segment in segments {
        let text = segment.text.as_str();
        let mut start = 0;
        let mut space = None;
        for (index, c) in text.char_indices() {
            let is_space = c.is_whitespace();
            if space.is_some_and(|space| space != is_space) {
                tokens.push((&text[start..index], segment));
                start = index;
            }
            space = Some(is_space);
        }
        if start < text.len() {
            tokens.push((&text[start..], segment));
        }
    }
    tokens
}

/// Appends text to a row, joining it to the last segment when they match.
fn append(row: &mut Vec<Segment>, text: &str, segment: &Segment) {
    match row.last_mut() {
        Some(last) if last.style == segment.style && last.link == segment.link => {
            last.text.push_str(text)
        }
        _ => row.push(Segment {
            text: text.to_string(),
            style: segment.style,
            link: segment.link.clone(),
        }),
    }
}

fn trim_end(row: &mut Vec<Segment>) {
    while let Some(last) = row.last_mut() {
        let trimmed = last.text.trim_end().len();
        if trimmed == 0 {
            row.pop();
        } else {
            last.text.truncate(trimmed);
            break;
        }
    }
}

/// Appends `blocks`, each line starting with `prefix`, separated by blank
/// lines.
fn blocks(blocks: &[Block], prefix: &str, parts: &mut Vec<Part>) {
    let dim = Style::default().add_modifier(Modifier::DIM);
    for block in blocks {
        match block {
            Block::Text(text) => text_block(text, prefix, parts),
            Block::Image(image) => match image.sources.first() {
                Some(source) => parts.push(Part::Image {
                    url: source.url.clone(),
                    label: image
                        .caption
                        .as_ref()
                        .map(|caption| caption.text.clone())
                        .unwrap_or_default(),
                    link: image.link.as_deref().map(Arc::from),
                }),
                None => continue,
            },
            Block::Video(video) => {
                let url = video
                    .sources
                    .first()
                    .map(|source| source.url.clone())
                    .unwrap_or_default();
                parts.push(prefixed(prefix, vec![media_link("[video]", &url, dim)]));
            }
            Block::Audio(audio) => {
                let url = audio
                    .sources
                    .first()
                    .map(|source| source.url.clone())
                    .unwrap_or_default();
                parts.push(prefixed(prefix, vec![media_link("[audio]", &url, dim)]));
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
                        Part::Text(segments) if segments.is_empty() => {}
                        Part::Text(segments) => {
                            let lead = if first {
                                marker.as_str()
                            } else {
                                indent.as_str()
                            };
                            first = false;
                            let mut line =
                                vec![Segment::plain(format!("{prefix}{lead}"), Style::default())];
                            line.extend(segments);
                            parts.push(Part::Text(line));
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
                        Part::Text(segments) => {
                            let mut line = vec![Segment::plain(format!("{prefix}│ "), dim)];
                            line.extend(segments.into_iter().map(|segment| Segment {
                                style: segment.style.add_modifier(Modifier::ITALIC),
                                ..segment
                            }));
                            parts.push(Part::Text(line));
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
                    parts.push(prefixed(
                        prefix,
                        vec![Segment::plain(cells.join(" │ "), Style::default())],
                    ));
                }
            }
        }
        parts.push(Part::Text(Vec::new()));
    }
}

/// `[video] <url>`, linked to the URL.
fn media_link(kind: &str, url: &str, style: Style) -> Segment {
    Segment {
        text: format!("{kind} {url}"),
        style,
        link: (!url.is_empty()).then(|| Arc::from(url)),
    }
}

/// A text block's lines, styled and linked by its spans.
fn text_block(block: &TextBlock, prefix: &str, parts: &mut Vec<Part>) {
    let base = match block.kind {
        TextKind::Prose => Style::default(),
        TextKind::Preformatted | TextKind::Code => Style::default().fg(Color::Gray),
    };
    let mut line = Vec::new();
    for run in styled_runs(block, base) {
        let mut pieces = run.text.split('\n').peekable();
        while let Some(piece) = pieces.next() {
            if !piece.is_empty() {
                line.push(Segment {
                    text: piece.to_string(),
                    ..run.clone()
                });
            }
            if pieces.peek().is_some() {
                parts.push(prefixed(prefix, std::mem::take(&mut line)));
            }
        }
    }
    parts.push(prefixed(prefix, line));
}

/// The block's text cut at every span boundary, each run with the styles
/// of the spans covering it, and the link of the innermost link span.
fn styled_runs(block: &TextBlock, base: Style) -> Vec<Segment> {
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
            let covering = block
                .spans
                .iter()
                .filter(|span| span.start <= start && end <= span.end);
            let style = covering
                .clone()
                .fold(base, |style, span| style.patch(span_style(&span.style)));
            let mut link = covering.filter_map(|span| match &span.style {
                SpanStyle::Link { url } => Some(Arc::from(url.as_str())),
                _ => None,
            });
            Some(Segment {
                text: run.to_string(),
                style,
                link: link.next_back(),
            })
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
        SpanStyle::Link { .. } => link_style(),
        SpanStyle::Superscript | SpanStyle::Subscript | SpanStyle::Font { .. } => Style::default(),
    }
}

fn link_style() -> Style {
    Style::default()
        .fg(Color::Blue)
        .add_modifier(Modifier::UNDERLINED)
}

fn prefixed(prefix: &str, segments: Vec<Segment>) -> Part {
    if prefix.is_empty() {
        return Part::Text(segments);
    }
    let mut all = vec![Segment::plain(prefix, Style::default())];
    all.extend(segments);
    Part::Text(all)
}

#[cfg(test)]
mod tests {
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

    fn images() -> Images {
        Images::new(HttpFetcher::new().unwrap())
    }

    #[test]
    fn wraps_at_word_boundaries_and_breaks_long_words() {
        let segments = [Segment::plain(
            "one two three fourteenletters",
            Style::default(),
        )];
        let rows: Vec<String> = wrap(&segments, 9)
            .into_iter()
            .map(|row| row.into_iter().map(|segment| segment.text).collect())
            .collect();
        assert_eq!(rows, ["one two", "three", "fourteenl", "etters"]);
        assert_eq!(wrap(&[], 10), [Vec::<Segment>::new()]);
    }

    #[test]
    fn knows_where_links_are_after_wrapping() {
        let parts = parts(
            &content(
                r#"<p>See <a href="/one">the first link</a> and <a href="https://two.example">two</a>.</p>"#,
            ),
            "Feed",
        );
        let screen = layout(&parts, 16, &images(), true);
        let row = screen
            .lines
            .iter()
            .position(|line| line.to_string().starts_with("See"))
            .unwrap();
        assert_eq!(
            text(&screen.lines[row..row + 2]),
            ["See the first", "link and two."]
        );
        let first = &screen.links[row];
        assert_eq!(first.len(), 1);
        assert_eq!((first[0].start, first[0].end), (4, 13));
        assert_eq!(&*first[0].url, "https://example.com/one");
        let second: Vec<_> = screen.links[row + 1]
            .iter()
            .map(|link| (link.start, link.end, link.url.to_string()))
            .collect();
        assert_eq!(
            second,
            [
                (0, 4, "https://example.com/one".to_string()),
                (9, 12, "https://two.example/".to_string())
            ]
        );
        assert_eq!(
            links(&parts),
            [
                (
                    "https://example.com/post".to_string(),
                    "https://example.com/post".to_string()
                ),
                (
                    "the first link".to_string(),
                    "https://example.com/one".to_string()
                ),
                ("two".to_string(), "https://two.example/".to_string()),
            ]
        );
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

        let images = images();
        let red = image::RgbaImage::from_pixel(40, 20, image::Rgba([255, 0, 0, 255]));
        images.insert("https://example.com/a.png", Image::Ready(Arc::new(red)));
        images.insert("https://example.com/b.png", Image::Loading);

        let screen = layout(&parts, 20, &images, true);
        let rendered = text(&screen.lines);
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
        assert_eq!(screen.lines.len(), screen.links.len());
    }
}
