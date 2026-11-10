//! Compares parser output with reference output for every fixture under
//! `tests/fixtures/`. Reference files live in `outputs/feeds-reference/` at
//! the workspace root (never tracked), one `<fixture>.json` per fixture:
//! `{ "fixture_sha256", "base_url", "document" }`, where `base_url` is the
//! URL the feed is treated as subscribed from and each entry carries its
//! `public_id` for that URL. The parser's own output is written to
//! `outputs/feeds-actual/` for diffing. Fields the reference doesn't read
//! (see `drop_unreferenced`) aren't compared.
//!
//! Without reference output, the test skips. `MARINER_PARITY_ONLY=<prefix>`
//! narrows the run to fixtures whose name starts with the prefix.

use std::path::{Path, PathBuf};

use serde_json::Value;
use sha2::{Digest, Sha256};

#[test]
fn matches_reference_output() {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let fixtures = manifest.join("tests/fixtures");
    let outputs = manifest.join("../../outputs");
    let references = outputs.join("feeds-reference");
    if !references.is_dir() {
        println!("skipped: {} not found", references.display());
        return;
    }
    let only = std::env::var("MARINER_PARITY_ONLY").unwrap_or_default();

    let mut fixture_paths = Vec::new();
    collect_feeds(&fixtures, &mut fixture_paths);
    fixture_paths.sort();

    let (mut matched, mut skipped) = (0, 0);
    let mut failures = Vec::new();
    for path in fixture_paths {
        let name = path
            .strip_prefix(&fixtures)
            .unwrap()
            .with_extension("")
            .to_string_lossy()
            .into_owned();
        if !name.starts_with(&only) {
            continue;
        }
        let Ok(reference) = std::fs::read_to_string(references.join(format!("{name}.json"))) else {
            skipped += 1;
            continue;
        };
        let reference: Value = serde_json::from_str(&reference).unwrap();
        let bytes = std::fs::read(&path).unwrap();

        if reference["fixture_sha256"].as_str() != Some(sha256(&bytes).as_str()) {
            failures.push(format!(
                "{name}: fixture changed since its reference output was written; regenerate it"
            ));
            continue;
        }
        if reference.get("error").is_some() {
            skipped += 1;
            continue;
        }

        let base_url = reference["base_url"].as_str().unwrap_or_default();
        let actual = match mariner_feeds::parse(&bytes) {
            Ok(feed) => document(&feed, base_url),
            Err(error) => {
                failures.push(format!("{name}: {error}"));
                continue;
            }
        };
        write_actual(&outputs, &name, &actual);
        let mut actual = actual;
        drop_unreferenced(&reference["document"], &mut actual);

        match first_difference(&reference["document"], &actual, "$") {
            None => matched += 1,
            Some(difference) => failures.push(format!("{name}: {difference}")),
        }
    }

    println!(
        "{matched} matched, {} differ, {skipped} skipped (no reference output)",
        failures.len()
    );
    assert!(failures.is_empty(), "\n{}", failures.join("\n"));
}

/// The feed as JSON, with each entry's `public_id` for `base_url`.
fn document(feed: &mariner_feeds::Feed, base_url: &str) -> Value {
    let mut value = serde_json::to_value(feed).unwrap();
    let entries = value["entries"].as_array_mut().unwrap();
    for (entry, json) in feed.entries.iter().zip(entries) {
        json["public_id"] = Value::String(entry.public_id(base_url));
    }
    value
}

/// Removes what the parser reads beyond the reference: every entry's
/// `enclosures`, and `media` wherever the reference has none (it reads
/// Media RSS only for YouTube entries).
fn drop_unreferenced(expected: &Value, actual: &mut Value) {
    let empty = Vec::new();
    let expected_entries = expected["entries"].as_array().unwrap_or(&empty);
    let Some(entries) = actual["entries"].as_array_mut() else {
        return;
    };
    for (index, entry) in entries.iter_mut().enumerate() {
        let Some(entry) = entry.as_object_mut() else {
            continue;
        };
        entry.remove("enclosures");
        if expected_entries
            .get(index)
            .is_none_or(|e| e["media"].is_null())
        {
            entry.remove("media");
        }
    }
}

/// Writes the parser's output under `outputs/feeds-actual/` for diffing.
fn write_actual(outputs: &Path, name: &str, actual: &Value) {
    let path = outputs.join("feeds-actual").join(format!("{name}.json"));
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    std::fs::write(path, serde_json::to_string_pretty(actual).unwrap() + "\n").unwrap();
}

fn collect_feeds(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            collect_feeds(&path, out);
        } else if path
            .extension()
            .is_some_and(|ext| ext == "xml" || ext == "json")
        {
            out.push(path);
        }
    }
}

fn sha256(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

/// The JSON path of the first difference, with both values.
fn first_difference(expected: &Value, actual: &Value, path: &str) -> Option<String> {
    match (expected, actual) {
        (Value::Object(e), Value::Object(a)) => {
            let mut keys: Vec<&String> = e.keys().chain(a.keys()).collect();
            keys.sort();
            keys.dedup();
            keys.into_iter().find_map(|key| {
                let e = e.get(key).unwrap_or(&Value::Null);
                let a = a.get(key).unwrap_or(&Value::Null);
                first_difference(e, a, &format!("{path}.{key}"))
            })
        }
        (Value::Array(e), Value::Array(a)) => {
            let found = e
                .iter()
                .zip(a)
                .enumerate()
                .find_map(|(i, (e, a))| first_difference(e, a, &format!("{path}[{i}]")));
            found.or_else(|| {
                (e.len() != a.len())
                    .then(|| format!("{path}: expected {} items, got {}", e.len(), a.len()))
            })
        }
        (e, a) if e == a => None,
        (e, a) => Some(format!("{path}: expected {}, got {}", short(e), short(a))),
    }
}

fn short(value: &Value) -> String {
    let text = value.to_string();
    if text.chars().count() > 160 {
        format!("{}…", text.chars().take(160).collect::<String>())
    } else {
        text
    }
}
