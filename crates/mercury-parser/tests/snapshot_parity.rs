//! Runs every fixture through the parser and compares the result with the
//! JS implementation's output, recorded by `scripts/dump-snapshots.mjs`.
//!
//! Set `MERCURY_ONLY=<prefix>` to run a subset, and `MERCURY_VERBOSE=1` to
//! print each mismatch.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use mercury_parser::{ParseOptions, Parser};
use serde_json::Value;

const FIELDS: &[&str] = &[
    "title",
    "author",
    "date_published",
    "dek",
    "lead_image_url",
    "next_page_url",
    "url",
    "domain",
    "excerpt",
    "word_count",
    "direction",
    "content",
];

// Snapshots whose values depend on the time they were recorded ("5 hours
// ago", "now").
const TIME_DEPENDENT: &[(&str, &str)] = &[("www.reddit.com", "date_published")];

#[test]
fn matches_js_snapshots() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests");
    let only = std::env::var("MERCURY_ONLY").ok();
    let verbose = std::env::var("MERCURY_VERBOSE").is_ok();
    let parser = Parser::new();

    let mut snapshots = Vec::new();
    collect(&root.join("snapshots"), &mut snapshots);
    snapshots.sort();

    let mut mismatches: BTreeMap<&str, Vec<String>> = BTreeMap::new();
    let mut total = 0;

    for snapshot in snapshots {
        let name = snapshot
            .strip_prefix(root.join("snapshots"))
            .unwrap()
            .with_extension("")
            .to_string_lossy()
            .into_owned();
        if only
            .as_ref()
            .is_some_and(|only| !name.starts_with(only.as_str()))
        {
            continue;
        }
        total += 1;

        let domain = name
            .split('/')
            .next()
            .unwrap()
            .split("--")
            .next()
            .unwrap()
            .to_string();
        let html = fs::read_to_string(root.join("fixtures").join(format!("{name}.html"))).unwrap();
        let expected: Value =
            serde_json::from_str(&fs::read_to_string(&snapshot).unwrap()).unwrap();

        let result = parser.parse_html(
            &format!("https://{domain}/"),
            &html,
            &ParseOptions::default(),
        );
        let actual = match result {
            Ok(result) => serde_json::to_value(result).unwrap(),
            Err(error) => {
                mismatches
                    .entry("error")
                    .or_default()
                    .push(format!("{name}: {error}"));
                continue;
            }
        };

        for field in FIELDS {
            if TIME_DEPENDENT.contains(&(domain.as_str(), field)) {
                continue;
            }
            let expected = expected.get(*field).unwrap_or(&Value::Null);
            let actual = actual.get(*field).unwrap_or(&Value::Null);
            if expected != actual {
                if verbose {
                    let (expected, actual) = difference(expected, actual);
                    println!("{name} {field}\n  expected: {expected}\n  actual:   {actual}");
                }
                mismatches.entry(field).or_default().push(name.clone());
            }
        }
    }

    println!("{total} snapshots");
    for (field, names) in &mismatches {
        println!("{field}: {} mismatches", names.len());
        for name in names {
            println!("  {name}");
        }
    }
    assert!(
        mismatches.is_empty(),
        "snapshot mismatches; see output above"
    );
}

fn collect(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            collect(&path, out);
        } else if path.extension().is_some_and(|ext| ext == "json") {
            out.push(path);
        }
    }
}

// The two values from just before their first difference.
fn difference(expected: &Value, actual: &Value) -> (String, String) {
    let expected: Vec<char> = expected.to_string().chars().collect();
    let actual: Vec<char> = actual.to_string().chars().collect();
    let first = expected
        .iter()
        .zip(&actual)
        .take_while(|(e, a)| e == a)
        .count();
    let excerpt = |chars: &[char]| {
        let start = first.saturating_sub(80);
        let end = (first + 220).min(chars.len());
        chars[start.min(end)..end].iter().collect::<String>()
    };
    (excerpt(&expected), excerpt(&actual))
}
