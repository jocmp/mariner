//! Tests ported from the fixtures' original suite: one module per test
//! class, grouped as that suite groups them. Each checks the first entry
//! against the values its fixture was written to test, kept only where
//! they agree with the reference output; text is compared trimmed, and
//! `summary` and `content` by their first line.

use mariner_feeds::{Entry, Feed, parse_with_charset};

mod atom;
mod json;
mod malformed_feed;
mod rdf;
mod rss;

pub(crate) struct Case {
    fixture: &'static str,
    charset: Option<&'static str>,
    title: Option<&'static str>,
    site_url: Option<&'static str>,
    description: Option<&'static str>,
    entry_id: Option<&'static str>,
    entry_title: Option<&'static str>,
    entry_author: Option<&'static str>,
    entry_url: Option<&'static str>,
    categories: Option<&'static [&'static str]>,
    summary_prefix: Option<&'static str>,
    content_prefix: Option<&'static str>,
}

impl Case {
    pub const DEFAULT: Case = Case {
        fixture: "",
        charset: None,
        title: None,
        site_url: None,
        description: None,
        entry_id: None,
        entry_title: None,
        entry_author: None,
        entry_url: None,
        categories: None,
        summary_prefix: None,
        content_prefix: None,
    };

    /// Parses the fixture and fails with every difference.
    pub fn check(&self) {
        let feed = parse_with_charset(&read(self.fixture), self.charset).unwrap();
        let differences = self.differences(&feed);
        assert!(differences.is_empty(), "\n{}", differences.join("\n"));
    }

    fn differences(&self, feed: &Feed) -> Vec<String> {
        let entry = feed.entries.first().cloned().unwrap_or_default();
        let mut differences = Vec::new();
        let mut equal = |name: &str, expected: Option<&str>, actual: &Option<String>| {
            if let Some(expected) = expected
                && actual.as_deref().map(str::trim) != Some(expected)
            {
                differences.push(format!("{name}: expected {expected:?}, got {actual:?}"));
            }
        };
        equal("title", self.title, &feed.title);
        equal("site_url", self.site_url, &feed.site_url);
        equal("description", self.description, &feed.description);
        equal("entry id", self.entry_id, &entry.id);
        equal("entry title", self.entry_title, &entry.title);
        equal("entry author", self.entry_author, &entry.author);
        equal("entry url", self.entry_url, &entry.url);
        let mut starts = |name: &str, expected: Option<&str>, actual: &Option<String>| {
            if let Some(expected) = expected
                && !actual
                    .as_deref()
                    .is_some_and(|actual| actual.trim().starts_with(expected))
            {
                differences.push(format!("{name}: expected {expected:?}..., got {actual:?}"));
            }
        };
        starts("entry summary", self.summary_prefix, &entry.summary);
        starts("entry content", self.content_prefix, &entry.content);
        if let Some(expected) = self.categories
            && entry.categories != expected
        {
            differences.push(format!(
                "entry categories: expected {expected:?}, got {:?}",
                entry.categories
            ));
        }
        differences
    }
}

/// The fixture's first entry.
pub(crate) fn first_entry(fixture: &str) -> Entry {
    mariner_feeds::parse(&read(fixture))
        .unwrap()
        .entries
        .remove(0)
}

fn read(fixture: &str) -> Vec<u8> {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/rssparser");
    std::fs::read(format!("{path}/{fixture}")).unwrap()
}
