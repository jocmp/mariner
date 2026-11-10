//! Extractor behavior checked against fixtures in `tests/custom_fixtures`.

use std::fs;
use std::path::Path;

use dom_query::Document;
use mercury_parser::{ParseOptions, ParseResult, Parser};
use regex::Regex;

const ROSA_URL: &str = "https://www.rosa.gr/athlitismos/cheimerinoi-olybiakoi-i-doe-apekleise-ton-oukrano-athliti-pou-forese-kranos-me-ta-thymata-tou-polemou/";
const ROSA_FIXTURE: &str =
    "www.rosa.gr/cheimerinoi-olybiakoi-i-doe-apekleise-ton-oukrano-athliti.html";

#[test]
fn androidauthority_cleans_the_add_on_google_badges() {
    let content = androidauthority_content();

    assert!(!content.contains("Add Android Authority on Google"));
    assert!(!content.contains("Google_Symbol_0.svg"));
    assert!(!content.contains("Google_Discover-1.svg"));
}

#[test]
fn androidauthority_cleans_the_affiliate_disclaimer() {
    assert!(!androidauthority_content().contains("Affiliate links on Android Authority"));
}

#[test]
fn androidauthority_keeps_the_article_body() {
    assert!(androidauthority_content().contains("on a mission to replace as many Android apps"));
}

#[test]
fn npr_expands_brightspot_image_templates() {
    let content = parse(
        "https://www.npr.org/2026/10/26/nx-s1-6008167/republican-redistricting-midterms-competitive-map",
        &fixture("www.npr.org/republican-redistricting-midterms-competitive-map.html"),
    )
    .content
    .unwrap_or_default();

    assert!(
        !Regex::new(r"(?i)\{\w+\}|%7B\w+%7D")
            .unwrap()
            .is_match(&content)
    );
    assert!(content.contains("/resize/1200/quality/85/format/webp/?url=http%3A%2F%2Fnpr-brightspot.s3.amazonaws.com%2F06%2F6a%2F46ec36764e5db11da70604db9122%2Fgettyimages-2301229190.jpg"));
}

#[test]
fn rosa_returns_the_metadata() {
    let result = parse(ROSA_URL, &fixture(ROSA_FIXTURE));

    assert_eq!(
        result.title.as_deref(),
        Some(
            "Χειμερινοί Ολυμπιακοί: Η ΔΟΕ απέκλεισε τον Ουκρανό αθλητή που φόρεσε κράνος με τα θύματα του πολέμου"
        ),
    );
    assert_eq!(result.author.as_deref(), Some("Ιωάννα Καρδούλια"));
    assert_eq!(
        result.date_published.as_deref(),
        Some("2026-02-12T12:12:00.000Z"),
    );
    assert_eq!(
        result.lead_image_url.as_deref(),
        Some("https://www.rosa.gr/wp-content/uploads/2026/02/ukraine-athlitis.jpg"),
    );
}

#[test]
fn rosa_keeps_the_hero_image_as_a_figure_ahead_of_the_body() {
    let doc = rosa_document(&fixture(ROSA_FIXTURE));
    let figures = doc.select("figure");

    assert_eq!(figures.length(), 1);
    assert_eq!(
        figures.select("img").first().attr("src").as_deref(),
        Some("https://www.rosa.gr/wp-content/uploads/2026/02/ukraine-athlitis.jpg"),
    );
    assert!(doc.select("figure + .post-text").exists());
}

#[test]
fn rosa_drops_a_bare_credit_label() {
    let doc = rosa_document(&fixture(ROSA_FIXTURE));

    assert!(!doc.select("figcaption, .image-credits").exists());
}

#[test]
fn rosa_keeps_a_real_credit_as_the_figure_caption() {
    let html = fixture(ROSA_FIXTURE).replace(
        r#"<span class="image-credits">X</span>"#,
        r#"<span class="image-credits">AP</span>"#,
    );
    let doc = rosa_document(&html);
    let captions: Vec<_> = doc
        .select("figure > figcaption")
        .iter()
        .map(|caption| caption.text().to_string())
        .collect();

    assert_eq!(captions, ["AP"]);
}

#[test]
fn rosa_cleans_the_read_also_block() {
    let content = rosa_content(&fixture(ROSA_FIXTURE));

    assert!(
        !Document::from(content.as_str())
            .select(".short-read-also")
            .exists()
    );
    assert!(!content.contains("cheimerinoi-olybiakoi-o-foros-timis-oukranou-athliti"));
}

#[test]
fn rosa_excludes_the_social_icons() {
    let doc = rosa_document(&fixture(ROSA_FIXTURE));

    assert!(
        !doc.select(r#"svg, img[src*="rosa-theme/assets/images/"]"#)
            .exists()
    );
}

#[test]
fn rosa_keeps_the_article_body() {
    let content = rosa_content(&fixture(ROSA_FIXTURE));

    assert!(content.contains("Αποκλείστηκε με απόφαση της ΔΟΕ"));
    assert!(content.contains("Ο υπουργός Εξωτερικών της χώρας"));
}

fn androidauthority_content() -> String {
    parse(
        "https://www.androidauthority.com/popular-android-apps-open-source-alternatives-3725412/",
        &fixture("www.androidauthority.com/popular-android-apps-open-source-alternatives.html"),
    )
    .content
    .unwrap_or_default()
}

fn rosa_document(html: &str) -> Document {
    Document::from(rosa_content(html).as_str())
}

fn rosa_content(html: &str) -> String {
    parse(ROSA_URL, html).content.unwrap_or_default()
}

fn parse(url: &str, html: &str) -> ParseResult {
    Parser::new()
        .parse_html(url, html, &ParseOptions::default())
        .unwrap()
}

fn fixture(name: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/custom_fixtures")
        .join(name);
    fs::read_to_string(path).unwrap()
}
