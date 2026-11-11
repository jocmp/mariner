//! An `<itunes:owner>`.

use crate::rules::{Class, Field, text};

pub(super) static ITUNES_RSS_OWNER: Class = Class::new(&[
    text("itunes:name", Field::Other),
    text("itunes:email", Field::Other),
]);
