//! timesofindia.indiatimes.com

use crate::extractors::{CustomExtractor, Field};

pub fn extractor() -> CustomExtractor {
    CustomExtractor::new("timesofindia.indiatimes.com")
        .title(Field::new().selector("h1"))
        .extend("reporter", Field::new().selector("div.byline"))
        .date_published(
            Field::new()
                .selector(".byline")
                .format("MMM D, YYYY, HH:mm z")
                .timezone("Asia/Kolkata"),
        )
        .lead_image_url(Field::new().attr(r#"meta[name="og:image"]"#, "value"))
        .content(
            Field::new()
                .selector("div.contentwrapper:has(section)")
                .default_cleaner(false)
                .clean([
                    "section",
                    "h1",
                    ".byline",
                    ".img_cptn",
                    ".icon_share_wrap",
                    r#"ul[itemtype="https://schema.org/BreadcrumbList"]"#,
                ]),
        )
}
