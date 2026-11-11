//! `feed-image-enclosure.xml`.

use crate::Case;

#[test]
fn first_entry_matches() {
    Case {
        fixture: "feed-image-enclosure.xml",
        title: Some("Centrum dopravního výzkumu, v. v. i. (RSS 2.0)"),
        site_url: Some("https://www.cdv.cz/"),
        description: Some(
            "Informace o novinkách na webu Centra dopravního výzkumu, pořádaných akcích v oblasti dopravy a odborných článcích z oblasti dopravy.",
        ),
        entry_title: Some("Podpora udržitelné aktivní mobility a nástroje práce s veřejností"),
        entry_url: Some(
            "https://www.cdv.cz/podpora-udrzitelne-aktivni-mobility-a-nastroje-prace-s-verejnosti/",
        ),
        categories: Some(&["event"]),
        ..Case::DEFAULT
    }
    .check();
}
