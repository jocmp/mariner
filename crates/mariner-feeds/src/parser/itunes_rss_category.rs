//! An `<itunes:category>`, which may nest more categories.

use crate::rules::{Class, Field, collection};

pub(super) static ITUNES_RSS_CATEGORY: Class = Class::new(&[collection(
    "itunes:category",
    Field::Nested,
    &ITUNES_RSS_CATEGORY,
)]);
