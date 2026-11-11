//! `feed-itunes.xml`.

use crate::Case;

#[test]
fn first_entry_matches() {
    Case {
        fixture: "feed-itunes.xml",
        title: Some("The Joe Rogan Experience"),
        site_url: Some("https://www.joerogan.com"),
        description: Some("Conduit to the Gaian Mind"),
        entry_id: Some("00a5d989b6b2cd8267cf8239f3b5585c"),
        entry_title: Some("#1109 - Matthew Walker"),
        entry_url: Some("http://traffic.libsyn.com/joeroganexp/p1109.mp3"),
        categories: Some(&[]),
        summary_prefix: Some(
            "Matthew Walker is Professor of Neuroscience and Psychology at the University of ",
        ),
        content_prefix: Some(
            "Matthew Walker is Professor of Neuroscience and Psychology at the University of ",
        ),
        ..Case::DEFAULT
    }
    .check();
}
