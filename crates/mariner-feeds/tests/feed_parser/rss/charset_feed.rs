//! `feed-test-charset.xml`.

use crate::{Case, first_entry};

#[test]
fn first_entry_matches() {
    Case {
        fixture: "feed-test-charset.xml",
        title: Some("Lørdagsrådet"),
        site_url: Some("https://radio.nrk.no/podkast/loerdagsraadet"),
        description: Some(
            "Hver uke blar tre (mer eller mindre) kvalifiserte rådgivere i sitt livserfaringsarkiv for å hjelpe deg. Lurer du på om du skal gjøre det slutt, er naboen din en tyrann eller er du håpløst forelsket? Vi skal prøve å gi deg gode råd. <a href=\"https://radio.nrk.no/podkast/loerdagsraadet?utm_source=thirdparty&utm_medium=rss&utm_content=podcastseries%3Aloerdagsraadet\">Hør alle episodene først i appen NRK Radio</a>",
        ),
        entry_id: Some("l_e6338d4f-2777-43c4-b38d-4f2777e3c4de"),
        entry_title: Some("Høydepunkter fra 2022 - nr.11"),
        summary_prefix: Some(
            "Høydepunkter fra 2022 - nr.11. <a href=\"https://radio.nrk.no/podkast/loerdagsraa",
        ),
        ..Case::DEFAULT
    }
    .check();
}

#[test]
fn audio() {
    let entry = first_entry("feed-test-charset.xml");
    let url = "https://podkast.nrk.no/fil/loerdagsraadet/23ee45fc-d869-4110-bf97-34138fcfaba6_0_ID192MP3.mp3";
    assert_eq!(
        entry.enclosure.map(|enclosure| enclosure.url).as_deref(),
        Some(url)
    );
    assert_eq!(entry.enclosures[0].url, url);
}
