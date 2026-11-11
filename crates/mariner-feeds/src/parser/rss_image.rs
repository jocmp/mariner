//! An RSS `<image>`.

use crate::rules::{Class, Field, text};

pub(super) static RSS_IMAGE: Class = Class::new(&[
    text("description", Field::Other),
    text("height", Field::Other),
    text("link", Field::Other),
    text("title", Field::Other),
    text("url", Field::Url),
    text("width", Field::Other),
]);
