//! Image, video and audio sources.

use dom_query::NodeRef;
use url::Url;

use crate::model::{AudioSource, Ids, ImageSource, TextBlock, TextKind, VideoSource};
use crate::style;

/// Size of the YouTube thumbnail, used when an embed doesn't give one.
const YOUTUBE_WIDTH: u32 = 480;
const YOUTUBE_HEIGHT: u32 = 360;

/// Resolves `raw` against `base`, falling back to the raw value when it
/// can't be resolved. Empty values give `None`.
pub(crate) fn resolve(base: Option<&Url>, raw: &str) -> Option<String> {
    let raw = raw.trim();
    if raw.is_empty() {
        return None;
    }
    let resolved = match base {
        Some(base) => base.join(raw),
        None => Url::parse(raw),
    };
    Some(
        resolved
            .map(String::from)
            .unwrap_or_else(|_| raw.to_string()),
    )
}

/// Like [`resolve`], except an empty `href` links to the base URL itself.
pub(crate) fn resolve_link(base: Option<&Url>, href: &str) -> Option<String> {
    if href.trim().is_empty() {
        return base.map(|base| base.to_string());
    }
    resolve(base, href)
}

/// Candidate sources for an `<img>`, in order: `srcset` entries, `data-src`,
/// `src`, then an inline `background-image`.
pub(crate) fn img_sources(node: &NodeRef, base: Option<&Url>) -> Vec<ImageSource> {
    let mut sources = node
        .attr("srcset")
        .map(|srcset| srcset_sources(&srcset, base))
        .unwrap_or_default();

    let (width, height) = dimensions(node);
    let attributes = ["data-src", "src"]
        .into_iter()
        .filter_map(|name| node.attr(name))
        .filter_map(|raw| resolve(base, &raw));
    let background = style::background_image(node).and_then(|raw| resolve(base, &raw));
    for url in attributes.chain(background) {
        sources.push(ImageSource {
            url,
            width,
            height,
            srcset_width: None,
            density: None,
        });
    }
    dedupe(sources)
}

pub(crate) fn dedupe(sources: Vec<ImageSource>) -> Vec<ImageSource> {
    let mut seen = std::collections::HashSet::new();
    sources
        .into_iter()
        .filter(|source| seen.insert(source.url.clone()))
        .collect()
}

/// An `<img>`'s `alt` text, HTML stripped and whitespace collapsed.
pub(crate) fn alt_caption(node: &NodeRef) -> Option<TextBlock> {
    let alt = node.attr("alt")?;
    let stripped = dom_query::Document::fragment(alt.to_string()).text();
    let text = stripped.split_whitespace().collect::<Vec<_>>().join(" ");
    (!text.is_empty()).then_some(TextBlock {
        ids: Ids::new(),
        text,
        spans: Vec::new(),
        kind: TextKind::Prose,
    })
}

/// `<video src>` and its `<source>` children. Each source links to itself.
pub(crate) fn video_sources(node: &NodeRef, base: Option<&Url>) -> Vec<VideoSource> {
    let thumbnail = node
        .attr("poster")
        .and_then(|poster| resolve(base, &poster));
    let (width, height) = dimensions(node);
    let own = node
        .attr("src")
        .and_then(|src| resolve(base, &src))
        .map(|url| (url, mime_type(node)));
    own.into_iter()
        .chain(source_urls(node, base))
        .map(|(url, mime_type)| VideoSource {
            link: Some(url.clone()),
            url,
            thumbnail: thumbnail.clone(),
            width,
            height,
            mime_type,
        })
        .collect()
}

/// An `<audio>`'s `<source>` children.
pub(crate) fn audio_sources(node: &NodeRef, base: Option<&Url>) -> Vec<AudioSource> {
    source_urls(node, base)
        .into_iter()
        .map(|(url, mime_type)| AudioSource { url, mime_type })
        .collect()
}

/// A YouTube embed or watch URL as a video with a thumbnail and watch link.
/// The source keeps the iframe's URL; size comes from the iframe or
/// defaults to the thumbnail's.
pub(crate) fn youtube(src: &str, iframe: &NodeRef, base: Option<&Url>) -> Option<VideoSource> {
    let src = src.trim();
    let resolved = if src.starts_with("//") {
        format!("https:{src}")
    } else {
        resolve(base, src)?
    };
    let id = youtube_id(&Url::parse(&resolved).ok()?)?;

    Some(VideoSource {
        url: resolved,
        link: Some(format!("https://www.youtube.com/watch?v={id}")),
        thumbnail: Some(format!("https://img.youtube.com/vi/{id}/hqdefault.jpg")),
        width: Some(dimension(iframe, "width").unwrap_or(YOUTUBE_WIDTH)),
        height: Some(dimension(iframe, "height").unwrap_or(YOUTUBE_HEIGHT)),
        mime_type: None,
    })
}

/// `width` and `height` attributes, only when both are plain integers.
pub(crate) fn dimensions(node: &NodeRef) -> (Option<u32>, Option<u32>) {
    match (dimension(node, "width"), dimension(node, "height")) {
        (Some(width), Some(height)) => (Some(width), Some(height)),
        _ => (None, None),
    }
}

/// A single plain-integer `width` or `height` attribute.
pub(crate) fn dimension(node: &NodeRef, name: &str) -> Option<u32> {
    node.attr(name)?.trim().parse().ok()
}

fn youtube_id(url: &Url) -> Option<String> {
    let host = url.host_str()?;
    let host = host.strip_prefix("www.").unwrap_or(host);
    let host = host.strip_prefix("m.").unwrap_or(host);
    let segments: Vec<&str> = url.path_segments()?.filter(|s| !s.is_empty()).collect();

    let id = match host {
        "youtube.com" | "youtube-nocookie.com" => match segments.as_slice() {
            ["embed" | "v" | "shorts" | "live", id, ..] => Some(id.to_string()),
            ["watch", ..] => url
                .query_pairs()
                .find(|(key, _)| key == "v")
                .map(|(_, value)| value.into_owned()),
            _ => None,
        },
        "youtu.be" => segments.first().map(|id| id.to_string()),
        _ => None,
    }?;
    let valid = id.len() >= 6
        && id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_');
    valid.then_some(id)
}

fn srcset_sources(srcset: &str, base: Option<&Url>) -> Vec<ImageSource> {
    parse_srcset(srcset)
        .into_iter()
        .filter_map(|candidate| {
            Some(ImageSource {
                url: resolve(base, &candidate.url)?,
                width: None,
                height: None,
                srcset_width: candidate.width,
                density: candidate.density,
            })
        })
        .collect()
}

struct SrcsetCandidate {
    url: String,
    width: Option<u32>,
    density: Option<f32>,
}

/// Splits a `srcset` into candidates. A URL ends at whitespace; a trailing
/// comma on the URL ends the candidate with no descriptor.
fn parse_srcset(srcset: &str) -> Vec<SrcsetCandidate> {
    let mut candidates = Vec::new();
    let mut rest = srcset;
    loop {
        rest = rest.trim_start_matches(|c: char| c.is_ascii_whitespace() || c == ',');
        if rest.is_empty() {
            break;
        }
        let url_end = rest
            .find(|c: char| c.is_ascii_whitespace())
            .unwrap_or(rest.len());
        let mut url = &rest[..url_end];
        rest = &rest[url_end..];

        let mut descriptors = "";
        if url.ends_with(',') {
            url = url.trim_end_matches(',');
        } else {
            let descriptors_end = rest.find(',').unwrap_or(rest.len());
            descriptors = &rest[..descriptors_end];
            rest = &rest[descriptors_end..];
        }

        let mut candidate = SrcsetCandidate {
            url: url.to_string(),
            width: None,
            density: None,
        };
        for descriptor in descriptors.split_whitespace() {
            if let Some(width) = descriptor.strip_suffix('w') {
                candidate.width = width.parse().ok();
            } else if let Some(density) = descriptor.strip_suffix('x') {
                candidate.density = density.parse().ok();
            }
        }
        if !candidate.url.is_empty() {
            candidates.push(candidate);
        }
    }
    candidates
}

/// `<source src>` children, without duplicates.
fn source_urls(node: &NodeRef, base: Option<&Url>) -> Vec<(String, Option<String>)> {
    let mut seen = std::collections::HashSet::new();
    node.element_children()
        .into_iter()
        .filter(|child| child.node_name().as_deref() == Some("source"))
        .filter_map(|source| {
            let url = resolve(base, &source.attr("src")?)?;
            seen.insert(url.clone()).then(|| (url, mime_type(&source)))
        })
        .collect()
}

fn mime_type(node: &NodeRef) -> Option<String> {
    let value = node.attr("type")?;
    let value = value.trim();
    (!value.is_empty()).then(|| value.to_string())
}
