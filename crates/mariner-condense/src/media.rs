//! Image, video and audio sources.

use dom_query::NodeRef;
use url::Url;

use crate::model::{AudioSource, Ids, ImageSource, TextBlock, TextKind, VideoSource};

/// Attributes lazy-loading scripts use for the real image URL.
const LAZY_SRC_ATTRS: [&str; 5] = [
    "data-src",
    "data-lazy-src",
    "data-original",
    "data-hi-res-src",
    "data-url",
];

const SRCSET_ATTRS: [&str; 3] = ["srcset", "data-srcset", "data-lazy-srcset"];

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

/// Candidate sources for an `<img>`: `src` (unless it's a placeholder for a
/// lazy-load attribute), lazy-load attributes, then `srcset` entries.
pub(crate) fn img_sources(node: &NodeRef, base: Option<&Url>) -> Vec<ImageSource> {
    let width = dimension(node, "width");
    let height = dimension(node, "height");
    let lazy: Vec<String> = LAZY_SRC_ATTRS
        .iter()
        .filter_map(|name| node.attr(name).map(|value| value.to_string()))
        .collect();

    let mut urls = Vec::new();
    if let Some(src) = node.attr("src") {
        let placeholder = src.trim().starts_with("data:") && !lazy.is_empty();
        if !placeholder {
            urls.push(src.to_string());
        }
    }
    urls.extend(lazy);

    let mut sources: Vec<ImageSource> = urls
        .iter()
        .filter_map(|raw| resolve(base, raw))
        .map(|url| ImageSource {
            url,
            width,
            height,
            srcset_width: None,
            density: None,
        })
        .collect();
    for name in SRCSET_ATTRS {
        if let Some(srcset) = node.attr(name) {
            sources.extend(srcset_sources(&srcset, base));
        }
    }
    dedupe(sources)
}

/// Sources from a `<picture>`'s `<source srcset>` children.
pub(crate) fn picture_sources(picture: &NodeRef, base: Option<&Url>) -> Vec<ImageSource> {
    picture
        .element_children()
        .iter()
        .filter(|child| child.node_name().as_deref() == Some("source"))
        .filter_map(|source| source.attr("srcset"))
        .flat_map(|srcset| srcset_sources(&srcset, base))
        .collect()
}

pub(crate) fn background_source(url: &str, base: Option<&Url>) -> Option<ImageSource> {
    Some(ImageSource {
        url: resolve(base, url)?,
        width: None,
        height: None,
        srcset_width: None,
        density: None,
    })
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

/// `<video src>` and its `<source>` children.
pub(crate) fn video_sources(node: &NodeRef, base: Option<&Url>) -> Vec<VideoSource> {
    let thumbnail = node
        .attr("poster")
        .and_then(|poster| resolve(base, &poster));
    let width = dimension(node, "width");
    let height = dimension(node, "height");
    media_urls(node, base)
        .into_iter()
        .map(|(url, mime_type)| VideoSource {
            url,
            link: None,
            thumbnail: thumbnail.clone(),
            width,
            height,
            mime_type,
        })
        .collect()
}

/// `<audio src>` and its `<source>` children.
pub(crate) fn audio_sources(node: &NodeRef, base: Option<&Url>) -> Vec<AudioSource> {
    media_urls(node, base)
        .into_iter()
        .map(|(url, mime_type)| AudioSource { url, mime_type })
        .collect()
}

/// A YouTube embed or watch URL as a video with a thumbnail and watch link.
pub(crate) fn youtube(src: &str, base: Option<&Url>) -> Option<VideoSource> {
    let src = src.trim();
    let resolved = if src.starts_with("//") {
        format!("https:{src}")
    } else {
        resolve(base, src)?
    };
    let url = Url::parse(&resolved).ok()?;
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
    if id.len() < 6
        || !id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        return None;
    }

    Some(VideoSource {
        url: format!("https://www.youtube.com/embed/{id}"),
        link: Some(format!("https://www.youtube.com/watch?v={id}")),
        thumbnail: Some(format!("https://img.youtube.com/vi/{id}/hqdefault.jpg")),
        width: None,
        height: None,
        mime_type: None,
    })
}

pub(crate) fn dimension(node: &NodeRef, name: &str) -> Option<u32> {
    let value = node.attr(name)?;
    let digits: String = value
        .trim()
        .chars()
        .take_while(|c| c.is_ascii_digit())
        .collect();
    digits.parse().ok().filter(|&n| n > 0)
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

fn media_urls(node: &NodeRef, base: Option<&Url>) -> Vec<(String, Option<String>)> {
    let mut urls = Vec::new();
    if let Some(url) = node.attr("src").and_then(|src| resolve(base, &src)) {
        urls.push((url, mime_type(node)));
    }
    for source in node.element_children() {
        if source.node_name().as_deref() != Some("source") {
            continue;
        }
        if let Some(url) = source.attr("src").and_then(|src| resolve(base, &src)) {
            urls.push((url, mime_type(&source)));
        }
    }
    let mut seen = std::collections::HashSet::new();
    urls.retain(|(url, _)| seen.insert(url.clone()));
    urls
}

fn mime_type(node: &NodeRef) -> Option<String> {
    let value = node.attr("type")?;
    let value = value.trim();
    (!value.is_empty()).then(|| value.to_string())
}
