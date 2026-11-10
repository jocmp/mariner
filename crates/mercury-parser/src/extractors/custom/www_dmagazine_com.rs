//! www.dmagazine.com

use crate::extractors::{CustomExtractor, Field};

pub fn extractor() -> CustomExtractor {
    CustomExtractor::new("www.dmagazine.com")
        .title(Field::new().selector("h1.story__title"))
        .author(Field::new().selector(".story__info .story__info__item:first-child"))
        .date_published(
            Field::new()
                .selector(".story__info")
                .timezone("America/Chicago")
                .format("MMMM D, YYYY h:mm a"),
        )
        .dek(Field::new().selector(".story__subhead"))
        .lead_image_url(Field::new().attr("article figure a:first-child", "href"))
        .content(Field::new().selector(".story__content"))
}
