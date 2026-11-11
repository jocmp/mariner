//! `feed-test-malformed.xml`: a malformed feed still has items.

use crate::first_entry;

#[test]
fn still_has_items() {
    first_entry("feed-test-malformed.xml");
}
