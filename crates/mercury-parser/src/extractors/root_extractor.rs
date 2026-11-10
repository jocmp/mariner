//! `src/extractors/root-extractor.js`: runs a custom extractor's fields,
//! falling back to the generic extractor for anything it doesn't cover.

use dom_query::{Document, Selection};
use serde_json::Value;
use url::Url;

use super::custom_extractor::{CustomExtractor, Field, Selector, Transform};
use super::generic;
use crate::cleaners::{
    clean_author, clean_content, clean_date_published, clean_dek, clean_image, clean_title,
};
use crate::utils::dom::{
    convert_node_to, find, html, make_links_absolute, new_div, outer_html, select, wrap,
};
use crate::utils::js;

/// Everything extraction needs to know about the page being parsed.
#[derive(Clone, Copy)]
pub struct Context<'a> {
    pub doc: &'a Document,
    pub url: &'a str,
    pub parsed_url: &'a Url,
    /// The page's HTML as given (or as fetched), before resource cleaning.
    pub html: &'a str,
    /// Every meta `name` in the document.
    pub meta_cache: &'a [String],
    /// Run the generic extractor for fields the custom extractor misses.
    pub fallback: bool,
    /// URLs of pages already collected, for multi-page articles.
    pub previous_urls: &'a [String],
}

/// The fields extracted from a single page.
#[derive(Clone, Debug, Default)]
pub struct Extracted {
    pub title: Option<String>,
    pub content: Option<String>,
    pub author: Option<String>,
    pub date_published: Option<String>,
    pub lead_image_url: Option<String>,
    pub dek: Option<String>,
    pub next_page_url: Option<String>,
    pub url: Option<String>,
    pub domain: Option<String>,
    pub excerpt: Option<String>,
    pub word_count: Option<usize>,
    pub direction: Option<String>,
    pub extended: Vec<(String, Value)>,
}

// The result types with a default cleaner.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Title,
    Author,
    DatePublished,
    LeadImageUrl,
    Dek,
    Other,
}

/// Extract every field from a page, using `extractor` where it has an
/// opinion and the generic extractor otherwise.
pub fn extract(extractor: Option<&CustomExtractor>, ctx: &Context<'_>) -> Extracted {
    let Some(extractor) = extractor else {
        return generic::extract(ctx);
    };

    let extended = select_extended_types(&extractor.extend, ctx);

    let title = extract_result(ctx, extractor.title.as_ref(), Kind::Title, None, || {
        Some(generic::title::extract(ctx.doc, ctx.url, ctx.meta_cache))
    });
    let date_published = extract_result(
        ctx,
        extractor.date_published.as_ref(),
        Kind::DatePublished,
        None,
        || generic::date_published::extract(ctx.doc, ctx.url, ctx.meta_cache),
    );
    let author = extract_result(ctx, extractor.author.as_ref(), Kind::Author, None, || {
        generic::author::extract(ctx.doc, ctx.meta_cache)
    });
    let next_page_url = extract_result(
        ctx,
        extractor.next_page_url.as_ref(),
        Kind::Other,
        None,
        || generic::next_page_url::extract(ctx.doc, ctx.url, ctx.parsed_url, ctx.previous_urls),
    );
    let content = extract_content(extractor, ctx, title.as_deref());
    let lead_image_url = extract_result(
        ctx,
        extractor.lead_image_url.as_ref(),
        Kind::LeadImageUrl,
        None,
        || generic::lead_image_url::extract(ctx.doc, content.as_deref(), ctx.meta_cache, ctx.html),
    );
    let excerpt = extract_result(ctx, extractor.excerpt.as_ref(), Kind::Other, None, || {
        Some(generic::excerpt::extract(
            ctx.doc,
            content.as_deref(),
            ctx.meta_cache,
        ))
    });
    let dek = extract_result(
        ctx,
        extractor.dek.as_ref(),
        Kind::Dek,
        excerpt.as_deref(),
        || None,
    );

    let (word_count, direction, url, domain) = if ctx.fallback {
        let (url, domain) = generic::url::extract(ctx.doc, ctx.url, ctx.meta_cache);
        (
            Some(generic::word_count::extract(
                content.as_deref().unwrap_or_default(),
            )),
            title
                .as_deref()
                .map(|title| generic::direction::direction(title).to_string()),
            Some(url),
            domain,
        )
    } else {
        (None, None, None, None)
    };

    Extracted {
        title,
        content,
        author,
        date_published,
        lead_image_url,
        dek,
        next_page_url,
        url,
        domain,
        excerpt,
        word_count,
        direction,
        extended,
    }
}

/// Extract only the content, as done for subsequent pages of a multi-page
/// article.
pub fn extract_content(
    extractor: &CustomExtractor,
    ctx: &Context<'_>,
    title: Option<&str>,
) -> Option<String> {
    let custom = extractor
        .content
        .as_ref()
        .and_then(|field| select_html(ctx, field, title))
        .and_then(|values| values.into_iter().next())
        .filter(|content| !content.is_empty());

    custom.or_else(|| {
        ctx.fallback
            .then(|| generic::content::extract(ctx.doc, ctx.html, title, ctx.url))
            .flatten()
    })
}

/// Run each extended field's selectors against the page. Values are
/// strings, arrays for `allow_multiple` fields, or null.
pub fn select_extended_types(
    extend: &[(String, Field)],
    ctx: &Context<'_>,
) -> Vec<(String, Value)> {
    let mut results: Vec<(String, Value)> = Vec::new();

    for (name, field) in extend {
        if results
            .iter()
            .any(|(existing, value)| existing == name && !is_falsy(value))
        {
            continue;
        }

        let value = match select_text(ctx, field, Kind::Other, None) {
            Some(values) if field.allow_multiple => Value::from(values),
            Some(values) => values.into_iter().next().map_or(Value::Null, Value::from),
            None => Value::Null,
        };

        match results.iter_mut().find(|(existing, _)| existing == name) {
            Some(entry) => entry.1 = value,
            None => results.push((name.clone(), value)),
        }
    }

    results
}

fn extract_result(
    ctx: &Context<'_>,
    field: Option<&Field>,
    kind: Kind,
    excerpt: Option<&str>,
    generic: impl FnOnce() -> Option<String>,
) -> Option<String> {
    let result = field
        .and_then(|field| select_text(ctx, field, kind, excerpt))
        .and_then(|values| values.into_iter().next())
        .filter(|value| !value.is_empty());

    // If nothing matches the selector, and fallback is enabled, run the
    // generic extraction.
    result.or_else(|| if ctx.fallback { generic() } else { None })
}

// Mirrors upstream's `select` for non-HTML types. Returns every value when
// the field allows multiple matches, otherwise the first.
fn select_text(
    ctx: &Context<'_>,
    field: &Field,
    kind: Kind,
    excerpt: Option<&str>,
) -> Option<Vec<String>> {
    // If a string is hardcoded for a type (e.g., Wikipedia contributors),
    // return the string.
    if let Some(literal) = &field.literal {
        return Some(vec![literal.clone()]);
    }

    let allow_multiple = kind == Kind::LeadImageUrl || field.allow_multiple;
    let matching = find_matching_selector(ctx.doc, &field.selectors, false, allow_multiple)?;

    let values: Vec<String> = match matching {
        Selector::Attr { css, attr, map } => {
            let matches = select(ctx.doc, css);
            transform_and_clean(&matches, ctx, field);
            matches
                .nodes()
                .iter()
                .filter_map(|node| node.attr(attr))
                .map(|value| {
                    let value = js::trim(&value).to_string();
                    map.as_ref()
                        .map_or_else(|| value.clone(), |map| map(&value))
                })
                .collect()
        }
        Selector::Css(css) => {
            let matches = select(ctx.doc, css);
            transform_and_clean(&matches, ctx, field);
            matches
                .nodes()
                .iter()
                .map(|node| js::trim(&node.text()).to_string())
                .collect()
        }
        Selector::Group(_) => return None,
    };

    let values = if field.allow_multiple {
        values
    } else {
        values.into_iter().take(1).collect()
    };

    // Allow custom extractors to skip the default cleaner for this type.
    if !field.default_cleaner {
        return Some(values);
    }

    let cleaned: Vec<String> = values
        .iter()
        .filter_map(|value| match kind {
            Kind::Title => Some(clean_title(value, ctx.url, ctx.doc)),
            Kind::Author => Some(clean_author(value)),
            Kind::DatePublished => {
                clean_date_published(value, field.timezone.as_deref(), field.format.as_deref())
            }
            Kind::LeadImageUrl => clean_image(value),
            Kind::Dek => clean_dek(value, excerpt),
            Kind::Other => Some(value.clone()),
        })
        .collect();

    (!cleaned.is_empty()).then_some(cleaned)
}

// Mirrors upstream's `select` for content: transform and clean the matched
// element(s), and return their HTML.
fn select_html(ctx: &Context<'_>, field: &Field, title: Option<&str>) -> Option<Vec<String>> {
    if let Some(literal) = &field.literal {
        return Some(vec![literal.clone()]);
    }

    let matching = find_matching_selector(ctx.doc, &field.selectors, true, field.allow_multiple)?;

    // An array selector is a multi-match selection: every selector must
    // match, and all of their elements are combined in one wrapper.
    let content: Vec<_> = match matching {
        Selector::Group(parts) => group_nodes(ctx.doc, parts)?,
        // Upstream treats any array as a group here, attribute pairs too.
        Selector::Attr { css, attr, .. } => group_nodes(ctx.doc, &[css.clone(), attr.clone()])?,
        Selector::Css(css) => select(ctx.doc, css).nodes().to_vec(),
    };

    // Wrap in a div so transformation can take place on the root element.
    let mut wrappers: Vec<_> = Vec::new();
    for node in &content {
        let wrapper = new_div(node);
        wrap(node, &wrapper);
        wrappers.push(wrapper);
    }
    let content = Selection::from(wrappers);

    transform_and_clean(&content, ctx, field);
    clean_content(
        &content,
        ctx.doc,
        true,
        title,
        ctx.url,
        field.default_cleaner,
    );

    if field.allow_multiple {
        return Some(content.children().nodes().iter().map(html).collect());
    }

    Some(vec![outer_html(&content)])
}

// Move every node matching any of `parts` into one detached wrapper, in
// document order.
fn group_nodes<'a>(doc: &'a Document, parts: &[String]) -> Option<Vec<dom_query::NodeRef<'a>>> {
    let matches = select(doc, &parts.join(","));
    let first = *matches.nodes().first()?;
    let wrapper = new_div(&first);
    for node in matches.nodes() {
        wrapper.append_child(node);
    }
    Some(vec![wrapper])
}

fn find_matching_selector<'f>(
    doc: &Document,
    selectors: &'f [Selector],
    extract_html: bool,
    allow_multiple: bool,
) -> Option<&'f Selector> {
    selectors.iter().find(|selector| match selector {
        Selector::Group(parts) => {
            extract_html && parts.iter().all(|part| !select(doc, part).is_empty())
        }
        Selector::Attr { css, attr, .. } if extract_html => {
            !select(doc, css).is_empty() && !select(doc, attr).is_empty()
        }
        Selector::Attr { css, attr, .. } => {
            let matches = select(doc, css);
            (allow_multiple || matches.length() == 1)
                && matches
                    .attr(attr)
                    .is_some_and(|value| !js::trim(&value).is_empty())
        }
        Selector::Css(css) => {
            let matches = select(doc, css);
            (allow_multiple || matches.length() == 1) && !js::trim(&matches.text()).is_empty()
        }
    })
}

fn transform_and_clean(content: &Selection<'_>, ctx: &Context<'_>, field: &Field) {
    make_links_absolute(content, ctx.doc, ctx.url);
    clean_by_selectors(content, field);
    transform_elements(content, field);
}

// Remove elements by an array of selectors.
fn clean_by_selectors(content: &Selection<'_>, field: &Field) {
    if field.clean.is_empty() {
        return;
    }
    for node in find(content, &field.clean.join(",")).nodes() {
        node.remove_from_parent();
    }
}

// Transform matching elements.
fn transform_elements(content: &Selection<'_>, field: &Field) {
    for (css, transform) in &field.transforms {
        let matches = find(content, css);
        for node in matches.nodes() {
            let tag = match transform {
                Transform::Rename(tag) => Some(tag.clone()),
                Transform::Fn(f) => f(&Selection::from(*node)),
            };
            if let Some(tag) = tag {
                convert_node_to(node, &tag);
            }
        }
    }
}

fn is_falsy(value: &Value) -> bool {
    match value {
        Value::Null => true,
        Value::String(s) => s.is_empty(),
        _ => false,
    }
}
