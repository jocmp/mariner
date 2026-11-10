//! Compares condense output with reference output for every fixture under
//! `tests/fixtures/`. Reference files live in `outputs/condense-reference/`
//! at the workspace root (never tracked), one `<fixture>.json` per fixture:
//! `{ "fixture_sha256", "base_url", "document" }`. Condense's own output is
//! written to `outputs/condense-actual/` for diffing.
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
    let references = outputs.join("condense-reference");
    if !references.is_dir() {
        println!("skipped: {} not found", references.display());
        return;
    }
    let only = std::env::var("MARINER_PARITY_ONLY").unwrap_or_default();

    let mut fixture_paths = Vec::new();
    collect_html(&fixtures, &mut fixture_paths);
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

        let base_url = reference["base_url"].as_str().unwrap_or_default();
        let html = String::from_utf8_lossy(&bytes);
        let document = mariner_condense::condense(&html, base_url).unwrap();
        let mut actual = serde_json::to_value(&document).unwrap();
        let mut expected = reference["document"].clone();
        normalize(&mut actual);
        normalize(&mut expected);
        write_actual(&outputs, &name, &actual);

        match first_difference(&expected, &actual, "$") {
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

/// Writes condense's output under `outputs/condense-actual/` for diffing.
fn write_actual(outputs: &Path, name: &str, actual: &Value) {
    let path = outputs.join("condense-actual").join(format!("{name}.json"));
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    std::fs::write(path, serde_json::to_string_pretty(actual).unwrap() + "\n").unwrap();
}

fn collect_html(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            collect_html(&path, out);
        } else if path.extension().is_some_and(|ext| ext == "html") {
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

/// Puts unordered collections in a canonical order (`spans` by start, then
/// longest first, then style) and drops `ids` and `anchors`: where an id
/// lands follows the spec, which `tests/condense.rs` checks, and the
/// reference places container ids differently.
fn normalize(value: &mut Value) {
    match value {
        Value::Object(map) => {
            map.remove("ids");
            map.remove("anchors");
            if let Some(Value::Array(spans)) = map.get_mut("spans") {
                spans.sort_by_key(|span| {
                    (
                        span["start"].as_u64().unwrap_or_default(),
                        std::cmp::Reverse(span["end"].as_u64().unwrap_or_default()),
                        span.to_string(),
                    )
                });
            }
            map.values_mut().for_each(normalize);
        }
        Value::Array(items) => items.iter_mut().for_each(normalize),
        _ => {}
    }
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
        (e, a) if numbers_equal(e, a) || e == a => None,
        (e, a) => Some(format!("{path}: expected {}, got {}", short(e), short(a))),
    }
}

fn numbers_equal(expected: &Value, actual: &Value) -> bool {
    match (expected.as_f64(), actual.as_f64()) {
        (Some(e), Some(a)) => (e - a).abs() < 1e-6,
        _ => false,
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
