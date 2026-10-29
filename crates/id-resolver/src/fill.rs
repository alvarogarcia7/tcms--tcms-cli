//! Fill empty `external_refs[].url` from the resolver. Records never store site URLs by hand.

use anyhow::Result;
use serde_yaml::Value;
use testcase_models::ExternalRef;

use crate::Resolver;

/// Fill every ref whose `url` is absent or empty. Returns how many were filled.
/// Refs of a system family the config does not cover (xray, other, ...) are left alone.
pub fn fill_external_refs(resolver: &Resolver, refs: &mut [ExternalRef]) -> Result<usize> {
    let mut n = 0;
    for r in refs {
        if r.url.as_deref().is_some_and(|u| !u.is_empty()) {
            continue;
        }
        if let Ok(res) = resolver.resolve_id_in_family(&r.key, r.system) {
            r.url = Some(res.url);
            n += 1;
        } else if matches!(
            r.system,
            testcase_models::ExternalSystem::Jira | testcase_models::ExternalSystem::Strictdoc
        ) {
            // A jira/strictdoc ref the config cannot resolve is a real problem: report it
            resolver.resolve_id_in_family(&r.key, r.system)?;
        }
    }
    Ok(n)
}

/// Walk a YAML document and fill `external_refs[].url` wherever it is missing or empty.
/// Works for every record type. Returns the number of URLs filled.
pub fn fill_yaml_value(resolver: &Resolver, value: &mut Value) -> Result<usize> {
    let mut n = 0;
    match value {
        Value::Mapping(map) => {
            for (k, v) in map.iter_mut() {
                if k.as_str() == Some("external_refs") {
                    if let Value::Sequence(items) = v {
                        for item in items {
                            n += fill_one(resolver, item)?;
                        }
                        continue;
                    }
                }
                n += fill_yaml_value(resolver, v)?;
            }
        }
        Value::Sequence(items) => {
            for v in items {
                n += fill_yaml_value(resolver, v)?;
            }
        }
        _ => {}
    }
    Ok(n)
}

fn fill_one(resolver: &Resolver, item: &mut Value) -> Result<usize> {
    let Some(map) = item.as_mapping_mut() else {
        return Ok(0);
    };
    let url_key = Value::from("url");
    if map
        .get(&url_key)
        .and_then(Value::as_str)
        .is_some_and(|u| !u.is_empty())
    {
        return Ok(0);
    }
    let mut r: ExternalRef = match serde_yaml::from_value(item.clone()) {
        Ok(r) => r,
        Err(_) => return Ok(0), // not a well-formed ref: leave it for schema validation
    };
    r.url = None;
    if fill_external_refs(resolver, std::slice::from_mut(&mut r))? == 1 {
        item.as_mapping_mut()
            .expect("checked above")
            .insert(url_key, Value::from(r.url.unwrap_or_default()));
        return Ok(1);
    }
    Ok(0)
}
