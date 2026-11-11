//! `atom-replies-link-example.xml`.

use crate::Case;

#[test]
fn first_entry_matches() {
    Case {
        fixture: "atom-replies-link-example.xml",
        title: Some("9to5Linux"),
        site_url: Some("https://9to5linux.com/"),
        description: Some("Linux news, reviews, tutorials, and more"),
        entry_id: Some("https://9to5linux.com/?p=23839"),
        entry_title: Some(
            "Fwupd 1.9.24 Firmware Updater Adds Support for Capsule-on-Disk for Dell Systems",
        ),
        entry_author: Some("Marius Nestor"),
        entry_url: Some(
            "https://9to5linux.com/fwupd-1-9-24-adds-support-for-capsule-on-disk-for-dell-systems",
        ),
        categories: Some(&[
            "App",
            "News",
            "firmware update",
            "firmware upgrade",
            "fwupd",
            "Fwupd 1.9.24",
            "Linux firmware",
        ]),
        summary_prefix: Some(
            "<p>fwupd 1.9.24 Linux firmware updater is now available for download with suppor",
        ),
        ..Case::DEFAULT
    }
    .check();
}
