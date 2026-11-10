//! `src/extractors/generic/next-page-url/`: find the next page of a
//! multi-page article.

use std::sync::LazyLock;

use dom_query::{Document, NodeRef};
use regex::Regex;
use url::Url;

use crate::utils::difflib;
use crate::utils::dom::constants::{
    EXTRANEOUS_LINK_HINTS_RE, IS_WP_SELECTOR, NEGATIVE_SCORE_RE, PAGE_RE, POSITIVE_SCORE_RE,
};
use crate::utils::dom::select;
use crate::utils::js;
use crate::utils::text::{IS_DIGIT_RE, article_base_url, page_num_from_url, remove_anchor};
use crate::utils::url::parse_url;

static DIGIT_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\d").unwrap());

// Link text, class names, or ids that look like they could mean the next
// page: next, continue, >, >>, » but not >|, »| as those can mean last page.
static NEXT_LINK_TEXT_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)(next|weiter|continue|>([^|]|$)|»([^|]|$))").unwrap());

// Link text that looks like an end link: "first", "last", "end", etc.
static CAP_LINK_TEXT_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new("(?i)(first|last|end)").unwrap());

// Link text that looks like it means the previous page.
static PREV_LINK_TEXT_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new("(?i)(prev|earl|old|new|<|«)").unwrap());

struct ScoredLink {
    href: String,
    score: f64,
}

/// The URL of the next page, when a link scores confidently enough.
pub fn extract(
    doc: &Document,
    url: &str,
    parsed_url: &Url,
    previous_urls: &[String],
) -> Option<String> {
    let article_url = remove_anchor(url);
    let base_url = article_base_url(url, Some(parsed_url));

    let scored_links = score_links(doc, &article_url, &base_url, parsed_url, previous_urls);

    // Now that we've scored all possible pages, find the biggest one.
    let mut top_page: Option<&ScoredLink> = None;
    let mut top_score = -100.0;
    for link in &scored_links {
        if link.score > top_score {
            top_score = link.score;
            top_page = Some(link);
        }
    }

    // If the score is less than 50, we're not confident enough to use it.
    top_page
        .filter(|page| page.score >= 50.0)
        .map(|page| page.href.clone())
}

fn score_links(
    doc: &Document,
    article_url: &str,
    base_url: &str,
    parsed_url: &Url,
    previous_urls: &[String],
) -> Vec<ScoredLink> {
    // Upstream builds this from the unescaped base URL; an expression that
    // doesn't compile simply never matches here.
    let base_regex = Regex::new(&format!("(?i)^{base_url}")).ok();
    let is_wp = !select(doc, IS_WP_SELECTOR).is_empty();

    // Loop through all links, looking for hints that they may be next-page
    // links. Things like having "page" in their textContent, className or
    // id, or being a child of a node with a page-y className or id.
    let mut possible_pages: Vec<ScoredLink> = Vec::new();

    for link in select(doc, "a[href]").nodes() {
        let Some(raw_href) = link.attr("href").filter(|href| !href.is_empty()) else {
            continue;
        };

        // Remove any anchor data since we don't do a good job standardizing
        // URLs (it's hard); we do some checking with and without a trailing
        // slash.
        let href = remove_anchor(&raw_href);
        let link_text = link.text().to_string();

        if !should_score(
            &href,
            article_url,
            base_url,
            parsed_url,
            &link_text,
            previous_urls,
        ) {
            continue;
        }

        let link_data = format!(
            "{link_text} {} {}",
            link.attr("class").unwrap_or_default(),
            link.attr("id").unwrap_or_default()
        );
        let page_num = page_num_from_url(&href);

        let mut score = score_base_url(&href, base_regex.as_ref());
        score += score_next_link_text(&link_data);
        score += score_cap_links(&link_data);
        score += score_prev_link(&link_data);
        score += score_by_parents(link);
        score += score_extraneous_links(&href);
        score += score_page_in_link(page_num, is_wp);
        score += score_link_text(&link_text, page_num);
        score += score_similarity(score, article_url, &href);

        match possible_pages.iter_mut().find(|page| page.href == href) {
            Some(page) => page.score = score,
            None => possible_pages.push(ScoredLink { href, score }),
        }
    }

    possible_pages
}

fn should_score(
    href: &str,
    article_url: &str,
    base_url: &str,
    parsed_url: &Url,
    link_text: &str,
    previous_urls: &[String],
) -> bool {
    // Skip if we've already fetched this URL.
    if previous_urls.iter().any(|url| url == href) {
        return false;
    }

    // If we've already parsed this URL, or the URL matches the base URL, or
    // is empty, skip it.
    if href.is_empty() || href == article_url || href == base_url {
        return false;
    }

    // Domain mismatch.
    let Some(link_url) = parse_url(href) else {
        return false;
    };
    if link_url.host_str() != parsed_url.host_str() {
        return false;
    }

    // If href doesn't contain a digit after removing the base URL, it's
    // certainly not the next page.
    let fragment = href.replacen(base_url, "", 1);
    if !DIGIT_RE.is_match(&fragment) {
        return false;
    }

    // This link has extraneous content (like "comment") in its link text,
    // so we skip it.
    if EXTRANEOUS_LINK_HINTS_RE.is_match(link_text) {
        return false;
    }

    // Next page link text is never long, skip if it is too long.
    js::len(link_text) <= 25
}

// If the base URL isn't part of this URL, penalize this link. It could
// still be the link, but the odds are lower.
fn score_base_url(href: &str, base_regex: Option<&Regex>) -> f64 {
    if base_regex.is_some_and(|re| re.is_match(href)) {
        0.0
    } else {
        -25.0
    }
}

// Things like "next", ">>", etc.
fn score_next_link_text(link_data: &str) -> f64 {
    if NEXT_LINK_TEXT_RE.is_match(link_data) {
        50.0
    } else {
        0.0
    }
}

// Cap links are links like "last", etc. If a link is also marked "next",
// it's probably the last page: penalize it.
fn score_cap_links(link_data: &str) -> f64 {
    if CAP_LINK_TEXT_RE.is_match(link_data) && NEXT_LINK_TEXT_RE.is_match(link_data) {
        -65.0
    } else {
        0.0
    }
}

// If the link has something like "previous", it's definitely an old link.
fn score_prev_link(link_data: &str) -> f64 {
    if PREV_LINK_TEXT_RE.is_match(link_data) {
        -200.0
    } else {
        0.0
    }
}

// If a parent node contains paging-like classname or id, give a bonus.
// Additionally, if a parent node contains bad content (like 'sponsor'),
// give a penalty.
fn score_by_parents(link: &NodeRef<'_>) -> f64 {
    let mut parent = element_parent(link);
    let mut positive_match = false;
    let mut negative_match = false;
    let mut score = 0.0;

    // Upstream's `range(0, 4)` yields five values.
    for _ in 0..5 {
        let Some(current) = parent else {
            break;
        };

        let parent_data = format!(
            "{} {}",
            current.attr("class").unwrap_or_default(),
            current.attr("id").unwrap_or_default()
        );

        // If we have 'page' or 'paging' in our data, that's a good sign.
        if !positive_match && PAGE_RE.is_match(&parent_data) {
            positive_match = true;
            score += 25.0;
        }

        // If we have 'comment' or something in our data, and we don't have
        // something like 'content' as well, that's a bad sign.
        if !negative_match
            && NEGATIVE_SCORE_RE.is_match(&parent_data)
            && EXTRANEOUS_LINK_HINTS_RE.is_match(&parent_data)
            && !POSITIVE_SCORE_RE.is_match(&parent_data)
        {
            negative_match = true;
            score -= 25.0;
        }

        parent = element_parent(&current);
    }

    score
}

// If the URL itself contains extraneous values, give a penalty.
fn score_extraneous_links(href: &str) -> f64 {
    if EXTRANEOUS_LINK_HINTS_RE.is_match(href) {
        -25.0
    } else {
        0.0
    }
}

// Page in the link = bonus. Intentionally ignore WordPress because their
// ?p=123 link style gets caught by this even though it means separate
// documents entirely.
fn score_page_in_link(page_num: Option<i64>, is_wp: bool) -> f64 {
    if page_num.is_some_and(|n| n != 0) && !is_wp {
        50.0
    } else {
        0.0
    }
}

// If the link text can be parsed as a number, give it a minor bonus, with a
// slight bias towards lower numbered pages. This is so that pages that
// might not have 'next' in their text can still get scored, and sorted
// properly by score.
fn score_link_text(link_text: &str, page_num: Option<i64>) -> f64 {
    if !IS_DIGIT_RE.is_match(js::trim(link_text)) {
        return 0.0;
    }
    let Some(link_text_as_num) = js::parse_int(link_text) else {
        return 0.0;
    };

    // If it's the first page, we already got it on the first call. Give it
    // a negative score. Otherwise, up to page 10, give a small bonus.
    let mut score = if link_text_as_num < 2 {
        -30.0
    } else {
        (10 - link_text_as_num).max(0) as f64
    };

    // If it appears that the current page number is greater than this
    // link's page number, it's a very bad sign.
    if page_num.is_some_and(|n| n != 0 && n >= link_text_as_num) {
        score -= 50.0;
    }

    score
}

// Do this last and only if we have a real candidate, because it's
// potentially expensive computationally. Compare the link to this URL to
// get the % similarity of these URLs. On a sliding scale, subtract points
// from this link based on similarity.
fn score_similarity(score: f64, article_url: &str, href: &str) -> f64 {
    if score <= 0.0 {
        return 0.0;
    }

    let similarity = difflib::ratio(article_url, href);
    // Subtract .1 from diff_percent when calculating the modifier, which
    // means that if it's less than 10% different, we give a bonus instead.
    let diff_percent = 1.0 - similarity;
    let diff_modifier = -(250.0 * (diff_percent - 0.2));
    score + diff_modifier
}

fn element_parent<'a>(node: &NodeRef<'a>) -> Option<NodeRef<'a>> {
    node.parent().filter(NodeRef::is_element)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::dom::parse_document;

    #[test]
    fn finds_next_page_links() {
        let doc = parse_document(
            r#"<body><div class="pagination"><a href="http://example.com/foo/bar/2">Next</a><a href="http://example.com/foo/bar/1">Prev</a></div></body>"#,
        );
        let url = "http://example.com/foo/bar";
        let parsed = Url::parse(url).unwrap();
        assert_eq!(
            extract(&doc, url, &parsed, &[]),
            Some("http://example.com/foo/bar/2".to_string())
        );
    }

    #[test]
    fn ignores_links_without_hints() {
        let doc = parse_document(r#"<body><a href="http://example.com/other">About</a></body>"#);
        let url = "http://example.com/foo/bar";
        let parsed = Url::parse(url).unwrap();
        assert_eq!(extract(&doc, url, &parsed, &[]), None);
    }
}
