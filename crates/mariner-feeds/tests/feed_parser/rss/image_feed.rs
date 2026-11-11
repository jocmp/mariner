//! `feed-test-image.xml`.

use crate::Case;

#[test]
fn first_entry_matches() {
    Case {
        fixture: "feed-test-image.xml",
        title: Some("Movie Reviews"),
        site_url: Some("https://movieweb.com/movie-reviews/"),
        description: Some("Movie Reviews at MovieWeb"),
        entry_id: Some("https://movieweb.com/the-sun-is-also-a-star-review/"),
        entry_title: Some(
            "The Sun Is Also a Star Review: Yara Shahidi & Charles Melton Elevate Teen Romance",
        ),
        entry_url: Some("https://movieweb.com/the-sun-is-also-a-star-review/"),
        summary_prefix: Some(
            "The Sun Is Also a Star is a diverse romance that bucks Hollywood's YA genre.",
        ),
        ..Case::DEFAULT
    }
    .check();
}
