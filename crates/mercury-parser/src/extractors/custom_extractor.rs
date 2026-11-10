//! Declarative extractor definitions.
//!
//! Mirrors the object literals that live under `src/extractors/custom/` in
//! the JavaScript version:
//!
//! ```js
//! export const ExampleExtractor = {
//!   domain: 'example.com',
//!   title: { selectors: ['h1.title'] },
//!   author: { selectors: [['meta[name="author"]', 'value']] },
//!   content: {
//!     selectors: ['.article-body'],
//!     transforms: { h2: 'h3' },
//!     clean: ['.ad'],
//!   },
//! };
//! ```
//!
//! becomes
//!
//! ```
//! use mercury_parser::{CustomExtractor, Field};
//!
//! let extractor = CustomExtractor::new("example.com")
//!     .title(Field::new().selectors(["h1.title"]))
//!     .author(Field::new().attr(r#"meta[name="author"]"#, "value"))
//!     .content(
//!         Field::new()
//!             .selectors([".article-body"])
//!             .rename("h2", "h3")
//!             .clean([".ad"]),
//!     );
//! ```

use std::fmt;
use std::sync::Arc;

use dom_query::Selection;

/// A function transform applied to every node matching a transform selector.
///
/// Mirrors a function value in upstream's `transforms` map: it may mutate the
/// node in place, and returning `Some(tag)` converts the node to that tag
/// (the JS equivalent of returning a string).
pub type TransformFn = dyn Fn(&Selection<'_>) -> Option<String> + Send + Sync;

/// Post-processes an attribute value, like the third element of upstream's
/// `[selector, attribute, fn]` tuple.
pub type ValueFn = dyn Fn(&str) -> String + Send + Sync;

/// One entry in a field's `selectors` list.
#[derive(Clone)]
pub enum Selector {
    /// A plain CSS selector; the matched node's text (or HTML, for content)
    /// is extracted.
    Css(String),
    /// Upstream's `[selector, attribute]` tuple: read an attribute off the
    /// matched node, optionally post-processing the value.
    Attr {
        css: String,
        attr: String,
        map: Option<Arc<ValueFn>>,
    },
    /// Upstream's nested array for content, e.g. `[['.lede', '.body']]`.
    /// Every selector must match, and all matched nodes are combined into a
    /// single wrapper in document order.
    Group(Vec<String>),
}

impl fmt::Debug for Selector {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Selector::Css(css) => f.debug_tuple("Css").field(css).finish(),
            Selector::Attr { css, attr, map } => f
                .debug_struct("Attr")
                .field("css", css)
                .field("attr", attr)
                .field("map", &map.as_ref().map(|_| "fn"))
                .finish(),
            Selector::Group(parts) => f.debug_tuple("Group").field(parts).finish(),
        }
    }
}

/// A value in a field's `transforms` map.
#[derive(Clone)]
pub enum Transform {
    /// Convert matching nodes to the given tag, e.g. `{ h1: 'h2' }`.
    Rename(String),
    /// Run a function against each matching node.
    Fn(Arc<TransformFn>),
}

impl fmt::Debug for Transform {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Transform::Rename(tag) => f.debug_tuple("Rename").field(tag).finish(),
            Transform::Fn(_) => f.write_str("Fn"),
        }
    }
}

/// Extraction options for one result field (title, content, etc.).
#[derive(Clone, Debug)]
pub struct Field {
    pub selectors: Vec<Selector>,
    /// Selectors removed from the matched content before it is returned.
    pub clean: Vec<String>,
    /// Ordered `selector => transform` pairs, applied after `clean`.
    pub transforms: Vec<(String, Transform)>,
    /// Run the field's default cleaner on the result. Defaults to `true`.
    pub default_cleaner: bool,
    /// Return every match instead of requiring exactly one.
    pub allow_multiple: bool,
    /// A hardcoded value, like `author: 'TMZ STAFF'` upstream. When set, the
    /// selectors are ignored.
    pub literal: Option<String>,
    /// IANA timezone used to interpret `date_published` values that carry no
    /// offset of their own.
    pub timezone: Option<String>,
    /// A dayjs-style format string, e.g. `YYYY年M月D日`, used to parse
    /// `date_published`.
    pub format: Option<String>,
}

impl Default for Field {
    fn default() -> Self {
        Self {
            selectors: Vec::new(),
            clean: Vec::new(),
            transforms: Vec::new(),
            default_cleaner: true,
            allow_multiple: false,
            literal: None,
            timezone: None,
            format: None,
        }
    }
}

impl Field {
    pub fn new() -> Self {
        Self::default()
    }

    /// A field with a constant value.
    pub fn literal(value: impl Into<String>) -> Self {
        Self {
            literal: Some(value.into()),
            ..Self::default()
        }
    }

    /// Append a plain CSS selector.
    pub fn selector(mut self, css: impl Into<String>) -> Self {
        self.selectors.push(Selector::Css(css.into()));
        self
    }

    /// Append several plain CSS selectors, in order.
    pub fn selectors<I, S>(mut self, css: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.selectors
            .extend(css.into_iter().map(|s| Selector::Css(s.into())));
        self
    }

    /// Append an attribute selector, like `['meta[name="og:image"]', 'value']`.
    pub fn attr(mut self, css: impl Into<String>, attr: impl Into<String>) -> Self {
        self.selectors.push(Selector::Attr {
            css: css.into(),
            attr: attr.into(),
            map: None,
        });
        self
    }

    /// Append an attribute selector whose value is post-processed by `map`.
    pub fn attr_with<F>(mut self, css: impl Into<String>, attr: impl Into<String>, map: F) -> Self
    where
        F: Fn(&str) -> String + Send + Sync + 'static,
    {
        self.selectors.push(Selector::Attr {
            css: css.into(),
            attr: attr.into(),
            map: Some(Arc::new(map)),
        });
        self
    }

    /// Append a group of selectors that must all match; see [`Selector::Group`].
    pub fn group<I, S>(mut self, css: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.selectors
            .push(Selector::Group(css.into_iter().map(Into::into).collect()));
        self
    }

    /// Selectors to remove from the extracted content.
    pub fn clean<I, S>(mut self, css: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.clean.extend(css.into_iter().map(Into::into));
        self
    }

    /// Convert nodes matching `css` to `tag`.
    pub fn rename(mut self, css: impl Into<String>, tag: impl Into<String>) -> Self {
        self.transforms
            .push((css.into(), Transform::Rename(tag.into())));
        self
    }

    /// Run `f` against each node matching `css`. Returning `Some(tag)`
    /// converts the node to `tag`.
    pub fn transform<F>(mut self, css: impl Into<String>, f: F) -> Self
    where
        F: Fn(&Selection<'_>) -> Option<String> + Send + Sync + 'static,
    {
        self.transforms
            .push((css.into(), Transform::Fn(Arc::new(f))));
        self
    }

    pub fn default_cleaner(mut self, enabled: bool) -> Self {
        self.default_cleaner = enabled;
        self
    }

    pub fn allow_multiple(mut self, enabled: bool) -> Self {
        self.allow_multiple = enabled;
        self
    }

    pub fn timezone(mut self, timezone: impl Into<String>) -> Self {
        self.timezone = Some(timezone.into());
        self
    }

    pub fn format(mut self, format: impl Into<String>) -> Self {
        self.format = Some(format.into());
        self
    }
}

/// A site-specific extractor. Any field left as `None` falls back to the
/// generic extractor at parse time.
#[derive(Clone, Debug, Default)]
pub struct CustomExtractor {
    pub domain: String,
    pub supported_domains: Vec<String>,
    pub title: Option<Field>,
    pub author: Option<Field>,
    pub content: Option<Field>,
    pub date_published: Option<Field>,
    pub lead_image_url: Option<Field>,
    pub dek: Option<Field>,
    pub next_page_url: Option<Field>,
    pub excerpt: Option<Field>,
    /// Additional named fields, merged into [`crate::ParseResult::extended`].
    pub extend: Vec<(String, Field)>,
}

impl CustomExtractor {
    pub fn new(domain: impl Into<String>) -> Self {
        Self {
            domain: domain.into(),
            ..Self::default()
        }
    }

    pub fn supported_domains<I, S>(mut self, domains: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.supported_domains
            .extend(domains.into_iter().map(Into::into));
        self
    }

    pub fn title(mut self, field: Field) -> Self {
        self.title = Some(field);
        self
    }

    pub fn author(mut self, field: Field) -> Self {
        self.author = Some(field);
        self
    }

    pub fn content(mut self, field: Field) -> Self {
        self.content = Some(field);
        self
    }

    pub fn date_published(mut self, field: Field) -> Self {
        self.date_published = Some(field);
        self
    }

    pub fn lead_image_url(mut self, field: Field) -> Self {
        self.lead_image_url = Some(field);
        self
    }

    pub fn dek(mut self, field: Field) -> Self {
        self.dek = Some(field);
        self
    }

    pub fn next_page_url(mut self, field: Field) -> Self {
        self.next_page_url = Some(field);
        self
    }

    pub fn excerpt(mut self, field: Field) -> Self {
        self.excerpt = Some(field);
        self
    }

    /// Add a custom field to the result, like upstream's `extend` option.
    pub fn extend(mut self, name: impl Into<String>, field: Field) -> Self {
        self.extend.push((name.into(), field));
        self
    }

    /// Every domain this extractor answers for: `domain` followed by
    /// `supported_domains`.
    pub fn domains(&self) -> impl Iterator<Item = &str> {
        std::iter::once(self.domain.as_str())
            .chain(self.supported_domains.iter().map(String::as_str))
    }
}
