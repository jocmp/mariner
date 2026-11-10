//! `src/cleaners/resolve-split-title.js`.

use std::sync::LazyLock;

use regex::Regex;

use super::TITLE_SPLITTERS_RE;
use crate::utils::js;
use crate::utils::url::{host, parse_url};

static DOMAIN_ENDINGS_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r".com$|.net$|.org$|.co.uk$").unwrap());

/// Given a title with separators in it (colons, dashes, etc), resolve
/// whether any of the segments should be removed.
pub fn resolve_split_title(title: &str, url: &str) -> String {
    // Splits while preserving splitters, like:
    // ['The New New York', ' - ', 'The Washington Post']
    let split_title = split_keeping_separators(title);
    if split_title.len() == 1 {
        return title.to_string();
    }

    if let Some(new_title) = extract_breadcrumb_title(&split_title, title) {
        return new_title;
    }

    if let Some(new_title) = clean_domain_from_title(&split_title, url) {
        return new_title;
    }

    // Fuzzy ratio didn't find anything, so this title is probably legit.
    title.to_string()
}

fn split_keeping_separators(title: &str) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut last = 0;
    for separator in TITLE_SPLITTERS_RE.find_iter(title) {
        parts.push(&title[last..separator.start()]);
        parts.push(separator.as_str());
        last = separator.end();
    }
    parts.push(&title[last..]);
    parts
}

// This must be a very breadcrumbed title, like:
// The Best Gadgets on Earth : Bits : Blogs : NYTimes.com
// NYTimes - Blogs - Bits - The Best Gadgets on Earth
fn extract_breadcrumb_title(split_title: &[&str], text: &str) -> Option<String> {
    if split_title.len() < 6 {
        return None;
    }

    // Look to see if we can find a breadcrumb splitter that happens more
    // than once. If we can, we'll be able to better pull out the title.
    let mut term_counts: Vec<(&str, usize)> = Vec::new();
    for term in split_title {
        match term_counts
            .iter_mut()
            .find(|(existing, _)| existing == term)
        {
            Some((_, count)) => *count += 1,
            None => term_counts.push((term, 1)),
        }
    }
    let (max_term, term_count) =
        term_counts.iter().fold(
            ("", 0),
            |acc, &(term, count)| if acc.1 < count { (term, count) } else { acc },
        );

    // We found a splitter that was used more than once, so it is probably
    // the breadcrumber. Split our title on that instead. Note: max_term
    // should be <= 4 characters, so that " >> " will match, but nothing
    // longer than that.
    let first = if term_count >= 2 && js::len(max_term) <= 4 {
        text.split(max_term).next().unwrap_or_default()
    } else {
        split_title[0]
    };

    // Upstream compares the first segment against a one-element array
    // holding the last, so only the first segment can ever win.
    if js::len(first) > 10 {
        Some(first.to_string())
    } else {
        Some(text.to_string())
    }
}

// Search the ends of the title, looking for bits that fuzzy match the URL
// too closely. If one is found, discard it and return the rest.
fn clean_domain_from_title(split_title: &[&str], url: &str) -> Option<String> {
    let parsed = parse_url(url)?;

    // Strip out the big TLDs - it just makes the matching a bit more
    // accurate. Not the end of the world if it doesn't strip right.
    let naked_domain = DOMAIN_ENDINGS_RE
        .replace_all(&host(&parsed), "")
        .into_owned();

    let start_slug = split_title[0].to_lowercase().replacen(' ', "", 1);
    if levenshtein_ratio(&start_slug, &naked_domain) > 0.4 && js::len(&start_slug) > 5 {
        return Some(split_title[2..].concat());
    }

    let end_slug = split_title[split_title.len() - 1]
        .to_lowercase()
        .replacen(' ', "", 1);
    if levenshtein_ratio(&end_slug, &naked_domain) > 0.4 && js::len(&end_slug) >= 5 {
        return Some(split_title[..split_title.len() - 2].concat());
    }

    None
}

// wuzzy's levenshtein similarity over UTF-16 code units. Like wuzzy, an
// empty input returns the other's length rather than a ratio.
fn levenshtein_ratio(a: &str, b: &str) -> f64 {
    let a: Vec<u16> = a.encode_utf16().collect();
    let b: Vec<u16> = b.encode_utf16().collect();
    if a.is_empty() {
        return b.len() as f64;
    }
    if b.is_empty() {
        return a.len() as f64;
    }

    let mut v0: Vec<usize> = (0..=b.len()).collect();
    let mut v1 = vec![0; b.len() + 1];
    for (i, &ca) in a.iter().enumerate() {
        v1[0] = i + 1;
        for (j, &cb) in b.iter().enumerate() {
            let cost = usize::from(ca != cb);
            v1[j + 1] = (v1[j] + 1).min(v0[j + 1] + 1).min(v0[j] + cost);
        }
        v0.clone_from(&v1);
    }

    let max_len = a.len().max(b.len()) as f64;
    (max_len - v1[b.len()] as f64) / max_len
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn leaves_plain_titles_alone() {
        assert_eq!(
            resolve_split_title("The Best Gadgets", ""),
            "The Best Gadgets"
        );
    }

    #[test]
    fn extracts_breadcrumb_titles() {
        assert_eq!(
            resolve_split_title("The Best Gadgets on Earth : Bits : Blogs : NYTimes.com", ""),
            "The Best Gadgets on Earth "
        );
    }

    #[test]
    fn strips_domains_from_titles() {
        assert_eq!(
            resolve_split_title(
                "The New New York - The Washington Post",
                "https://www.washingtonpost.com/foo"
            ),
            "The New New York"
        );
    }
}
