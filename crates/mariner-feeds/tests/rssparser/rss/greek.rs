//! `feed-test-greek.xml`.

use crate::Case;

#[test]
fn first_entry_matches() {
    Case {
        fixture: "feed-test-greek.xml",
        title: Some("Liberal - ΕΠΙΚΑΙΡΟΤΗΤΑ"),
        site_url: Some("https://www.liberal.gr/news"),
        description: Some(
            "Ενημέρωση με άποψη για την οικονομία, την πολιτική και τις διεθνείς σχέσεις.",
        ),
        entry_id: Some(
            "https://www.liberal.gr/news/portraita%2Dkarantinas%2Dgia%2Dempneusi%2Dapo%2Dti%2Dsullogi%2Dfeliou/298923",
        ),
        entry_title: Some("«Πορτραίτα καραντίνας» για έμπνευση από τη συλλογή Φέλιου"),
        entry_url: Some(
            "https://www.liberal.gr/news/portraita%2Dkarantinas%2Dgia%2Dempneusi%2Dapo%2Dti%2Dsullogi%2Dfeliou/298923",
        ),
        summary_prefix: Some(
            "Η συνθήκη του αναγκαστικού εγκλεισμού οδήγησε τα μεγάλα μουσεία διεθνώς να βρουν",
        ),
        ..Case::DEFAULT
    }
    .check();
}
