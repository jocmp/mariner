//! The public entry point, from `src/mercury.js`.

use std::collections::HashMap;
use std::sync::Arc;

use dom_query::Document;
use serde::Serialize;
use serde_json::Value;
use url::Url;

use crate::error::Error;
#[cfg(feature = "fetch")]
use crate::extractors::collect_all_pages::collect_all_pages;
use crate::extractors::root_extractor::{self, Context, Extracted};
use crate::extractors::{CustomExtractor, Field, get_extractor, index};
use crate::resource;
use crate::utils::dom::{fragment_nodes, parse_fragment, select, text};
use crate::utils::url::{parse_url, validate_url};

/// The format of [`ParseResult::content`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ContentType {
    #[default]
    Html,
    Markdown,
    Text,
}

/// Options for [`Parser::parse`].
#[derive(Clone, Debug)]
pub struct ParseOptions {
    /// The page's HTML. When set, the page isn't fetched.
    pub html: Option<String>,
    /// Follow `next_page_url` links and append each page's content.
    /// Defaults to `true`.
    pub fetch_all_pages: bool,
    /// Use the generic extractor for fields a custom extractor misses.
    /// Defaults to `true`.
    pub fallback: bool,
    pub content_type: ContentType,
    /// Extra request headers.
    pub headers: Vec<(String, String)>,
    /// Additional fields to extract into [`ParseResult::extended`].
    pub extend: Vec<(String, Field)>,
    /// An extractor to use for this call, ahead of any registered one for
    /// the URL's domain.
    pub custom_extractor: Option<CustomExtractor>,
}

impl Default for ParseOptions {
    fn default() -> Self {
        Self {
            html: None,
            fetch_all_pages: true,
            fallback: true,
            content_type: ContentType::Html,
            headers: Vec::new(),
            extend: Vec::new(),
            custom_extractor: None,
        }
    }
}

impl ParseOptions {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn html(mut self, html: impl Into<String>) -> Self {
        self.html = Some(html.into());
        self
    }

    pub fn fetch_all_pages(mut self, enabled: bool) -> Self {
        self.fetch_all_pages = enabled;
        self
    }

    pub fn fallback(mut self, enabled: bool) -> Self {
        self.fallback = enabled;
        self
    }

    pub fn content_type(mut self, content_type: ContentType) -> Self {
        self.content_type = content_type;
        self
    }

    pub fn header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.push((name.into(), value.into()));
        self
    }

    pub fn extend(mut self, name: impl Into<String>, field: Field) -> Self {
        self.extend.push((name.into(), field));
        self
    }

    pub fn custom_extractor(mut self, extractor: CustomExtractor) -> Self {
        self.custom_extractor = Some(extractor);
        self
    }
}

/// The extracted article. Serializes to the same JSON shape as upstream.
#[derive(Clone, Debug, Default, PartialEq, Serialize)]
pub struct ParseResult {
    pub title: Option<String>,
    pub content: Option<String>,
    pub author: Option<String>,
    pub date_published: Option<String>,
    pub lead_image_url: Option<String>,
    pub dek: Option<String>,
    pub next_page_url: Option<String>,
    pub url: Option<String>,
    pub domain: Option<String>,
    pub excerpt: Option<String>,
    pub word_count: Option<usize>,
    pub direction: Option<String>,
    pub total_pages: usize,
    pub rendered_pages: usize,
    /// Fields from the extractor's `extend` and [`ParseOptions::extend`].
    #[serde(flatten)]
    pub extended: serde_json::Map<String, Value>,
}

/// Parses articles, using the built-in custom extractors plus any added
/// with [`Parser::add_extractor`].
#[derive(Clone, Debug, Default)]
pub struct Parser {
    extractors: HashMap<String, Arc<CustomExtractor>>,
}

impl Parser {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register an extractor for its domains. Added extractors take
    /// precedence over the built-in ones.
    pub fn add_extractor(&mut self, extractor: CustomExtractor) -> &mut Self {
        self.extractors.extend(index([extractor]));
        self
    }

    /// Fetch (unless [`ParseOptions::html`] is set) and parse the article
    /// at `url`, following next-page links when enabled.
    pub async fn parse(&self, url: &str, options: ParseOptions) -> Result<ParseResult, Error> {
        let parsed_url = parse_input_url(url)?;

        let html = match &options.html {
            Some(html) => html.clone(),
            None => fetch_html(url, &options.headers).await?,
        };
        let (mut result, next_page_url, extractor, meta_cache) =
            self.parse_page(url, &parsed_url, &html, &options)?;

        #[cfg(feature = "fetch")]
        if options.fetch_all_pages
            && let Some(next_page_url) = next_page_url
        {
            result = collect_all_pages(
                extractor.as_deref(),
                result,
                next_page_url,
                url,
                &meta_cache,
            )
            .await?;
        }
        #[cfg(not(feature = "fetch"))]
        let _ = (next_page_url, extractor, meta_cache);

        convert_content(&mut result, options.content_type);
        Ok(result)
    }

    /// Parse `html` as the page at `url`, without any network access.
    /// Next-page links are reported in [`ParseResult::next_page_url`] but
    /// not followed.
    pub fn parse_html(
        &self,
        url: &str,
        html: &str,
        options: &ParseOptions,
    ) -> Result<ParseResult, Error> {
        let parsed_url = parse_input_url(url)?;
        let (mut result, ..) = self.parse_page(url, &parsed_url, html, options)?;
        convert_content(&mut result, options.content_type);
        Ok(result)
    }

    /// The extractor that would be used for `url` and its markup, or `None`
    /// for the generic extractor.
    pub fn extractor_for(&self, url: &Url, doc: &Document) -> Option<Arc<CustomExtractor>> {
        get_extractor(url, doc, &self.extractors)
    }

    #[allow(clippy::type_complexity)]
    fn parse_page(
        &self,
        url: &str,
        parsed_url: &Url,
        html: &str,
        options: &ParseOptions,
    ) -> Result<
        (
            ParseResult,
            Option<String>,
            Option<Arc<CustomExtractor>>,
            Vec<String>,
        ),
        Error,
    > {
        let doc = resource::from_html(html)?;

        let extractor = match &options.custom_extractor {
            Some(custom) => {
                let mut extractors = self.extractors.clone();
                extractors.extend(index([custom.clone()]));
                get_extractor(parsed_url, &doc, &extractors)
            }
            None => self.extractor_for(parsed_url, &doc),
        };

        // Every meta name in the document, used when extracting title,
        // author, date_published, and dek.
        let meta_cache = meta_names(&doc);

        let ctx = Context {
            doc: &doc,
            url,
            parsed_url,
            html,
            meta_cache: &meta_cache,
            fallback: options.fallback,
            previous_urls: &[],
        };

        let extended = root_extractor::select_extended_types(&options.extend, &ctx);
        let extracted = root_extractor::extract(extractor.as_deref(), &ctx);

        let next_page_url = extracted.next_page_url.clone();
        let mut result = into_result(extracted);
        result.extended.extend(extended);

        Ok((result, next_page_url, extractor, meta_cache))
    }
}

/// Parse the article at `url` with the built-in extractors.
pub async fn parse(url: &str, options: ParseOptions) -> Result<ParseResult, Error> {
    Parser::new().parse(url, options).await
}

fn parse_input_url(url: &str) -> Result<Url, Error> {
    let parsed = parse_url(url);
    if !validate_url(parsed.as_ref()) {
        return Err(Error::InvalidUrl);
    }
    parsed.ok_or(Error::InvalidUrl)
}

#[cfg(feature = "fetch")]
async fn fetch_html(url: &str, headers: &[(String, String)]) -> Result<String, Error> {
    let response = resource::fetch_resource(url, headers).await?;
    let doc = resource::from_bytes(&response.body, &response.content_type)?;
    Ok(crate::utils::dom::inner_html(&doc.root()))
}

#[cfg(not(feature = "fetch"))]
async fn fetch_html(_url: &str, _headers: &[(String, String)]) -> Result<String, Error> {
    Err(Error::FetchDisabled)
}

fn meta_names(doc: &Document) -> Vec<String> {
    select(doc, "meta")
        .nodes()
        .iter()
        .filter_map(|node| node.attr("name").map(|name| name.to_string()))
        .collect()
}

fn into_result(extracted: Extracted) -> ParseResult {
    ParseResult {
        title: extracted.title,
        content: extracted.content,
        author: extracted.author,
        date_published: extracted.date_published,
        lead_image_url: extracted.lead_image_url,
        dek: extracted.dek,
        next_page_url: extracted.next_page_url,
        url: extracted.url,
        domain: extracted.domain,
        excerpt: extracted.excerpt,
        word_count: extracted.word_count,
        direction: extracted.direction,
        total_pages: 1,
        rendered_pages: 1,
        extended: extracted.extended.into_iter().collect(),
    }
}

// Markdown with turndown's defaults, which upstream uses: setext
// headings, indented code blocks, and `_` for emphasis.
fn to_markdown(html: &str) -> String {
    use htmd::element_handler::{HandlerResult, Handlers};
    use htmd::options::{CodeBlockStyle, HeadingStyle, Options};

    let converter = htmd::HtmlToMarkdown::builder()
        .options(Options {
            heading_style: HeadingStyle::Setex,
            code_block_style: CodeBlockStyle::Indented,
            ..Options::default()
        })
        .add_handler(
            vec!["em", "i"],
            |handlers: &dyn Handlers, element: htmd::Element| {
                let content = handlers.walk_children(element.node).content;
                let trimmed = content.trim();
                if trimmed.is_empty() {
                    return None;
                }
                let leading = &content[..content.len() - content.trim_start().len()];
                let trailing = &content[content.trim_end().len()..];
                Some(HandlerResult::from(format!(
                    "{leading}_{trimmed}_{trailing}"
                )))
            },
        )
        .build();

    converter.convert(html).unwrap_or_default()
}

fn convert_content(result: &mut ParseResult, content_type: ContentType) {
    let Some(content) = &result.content else {
        return;
    };

    match content_type {
        ContentType::Html => {}
        ContentType::Markdown => {
            result.content = Some(to_markdown(content));
        }
        ContentType::Text => {
            let fragment = parse_fragment(content);
            result.content = Some(text(&fragment_nodes(&fragment)));
        }
    }
}
