//! `feed-itunes-season.xml`.

use crate::{Case, first_entry};

#[test]
fn first_entry_matches() {
    Case {
        fixture: "feed-itunes-season.xml",
        title: Some("With Gourley And Rust"),
        site_url: Some("https://www.patreon.com/withgourleyandrust"),
        description: Some(
            "Nothing says slasher films and horror franchises more than an easy-listening, cozy-cast by two gentle movie lovers like <a href=\"mattgourley.com\" rel=\"noopener noreferrer\" target=\"_blank\">Matt Gourley</a> and <a href=\"twitter.com/paulrust\" rel=\"noopener noreferrer\" target=\"_blank\">Paul Rust</a>. Join them as they take you through the Jasons, the Michaels, the Freddies, and beyond. Get past seasons such as IN MYERS and KRUEGER WE TRUST, lots of bonus content, and regular episodes a week early at patreon.com/withgourleyandrust.<br /><hr><p style='color:grey; font-size:0.75em;'> Hosted on Acast. See <a style='color:grey;' target='_blank' rel='noopener noreferrer' href='https://acast.com/privacy'>acast.com/privacy</a> for more information.</p>",
        ),
        entry_id: Some("635861df5f0a000012334a6f"),
        entry_title: Some("RASING CAIN"),
        entry_url: Some("https://www.patreon.com/withgourleyandrust"),
        summary_prefix: Some(
            "<p>The De Palma Paradox.</p><br><p>With Gourley And Rust bonus content on <a hre",
        ),
        ..Case::DEFAULT
    }
    .check();
}

#[test]
fn audio() {
    let entry = first_entry("feed-itunes-season.xml");
    let url = "https://sphinx.acast.com/p/acast/s/with-gourley-and-rust/e/635861df5f0a000012334a6f/media.mp3";
    assert_eq!(
        entry.enclosure.map(|enclosure| enclosure.url).as_deref(),
        Some(url)
    );
    assert_eq!(entry.enclosures[0].url, url);
}
