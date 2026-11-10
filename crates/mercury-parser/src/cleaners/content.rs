//! `src/cleaners/content.js`.

use dom_query::{Document, Selection};

use crate::utils::dom::{
    clean_attributes, clean_h_ones, clean_headers, clean_images, clean_tags, make_links_absolute,
    mark_to_keep, remove_empty, rewrite_top_level, strip_junk_tags,
};

/// Clean article content in place.
///
/// `clean_conditionally` is accepted for parity with upstream, which
/// passes it along but never reads it.
pub fn clean_content(
    article: &Selection<'_>,
    doc: &Document,
    _clean_conditionally: bool,
    title: Option<&str>,
    url: &str,
    default_cleaner: bool,
) {
    // Rewrite the tag name to div if it's a top level node like body or
    // html to avoid later complications with multiple body tags.
    rewrite_top_level(doc);

    // Drop small images and spacer images. Only do this if default_cleaner
    // is set; this can sometimes be too aggressive.
    if default_cleaner {
        clean_images(article);
    }

    make_links_absolute(article, doc, url);

    // Mark elements to keep that would normally be removed. E.g.,
    // strip_junk_tags will remove iframes, so we're going to mark
    // YouTube/Vimeo videos as elements we want to keep.
    mark_to_keep(article, url);

    // Drop certain tags like <title>, etc. This is mostly for cleanliness,
    // not security.
    strip_junk_tags(article);

    // H1 tags are typically the article title, which should be extracted by
    // the title extractor instead.
    clean_h_ones(article);

    clean_headers(article, title);

    // We used to clean UL's and OL's here, but it was leading to too many
    // in-article lists being removed.
    if default_cleaner {
        clean_tags(article);
    }

    remove_empty(article);
    clean_attributes(article);
}
