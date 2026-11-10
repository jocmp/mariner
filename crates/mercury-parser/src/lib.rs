//! A Rust port of [Mercury Parser](https://github.com/jocmp/mercury-parser):
//! extract the meaningful content from a web page.
//!
//! ```no_run
//! # async fn run() -> Result<(), mercury_parser::Error> {
//! let result = mercury_parser::parse("https://example.com/article", Default::default()).await?;
//! println!("{:?}", result.title);
//! # Ok(())
//! # }
//! ```
//!
//! Site-specific behavior lives in [`CustomExtractor`]s. The built-in ones
//! are in [`extractors::custom`]; add your own with
//! [`Parser::add_extractor`]:
//!
//! ```
//! use mercury_parser::{CustomExtractor, Field, ParseOptions, Parser};
//!
//! let mut parser = Parser::new();
//! parser.add_extractor(
//!     CustomExtractor::new("example.com")
//!         .title(Field::new().selectors(["h1.headline"]))
//!         .content(Field::new().selectors(["article"]).clean([".ad"])),
//! );
//!
//! let html = r#"<html><body><h1 class="headline">Hello</h1>
//!     <article><p>World</p><div class="ad">Buy</div></article></body></html>"#;
//! let result = parser
//!     .parse_html("https://example.com/post", html, &ParseOptions::default())
//!     .unwrap();
//! assert_eq!(result.title.as_deref(), Some("Hello"));
//! assert_eq!(result.content.as_deref(), Some("<div><article><p>World</p></article></div>"));
//! ```

pub mod cleaners;
mod error;
pub mod extractors;
mod parser;
pub mod resource;
pub mod utils;

pub use dom_query;
pub use error::Error;
pub use extractors::{CustomExtractor, Field, Selector, Transform};
pub use parser::{ContentType, ParseOptions, ParseResult, Parser, parse};
