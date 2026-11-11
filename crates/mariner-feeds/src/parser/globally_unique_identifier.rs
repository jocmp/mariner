//! An RSS `<guid>`: its text, which is also the entry's URL unless
//! `isPermaLink="false"`.

use crate::rules::Class;
use crate::sax::Parsed;

pub(super) static GLOBALLY_UNIQUE_IDENTIFIER: Class = Class {
    rules: &[],
    keeps_text: true,
    attribute: Some("isPermaLink"),
};

pub(super) fn guid<'a>(guid: &Parsed<'a>) -> Option<&'a str> {
    guid.text()
}

pub(super) fn url<'a>(guid: &Parsed<'a>) -> Option<&'a str> {
    let perma_link = guid.attribute() != Some("false");
    guid.text().filter(|_| perma_link)
}
