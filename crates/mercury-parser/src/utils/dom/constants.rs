//! Constants from `src/utils/dom/constants.js`.

use std::sync::LazyLock;

use regex::Regex;

// Spacer images to be removed
pub static SPACER_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new("(?i)transparent|spacer|blank").unwrap());

// The class we will use to mark elements we want to keep but would
// normally remove
pub const KEEP_CLASS: &str = "mercury-parser-keep";

pub const KEEP_SELECTORS: &[&str] = &[
    r#"iframe[src^="https://www.youtube.com"]"#,
    r#"iframe[src^="https://www.youtube-nocookie.com"]"#,
    r#"iframe[src^="http://www.youtube.com"]"#,
    r#"iframe[src^="https://player.vimeo"]"#,
    r#"iframe[src^="http://player.vimeo"]"#,
    r#"iframe[src^="https://www.redditmedia.com"]"#,
];

// A list of tags to strip from the output if we encounter them.
pub const STRIP_OUTPUT_TAGS: &[&str] = &[
    "title", "script", "noscript", "link", "style", "hr", "embed", "iframe", "object",
];

pub const WHITELIST_ATTRS: &[&str] = &[
    "src",
    "srcset",
    "start",
    "sizes",
    "type",
    "href",
    "class",
    "id",
    "alt",
    "xlink:href",
    "width",
    "height",
];

pub const CLEAN_CONDITIONALLY_TAGS: &str = "ul,ol,table,div,button,form";

pub const HEADER_TAG_LIST: &str = "h2,h3,h4,h5,h6";

// A list of strings that can be considered unlikely candidates when
// extracting content from a resource.
const UNLIKELY_CANDIDATES_BLACKLIST: &[&str] = &[
    "ad-break",
    "adbox",
    "advert",
    "addthis",
    "agegate",
    "aux",
    "blogger-labels",
    "combx",
    "comment",
    "conversation",
    "disqus",
    "entry-unrelated",
    "extra",
    "foot",
    // "form", // This is too generic, has too many false positives
    "header",
    "hidden",
    "loader",
    "login", // Note: This can hit 'blogindex'.
    "menu",
    "meta",
    "nav",
    "outbrain",
    "pager",
    "pagination",
    "predicta",                  // readwriteweb inline ad box
    "presence_control_external", // lifehacker.com container full of false positives
    "popup",
    "printfriendly",
    "related",
    "remove",
    "remark",
    "rss",
    "share",
    "shoutbox",
    "sidebar",
    "sociable",
    "sponsor",
    "taboola",
    "tools",
];

// The inverse of the blacklist above: if something matches both, it is
// kept. Useful for, e.g., a className of "rss-content entry-content".
const UNLIKELY_CANDIDATES_WHITELIST: &[&str] = &[
    "and",
    "article",
    "body",
    "blogindex",
    "column",
    "content",
    "entry-content-asset",
    "format", // misuse of form
    "hfeed",
    "hentry",
    "hatom",
    "main",
    "page",
    "posts",
    "shadow",
];

// Tags which, if found inside, should cause a <div /> to NOT be turned into
// a paragraph tag.
pub const DIV_TO_P_BLOCK_TAGS: &str = "a,blockquote,dl,div,img,p,pre,table";

pub static PHOTO_HINTS_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new("(?i)figure|photo|image|caption").unwrap());

// Strings that denote a positive scoring for this content as being an
// article container. Checked against className and id.
const POSITIVE_SCORE_HINTS: &[&str] = &[
    "article",
    "articlecontent",
    "instapaper_body",
    "blog",
    "body",
    "content",
    "entry-content-asset",
    "entry",
    "hentry",
    "main",
    "Normal",
    "page",
    "pagination",
    "permalink",
    "post",
    "story",
    "text",
    "[-_]copy", // usatoday
    r"(?-u:\B)copy",
];

pub static POSITIVE_SCORE_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(&format!("(?i){}", POSITIVE_SCORE_HINTS.join("|"))).unwrap());

// Readability publisher-specific guidelines
pub static READABILITY_ASSET: LazyLock<Regex> =
    LazyLock::new(|| Regex::new("(?i)entry-content-asset").unwrap());

// Strings that denote a negative scoring for this content as being an
// article container. Checked against className and id.
const NEGATIVE_SCORE_HINTS: &[&str] = &[
    "adbox",
    "advert",
    "author",
    "bio",
    "bookmark",
    "bottom",
    "byline",
    "clear",
    "com-",
    "combx",
    "comment",
    r"comment(?-u:\B)",
    "contact",
    "copy",
    "credit",
    "crumb",
    "date",
    "deck",
    "excerpt",
    "featured", // tnr.com has a featured_content which throws us off
    "foot",
    "footer",
    "footnote",
    "graf",
    "head",
    "info",
    "infotext", // newscientist.com copyright
    "instapaper_ignore",
    "jump",
    "linebreak",
    "link",
    "masthead",
    "media",
    "meta",
    "modal",
    "outbrain", // slate.com junk
    "promo",
    "pr_", // autoblog - press release
    "related",
    "respond",
    "roundcontent", // lifehacker restricted content warning
    "scroll",
    "secondary",
    "share",
    "shopping",
    "shoutbox",
    "side",
    "sidebar",
    "sponsor",
    "stamp",
    "sub",
    "summary",
    "tags",
    "tools",
    "widget",
];

pub static NEGATIVE_SCORE_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(&format!("(?i){}", NEGATIVE_SCORE_HINTS.join("|"))).unwrap());

// Try to determine if a page is wordpress. Not always successful.
pub const IS_WP_SELECTOR: &str = "meta[name=generator][value^=WordPress]";

// Words that, if found in link text or URLs, likely mean that this link is
// not a next page link.
pub static EXTRANEOUS_LINK_HINTS_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        "(?i)print|archive|comment|discuss|e-mail|email|share|reply|all|login|sign|single|adx|entry-unrelated",
    )
    .unwrap()
});

// Match any phrase that looks like it could be page, or paging, or
// pagination
pub static PAGE_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new("(?i)pag(e|ing|inat)").unwrap());

// All of the block level tags known in HTML5 and below.
pub const BLOCK_LEVEL_TAGS: &[&str] = &[
    "article",
    "aside",
    "blockquote",
    "body",
    "br",
    "button",
    "canvas",
    "caption",
    "col",
    "colgroup",
    "dd",
    "div",
    "dl",
    "dt",
    "embed",
    "fieldset",
    "figcaption",
    "figure",
    "footer",
    "form",
    "h1",
    "h2",
    "h3",
    "h4",
    "h5",
    "h6",
    "header",
    "hgroup",
    "hr",
    "li",
    "map",
    "object",
    "ol",
    "output",
    "p",
    "pre",
    "progress",
    "section",
    "table",
    "tbody",
    "textarea",
    "tfoot",
    "th",
    "thead",
    "tr",
    "ul",
    "video",
];

pub static CANDIDATES_BLACKLIST: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(&format!("(?i){}", UNLIKELY_CANDIDATES_BLACKLIST.join("|"))).unwrap()
});

pub static CANDIDATES_WHITELIST: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(&format!("(?i){}", UNLIKELY_CANDIDATES_WHITELIST.join("|"))).unwrap()
});
