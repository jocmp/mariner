//! Load a [`CustomExtractor`] from JSON shaped like upstream's extractor
//! object literals, so extractors can be added without recompiling.
//!
//! ```json
//! {
//!   "domain": "example.com",
//!   "supportedDomains": ["www.example.com"],
//!   "title": { "selectors": ["h1.title"] },
//!   "author": { "selectors": [["meta[name=\"author\"]", "value"]] },
//!   "content": {
//!     "selectors": [[".lede", ".body"], "article"],
//!     "clean": [".ad"],
//!     "transforms": { "h1": "h2" },
//!     "defaultCleaner": false
//!   },
//!   "date_published": { "selectors": ["time"], "timezone": "UTC", "format": "YYYY/M/D" },
//!   "dek": "A hardcoded value",
//!   "extend": { "tags": { "selectors": [".tag"], "allowMultiple": true } }
//! }
//! ```
//!
//! Function-valued transforms can't be expressed in JSON; use the Rust API
//! for those.

use serde_json::{Map, Value};

use super::custom_extractor::{CustomExtractor, Field, Selector};

/// Why an extractor definition couldn't be loaded.
#[derive(Debug, thiserror::Error)]
pub enum ExtractorError {
    #[error("invalid JSON: {0}")]
    Json(#[from] serde_json::Error),
    #[error("{0}")]
    Invalid(String),
}

impl CustomExtractor {
    /// Parse an extractor definition; see the [module docs](self) for the
    /// format.
    pub fn from_json(json: &str) -> Result<Self, ExtractorError> {
        let value: Value = serde_json::from_str(json)?;
        let object = value
            .as_object()
            .ok_or_else(|| invalid("an extractor must be a JSON object"))?;

        let domain = object
            .get("domain")
            .and_then(Value::as_str)
            .filter(|domain| !domain.is_empty())
            .ok_or_else(|| invalid("Unable to add custom extractor. Invalid parameters."))?;

        let mut extractor = CustomExtractor::new(domain);
        if let Some(domains) = object.get("supportedDomains") {
            extractor = extractor.supported_domains(strings(domains, "supportedDomains")?);
        }

        for (key, value) in object {
            let is_content = key == "content";
            let slot = match key.as_str() {
                "title" => &mut extractor.title,
                "author" => &mut extractor.author,
                "content" => &mut extractor.content,
                "date_published" => &mut extractor.date_published,
                "lead_image_url" => &mut extractor.lead_image_url,
                "dek" => &mut extractor.dek,
                "next_page_url" => &mut extractor.next_page_url,
                "excerpt" => &mut extractor.excerpt,
                "extend" => {
                    let fields = value
                        .as_object()
                        .ok_or_else(|| invalid("extend must be an object"))?;
                    for (name, field) in fields {
                        extractor
                            .extend
                            .push((name.clone(), field_from(name, field, false)?));
                    }
                    continue;
                }
                _ => continue,
            };
            if !value.is_null() {
                *slot = Some(field_from(key, value, is_content)?);
            }
        }

        Ok(extractor)
    }
}

fn field_from(name: &str, value: &Value, is_content: bool) -> Result<Field, ExtractorError> {
    let object: &Map<String, Value> = match value {
        Value::String(literal) => return Ok(Field::literal(literal.clone())),
        Value::Object(object) => object,
        _ => return Err(invalid(format!("{name} must be a string or an object"))),
    };

    let mut field = Field::new();

    for selector in object
        .get("selectors")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        field.selectors.push(match selector {
            Value::String(css) => Selector::Css(css.clone()),
            Value::Array(_) if is_content => Selector::Group(strings(selector, name)?),
            Value::Array(_) => match strings(selector, name)?.as_slice() {
                [css, attr] => Selector::Attr {
                    css: css.clone(),
                    attr: attr.clone(),
                    map: None,
                },
                _ => {
                    return Err(invalid(format!(
                        "{name}: attribute selectors are [selector, attribute]"
                    )));
                }
            },
            // Upstream skips anything else, e.g. a null placeholder.
            _ => continue,
        });
    }

    if let Some(clean) = object.get("clean") {
        field = field.clean(strings(clean, name)?);
    }

    if let Some(transforms) = object.get("transforms").and_then(Value::as_object) {
        for (css, transform) in transforms {
            let tag = transform.as_str().ok_or_else(|| {
                invalid(format!(
                    "{name}: transform for {css:?} must be a tag name; function transforms need the Rust API"
                ))
            })?;
            field = field.rename(css.clone(), tag);
        }
    }

    if let Some(enabled) = object.get("defaultCleaner").and_then(Value::as_bool) {
        field = field.default_cleaner(enabled);
    }
    if let Some(enabled) = object.get("allowMultiple").and_then(Value::as_bool) {
        field = field.allow_multiple(enabled);
    }
    if let Some(timezone) = object.get("timezone").and_then(Value::as_str) {
        field = field.timezone(timezone);
    }
    if let Some(format) = object.get("format").and_then(Value::as_str) {
        field = field.format(format);
    }

    Ok(field)
}

fn strings(value: &Value, name: &str) -> Result<Vec<String>, ExtractorError> {
    value
        .as_array()
        .ok_or_else(|| invalid(format!("{name}: expected an array of strings")))?
        .iter()
        .map(|item| {
            item.as_str()
                .map(ToString::to_string)
                .ok_or_else(|| invalid(format!("{name}: expected an array of strings")))
        })
        .collect()
}

fn invalid(message: impl Into<String>) -> ExtractorError {
    ExtractorError::Invalid(message.into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::extractors::Transform;

    #[test]
    fn loads_extractors() {
        let extractor = CustomExtractor::from_json(
            r#"{
                "domain": "example.com",
                "supportedDomains": ["www.example.com"],
                "title": { "selectors": ["h1", ["meta[name=\"og:title\"]", "value"]] },
                "content": { "selectors": [[".a", ".b"]], "clean": [".ad"], "transforms": { "h1": "h2" } },
                "dek": "Hardcoded",
                "extend": { "tags": { "selectors": [".tag"], "allowMultiple": true } }
            }"#,
        )
        .unwrap();

        assert_eq!(
            extractor.domains().collect::<Vec<_>>(),
            ["example.com", "www.example.com"]
        );
        let title = extractor.title.unwrap();
        assert!(matches!(&title.selectors[0], Selector::Css(css) if css == "h1"));
        assert!(matches!(&title.selectors[1], Selector::Attr { attr, .. } if attr == "value"));
        let content = extractor.content.unwrap();
        assert!(matches!(&content.selectors[0], Selector::Group(parts) if parts == &[".a", ".b"]));
        assert!(matches!(&content.transforms[0].1, Transform::Rename(tag) if tag == "h2"));
        assert_eq!(extractor.dek.unwrap().literal.as_deref(), Some("Hardcoded"));
        assert!(extractor.extend[0].1.allow_multiple);
    }

    #[test]
    fn requires_a_domain() {
        assert!(CustomExtractor::from_json(r#"{ "title": "x" }"#).is_err());
    }
}
