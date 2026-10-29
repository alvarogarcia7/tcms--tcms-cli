//! YAML output that other YAML readers read back unchanged.
//!
//! `serde_yaml` 0.9 writes enum variants as YAML tags (`!Pass`) and leaves strings such
//! as `2026-10-28` unquoted. The JSON schemas in tcms-schemas describe enums as
//! single-key maps (`Pass: {...}`) and strings as strings, and their validator reads the
//! file with PyYAML (YAML 1.1), which rejects the tags and turns the date into a date
//! object. `to_yaml_string` serializes in the form the schemas expect.

use anyhow::{Context, Result};
use regex::Regex;
use serde::Serialize;
use serde_yaml::{Mapping, Value};
use std::sync::OnceLock;

/// Prefixed to strings that must be quoted. It contains NUL, so `serde_yaml` always
/// writes the string double-quoted and the escaped form below is easy to find and drop.
const MARKER: &str = "\u{0}tcms-quote\u{0}";
const MARKER_ESCAPED: &str = "\\0tcms-quote\\0";

/// Scalars that a YAML 1.1 reader (PyYAML) resolves to something other than a string:
/// booleans, nulls, ints, floats and timestamps.
fn yaml11_non_string() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(
            r"^(?:yes|Yes|YES|no|No|NO|true|True|TRUE|false|False|FALSE|on|On|ON|off|Off|OFF|~|null|Null|NULL|<<|=|[-+]?0b[0-1_]+|[-+]?0[0-7_]+|[-+]?(?:0|[1-9][0-9_]*)|[-+]?0x[0-9a-fA-F_]+|[-+]?[1-9][0-9_]*(?::[0-5]?[0-9])+|[-+]?(?:[0-9][0-9_]*)\.[0-9_]*(?:[eE][-+][0-9]+)?|\.[0-9][0-9_]*(?:[eE][-+][0-9]+)?|[-+]?[0-9][0-9_]*(?::[0-5]?[0-9])+\.[0-9_]*|[-+]?\.(?:inf|Inf|INF)|\.(?:nan|NaN|NAN)|[0-9]{4}-[0-9]{2}-[0-9]{2}|[0-9]{4}-[0-9]{1,2}-[0-9]{1,2}(?:[Tt]|[ \t]+)[0-9]{1,2}:[0-9]{2}:[0-9]{2}(?:\.[0-9]*)?(?:[ \t]*(?:Z|[-+][0-9]{1,2}(?::[0-9]{2})?))?)$",
        )
        .expect("valid YAML 1.1 scalar regex")
    })
}

/// True if `serde_yaml` would write `s` unquoted although YAML 1.1 reads it as a non-string.
fn needs_quotes(s: &str) -> bool {
    if !yaml11_non_string().is_match(s) {
        return false;
    }
    // serde_yaml already quotes what YAML 1.2 reads as a number, bool or null
    serde_yaml::to_string(s)
        .map(|y| !(y.starts_with('\'') || y.starts_with('"')))
        .unwrap_or(false)
}

/// Replace enum tags (`!Pass {...}`) with single-key maps (`Pass: {...}`), recursively.
/// Used on values read from YAML written by older versions of the tools.
pub fn untag(value: Value) -> Value {
    convert(value, false)
}

fn prepare(value: Value) -> Value {
    convert(value, true)
}

fn convert(value: Value, quote_ambiguous: bool) -> Value {
    match value {
        Value::Tagged(tagged) => {
            let mut map = Mapping::new();
            let name = tagged.tag.to_string();
            map.insert(
                Value::String(name.trim_start_matches('!').to_string()),
                convert(tagged.value, quote_ambiguous),
            );
            Value::Mapping(map)
        }
        Value::Mapping(map) => Value::Mapping(
            map.into_iter()
                .map(|(k, v)| (convert(k, quote_ambiguous), convert(v, quote_ambiguous)))
                .collect(),
        ),
        Value::Sequence(seq) => Value::Sequence(
            seq.into_iter()
                .map(|v| convert(v, quote_ambiguous))
                .collect(),
        ),
        Value::String(s) if quote_ambiguous && needs_quotes(&s) => {
            Value::String(format!("{MARKER}{s}"))
        }
        other => other,
    }
}

/// Serialize `value` to YAML that tcms-schemas and PyYAML read back unchanged:
/// enum variants become single-key maps and date-like strings stay quoted strings.
pub fn to_yaml_string<T: Serialize + ?Sized>(value: &T) -> Result<String> {
    let value = serde_yaml::to_value(value).context("Failed to convert value to YAML")?;
    let yaml = serde_yaml::to_string(&prepare(value)).context("Failed to serialize YAML")?;
    Ok(yaml.replace(MARKER_ESCAPED, ""))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;

    #[derive(Serialize, Deserialize, Debug, PartialEq)]
    enum Step {
        Pass { step: i64 },
        Done,
    }

    #[derive(Serialize)]
    struct Doc {
        steps: Vec<Step>,
        output: String,
    }

    #[test]
    fn enum_variants_are_single_key_maps_not_tags() {
        let doc = Doc {
            steps: vec![Step::Pass { step: 1 }, Step::Done],
            output: "plain".to_string(),
        };
        let yaml = to_yaml_string(&doc).unwrap();
        assert!(!yaml.contains('!'), "tags must not be written: {yaml}");
        let back: Value = serde_yaml::from_str(&yaml).unwrap();
        assert_eq!(back["steps"][0]["Pass"]["step"], Value::from(1));
        assert_eq!(back["steps"][1], Value::from("Done"));
    }

    #[test]
    fn date_like_strings_are_quoted() {
        for s in [
            "2026-10-28",
            "2026-10-28T12:39:09Z",
            "2026-10-28 12:39:09",
            "yes",
            "No",
            "on",
            "0755",
            "1_000",
            "1:30",
            "0x1F",
            "1e3",
            "1.5e+3",
            ".inf",
        ] {
            let doc = Doc {
                steps: vec![],
                output: s.to_string(),
            };
            let yaml = to_yaml_string(&doc).unwrap();
            assert!(!yaml.contains("tcms-quote"), "marker leaked: {yaml}");
            assert!(
                yaml.contains(&format!("output: \"{s}\""))
                    || yaml.contains(&format!("output: '{s}'")),
                "{s:?} must be quoted: {yaml}"
            );
            let back: Value = serde_yaml::from_str(&yaml).unwrap();
            assert_eq!(back["output"], Value::from(s));
        }
    }

    #[test]
    fn ordinary_strings_stay_plain() {
        let doc = Doc {
            steps: vec![],
            output: "hello world 2026".to_string(),
        };
        assert!(to_yaml_string(&doc)
            .unwrap()
            .contains("output: hello world 2026"));
    }
}
