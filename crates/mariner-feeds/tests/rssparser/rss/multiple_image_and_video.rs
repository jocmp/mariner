//! `feed-test-multiple-image-and-video.xml`.

use crate::{Case, first_entry};

#[test]
fn first_entry_matches() {
    Case {
        fixture: "feed-test-multiple-image-and-video.xml",
        title: Some("Motor.ru"),
        site_url: Some("https://motor.ru"),
        description: Some("Автомобильное издание Lenta.ru"),
        entry_title: Some("Kia сомневается в перспективах Stinger второго поколения"),
        entry_author: Some("Александр Пономарев"),
        entry_url: Some("https://motor.ru/news/kia-stinger-future-25-04-2020.htm"),
        categories: Some(&["Новости"]),
        summary_prefix: Some(
            "Планируемый рестайлинг Kia Stinger окажется весьма масштабным, но появление втор",
        ),
        content_prefix: Some(
            "<p>Планируемый рестайлинг Kia Stinger окажется весьма масштабным, но появление в",
        ),
        ..Case::DEFAULT
    }
    .check();
}

#[test]
fn video() {
    let entry = first_entry("feed-test-multiple-image-and-video.xml");
    let video = entry
        .enclosures
        .iter()
        .find(|enclosure| enclosure.mime_type.as_deref() == Some("video/mp4"))
        .map(|enclosure| enclosure.url.as_str());
    assert_eq!(
        video,
        Some("https://motor.media.eagleplatform.com/index/player?record_id=1410861")
    );
}
