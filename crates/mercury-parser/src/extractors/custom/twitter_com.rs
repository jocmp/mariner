//! twitter.com

use crate::extractors::{CustomExtractor, Field};
use crate::utils::dom::outer_html;

pub fn extractor() -> CustomExtractor {
    CustomExtractor::new("twitter.com")
        .content(
            Field::new()
                .transform(".permalink[role=main]", |node| {
                    let tweets = node.select(".tweet");
                    node.replace_with_html(format!(
                        r#"<div id="TWEETS_GO_HERE">{}</div>"#,
                        outer_html(&tweets)
                    ));
                    None
                })
                .rename("s", "span")
                .selector(".permalink[role=main]")
                .default_cleaner(false)
                .clean([".stream-item-footer", "button", ".tweet-details-fixer"]),
        )
        .author(Field::new().selector(".tweet.permalink-tweet .username"))
        .date_published(
            Field::new().attr(".permalink-tweet ._timestamp[data-time-ms]", "data-time-ms"),
        )
}
