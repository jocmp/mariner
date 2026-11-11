//! `feed-test-thumb.xml`.

use crate::Case;

#[test]
fn first_entry_matches() {
    Case {
        fixture: "feed-test-thumb.xml",
        title: Some("HDblog.it"),
        site_url: Some("https://www.hdblog.it"),
        description: Some("HDblog.it"),
        entry_id: Some("http://www.hdblog.it/?p=573063"),
        entry_title: Some(
            "Fairphone 5 è ufficiale: equo, modulare e aggiornato fino al 2031 | Anteprima IFA",
        ),
        entry_author: Some("HDblog.it"),
        entry_url: Some(
            "https://www.hdblog.it/smartphone/articoli/n573063/fairphone-5-ufficiale-caratteristiche-prezzo/",
        ),
        categories: Some(&["Android", "Fairphone"]),
        summary_prefix: Some(
            "Due fotocamere da 50MP sul retro, 5 anni di garanzia e 10 moduli sostituibili.",
        ),
        content_prefix: Some(
            "<a href=\"https://hd2.tudocdn.net/1113460?w=660\"><img src=\"https://hd2.tudocdn.ne",
        ),
        ..Case::DEFAULT
    }
    .check();
}
