//! `feed-char-escape.xml`.

use crate::Case;

#[test]
fn first_entry_matches() {
    Case {
        fixture: "feed-char-escape.xml",
        title: Some("NYT > Health"),
        site_url: Some("https://www.nytimes.com/section/health"),
        entry_id: Some(
            "https://www.nytimes.com/2021/06/11/science/drought-las-vegas-grass-mars.html",
        ),
        entry_title: Some("Where the Grass is Greener, Except When It’s ‘Nonfunctional Turf’"),
        entry_author: Some("Alan Burdick"),
        entry_url: Some(
            "https://www.nytimes.com/2021/06/11/science/drought-las-vegas-grass-mars.html",
        ),
        summary_prefix: Some(
            "Plus, mammoths in Vegas, watermelon snow, Miami’s looming sea wall and more in t",
        ),
        ..Case::DEFAULT
    }
    .check();
}
