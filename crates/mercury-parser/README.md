# mercury-parser

A Rust port of [Mercury Parser](https://github.com/jocmp/mercury-parser). Extracts the meaningful content from a web page: title, author, publish date, lead image, content, and more.

The port matches the JavaScript implementation's output exactly — including content HTML — for every fixture in the upstream repo. See [Parity](#parity).

## Crates

- `crates/mercury-parser/`: the library (`mercury_parser`)
- `crates/mercury-cli/`: the `mercury-parser` command-line tool

## Usage

```toml
[dependencies]
mercury-parser = { path = "../mercury-parser" }
```

```rust
#[tokio::main]
async fn main() -> Result<(), mercury_parser::Error> {
    let result = mercury_parser::parse("https://example.com/article", Default::default()).await?;

    println!("{:?}", result.title);
    println!("{:?}", result.content);
    Ok(())
}
```

`ParseOptions` mirrors upstream's options:

```rust
use mercury_parser::{ContentType, Field, ParseOptions};

let options = ParseOptions::new()
    .content_type(ContentType::Markdown) // or Html (default), Text
    .fetch_all_pages(false)              // don't follow next-page links
    .header("Cookie", "session=…")
    .extend("tags", Field::new().selectors([".tag"]).allow_multiple(true));
```

If you already have the HTML, skip the network entirely:

```rust
use mercury_parser::{ParseOptions, Parser};

let result = Parser::new().parse_html("https://example.com/article", &html, &ParseOptions::default())?;
```

`ParseResult` serializes with serde to the same JSON shape as upstream.

### Features

- `fetch` (default): fetch pages with `reqwest`. Without it, pass HTML through `ParseOptions::html` or use `Parser::parse_html`.

## Custom extractors

Site-specific extraction works the same way as upstream: a declarative description of where each field lives, with the generic extractor filling in anything left out. Each JS object literal under `src/extractors/custom/` has a Rust equivalent under `src/extractors/custom/`:

```js
// JavaScript
export const NYTimesExtractor = {
  domain: 'www.nytimes.com',
  title: { selectors: ['h1[data-testid="headline"]', 'h1.g-headline'] },
  author: { selectors: [['meta[name="author"]', 'value'], '.g-byline'] },
  content: {
    selectors: ['section[name="articleBody"]'],
    transforms: {
      'img.g-lazy': $node => {
        $node.attr('src', $node.attr('src').replace('{{size}}', 640));
      },
      h1: 'h2',
    },
    clean: ['.ad', '.promo'],
  },
  date_published: { selectors: [['meta[name="article:published_time"]', 'value']] },
};
```

```rust
// Rust
use mercury_parser::{CustomExtractor, Field};

pub fn extractor() -> CustomExtractor {
    CustomExtractor::new("www.nytimes.com")
        .title(Field::new().selectors([r#"h1[data-testid="headline"]"#, "h1.g-headline"]))
        .author(Field::new().attr(r#"meta[name="author"]"#, "value").selector(".g-byline"))
        .content(
            Field::new()
                .selectors([r#"section[name="articleBody"]"#])
                .transform("img.g-lazy", |node| {
                    if let Some(src) = node.attr("src") {
                        node.set_attr("src", &src.replace("{{size}}", "640"));
                    }
                    None // or Some("tag".into()) to convert the node
                })
                .rename("h1", "h2")
                .clean([".ad", ".promo"]),
        )
        .date_published(Field::new().attr(r#"meta[name="article:published_time"]"#, "value"))
}
```

| Upstream | Rust |
| --- | --- |
| `'css'` | `.selector("css")` / `.selectors([...])` |
| `['css', 'attr']` | `.attr("css", "attr")` |
| `['css', 'attr', fn]` | `.attr_with("css", "attr", \|value\| ...)` |
| `[['a', 'b']]` (content) | `.group(["a", "b"])` |
| `clean: [...]` | `.clean([...])` |
| `transforms: { css: 'tag' }` | `.rename("css", "tag")` |
| `transforms: { css: fn }` | `.transform("css", \|node\| ...)` |
| `defaultCleaner: false` | `.default_cleaner(false)` |
| `allowMultiple: true` | `.allow_multiple(true)` |
| `timezone` / `format` | `.timezone(..)` / `.format(..)` |
| `author: 'Staff'` | `Field::literal("Staff")` |
| `supportedDomains` | `.supported_domains([...])` |
| `extend: { name: {...} }` | `.extend("name", Field...)` |

Transforms receive a [`dom_query`](https://docs.rs/dom_query) `Selection` for the matched node (re-exported as `mercury_parser::dom_query`).

Register your own at runtime; added extractors take precedence over built-in ones:

```rust
let mut parser = mercury_parser::Parser::new();
parser.add_extractor(my_site::extractor());
```

or for a single call with `ParseOptions::custom_extractor`.

Extractors without function transforms can also be loaded from JSON in upstream's object shape, which is how the CLI's `--add-extractor` works:

```rust
let extractor = mercury_parser::CustomExtractor::from_json(r#"{
  "domain": "example.com",
  "title": { "selectors": ["h1"] },
  "content": { "selectors": ["article"], "clean": [".ad"], "transforms": { "h1": "h2" } }
}"#)?;
```

### Adding a built-in extractor

1. Add `src/extractors/custom/<domain>.rs`, with dots and dashes in the domain replaced by underscores (and a leading `_` if it starts with a digit), exporting `pub fn extractor() -> CustomExtractor`.
2. Declare the module and add it to `all()` in `src/extractors/custom.rs`.
3. Add a fixture and snapshot (see below).

## CLI

```
cargo install --path crates/mercury-cli
mercury-parser https://example.com/article [--format html|markdown|text] \
  [--header name=value]... [--extend name=selector]... [--extend-list name=selector]... \
  [--add-extractor extractor.json]
```

## Layout

The module tree follows upstream's `src/` hierarchy, using Rust's `foo.rs` + `foo/` convention. Directories of one-function JS files are collapsed into a single module where that reads better:

| Upstream | Rust |
| --- | --- |
| `mercury.js` | `parser.rs` (`Parser`, `ParseOptions`, `ParseResult`) |
| `resource/` | `resource.rs`, `resource/fetch.rs` |
| `cleaners/*.js` | `cleaners/*.rs` |
| `extractors/root-extractor.js` | `extractors/root_extractor.rs` |
| `extractors/generic/<field>/` | `extractors/generic/<field>.rs` |
| `extractors/generic/content/scoring/*.js` | `extractors/generic/content/scoring.rs` |
| `extractors/custom/<domain>/index.js` | `extractors/custom/<domain>.rs` |
| `utils/dom/*.js` | `utils/dom.rs` + `utils/dom/{clean,convert,extract,links}.rs` |
| `utils/text/*.js` | `utils/text.rs` |

Upstream relies on a few libraries whose exact behavior shows up in results. These are reimplemented rather than approximated:

- `new Date(string)` (V8's date parser) and dayjs's `customParseFormat`/`timezone` plugins: `cleaners/date_published/`
- cheerio's parsing and serialization (parse5, scripting enabled): `utils/dom.rs`, `utils/dom/serialize.rs`
- difflib's `SequenceMatcher`: `utils/difflib.rs`
- JavaScript string semantics (UTF-16 lengths, `\s`, `trim`, `parseFloat`): `utils/js.rs`

## Parity

`tests/snapshot_parity.rs` runs every upstream fixture through the parser and compares each field with the JavaScript output recorded in `tests/snapshots/`. To regenerate the fixtures and snapshots from a checkout of the JS repo next to the mariner checkout, from the mariner root:

```
(cd ../mercury-parser && npm install && npm run build)
TZ=UTC node crates/mercury-parser/scripts/dump-snapshots.mjs
```

`MERCURY_ONLY=<prefix> MERCURY_VERBOSE=1 cargo test -p mercury-parser --test snapshot_parity -- --nocapture` narrows the run and prints where each mismatch diverges.

The date cleaner has its own corpus of about 27,000 JS results in `tests/dates/`, generated by `scripts/date-corpus/`.

Known differences:

- Markdown output uses [htmd](https://crates.io/crates/htmd), configured like turndown's defaults. It's close to upstream's output, but not identical.
- Offset-less dates are interpreted as UTC. Upstream uses the machine's local timezone.

## License

MIT. See [LICENSE](../../LICENSE).
