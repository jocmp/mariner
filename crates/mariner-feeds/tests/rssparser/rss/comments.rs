//! `feed-comment.xml`.

use crate::Case;

#[test]
fn first_entry_matches() {
    Case {
        fixture: "feed-comment.xml",
        title: Some("Hacker News"),
        site_url: Some("https://news.ycombinator.com/"),
        description: Some("Links for the intellectually curious, ranked by readers."),
        entry_url: Some("https://gonzoknows.com/posts/GPTZero-Case-Study/"),
        summary_prefix: Some(
            "<a href=\"https://news.ycombinator.com/item?id=34858307\">Comments</a>",
        ),
        ..Case::DEFAULT
    }
    .check();
}
