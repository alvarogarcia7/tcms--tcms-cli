//! ID resolver (TCMS-42.1): an identifier to system and URL, a URL to system and identifier.
//!
//! Driven entirely by `integration-config.systems[]`; nothing about Jira or StrictDoc
//! deployments is hard-coded. Layer 3: depends on `testcase-models` and `testcase-common`.

pub mod config;
pub mod fill;
pub mod overlap;

use anyhow::{bail, Context, Result};
use regex::Regex;

pub use config::{IntegrationConfig, RequirementSource, SystemConfig, SystemType};
pub use fill::{fill_external_refs, fill_yaml_value};

/// Result of a resolution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Resolved {
    /// `systems[].name`
    pub system: String,
    pub system_type: SystemType,
    pub id: String,
    pub url: String,
}

struct Compiled {
    cfg: SystemConfig,
    id_re: Regex,
    url_re: Option<Regex>,
}

pub struct Resolver {
    systems: Vec<Compiled>,
}

impl Resolver {
    /// Build a resolver. Rejects invalid and overlapping patterns (same checks as config load).
    pub fn new(config: &IntegrationConfig) -> Result<Self> {
        config.check()?;
        let mut systems = Vec::new();
        for s in &config.systems {
            let id_re = Regex::new(&overlap::full_match(&s.id_pattern))
                .with_context(|| format!("system '{}': bad id_pattern", s.name))?;
            let url_re = s
                .url_pattern
                .as_deref()
                .map(Regex::new)
                .transpose()
                .with_context(|| format!("system '{}': bad url_pattern", s.name))?;
            systems.push(Compiled {
                cfg: s.clone(),
                id_re,
                url_re,
            });
        }
        Ok(Self { systems })
    }

    pub fn system_names(&self) -> Vec<&str> {
        self.systems.iter().map(|c| c.cfg.name.as_str()).collect()
    }

    /// Resolve an identifier. Errors when no system, or more than one, claims it.
    pub fn resolve_id(&self, id: &str) -> Result<Resolved> {
        let matching: Vec<&Compiled> = self
            .systems
            .iter()
            .filter(|c| c.id_re.is_match(id))
            .collect();
        match matching.as_slice() {
            [one] => build(one, id),
            [] => bail!(
                "id '{id}' matches no system id_pattern (systems: {})",
                self.systems
                    .iter()
                    .map(|c| format!("{} {}", c.cfg.name, c.cfg.id_pattern))
                    .collect::<Vec<_>>()
                    .join("; ")
            ),
            many => bail!(
                "id '{id}' is ambiguous: it matches systems {}",
                many.iter()
                    .map(|c| c.cfg.name.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        }
    }

    /// Resolve an identifier only among systems of the given family (used for `external_refs`).
    pub fn resolve_id_in_family(
        &self,
        id: &str,
        family: testcase_models::ExternalSystem,
    ) -> Result<Resolved> {
        let matching: Vec<&Compiled> = self
            .systems
            .iter()
            .filter(|c| c.cfg.system_type.family() == family && c.id_re.is_match(id))
            .collect();
        match matching.as_slice() {
            [one] => build(one, id),
            [] => bail!("id '{id}' matches no configured {family:?} system"),
            many => bail!(
                "id '{id}' is ambiguous: it matches systems {}",
                many.iter()
                    .map(|c| c.cfg.name.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        }
    }

    /// Parse a URL back into system and id.
    pub fn resolve_url(&self, url: &str) -> Result<Resolved> {
        let mut found: Vec<Resolved> = Vec::new();
        for c in &self.systems {
            let base = c.cfg.base_url.trim_end_matches('/');
            if !(url == base
                || url.starts_with(&format!("{base}/"))
                || url.starts_with(&format!("{base}#")))
            {
                continue;
            }
            let Some(re) = &c.url_re else { continue };
            let Some(caps) = re.captures(url) else {
                continue;
            };
            let Some(m) = caps.name("id") else { continue };
            let id = m.as_str().to_string();
            if !c.id_re.is_match(&id) {
                continue;
            }
            found.push(build(c, &id)?);
        }
        match found.len() {
            1 => Ok(found.remove(0)),
            0 => bail!("url '{url}' does not belong to any configured system (base_url and url_pattern must match)"),
            _ => bail!(
                "url '{url}' is ambiguous: it matches systems {}",
                found.iter().map(|r| r.system.as_str()).collect::<Vec<_>>().join(", ")
            ),
        }
    }

    /// Accept an id or a URL.
    pub fn resolve(&self, id_or_url: &str) -> Result<Resolved> {
        if id_or_url.contains("://") {
            self.resolve_url(id_or_url)
        } else {
            self.resolve_id(id_or_url)
        }
    }
}

fn build(c: &Compiled, id: &str) -> Result<Resolved> {
    let cfg = &c.cfg;
    let mut url = cfg
        .url_template
        .replace("{base_url}", cfg.base_url.trim_end_matches('/'))
        .replace("{id}", id);
    if url.contains("{doc}") {
        let prefix = id.split(['-', '.']).next().unwrap_or(id);
        let doc = cfg
            .document_map
            .get(prefix)
            .or_else(|| cfg.document_map.get(id))
            .with_context(|| {
                format!(
                    "system '{}': document_map has no entry for '{prefix}' (needed for {{doc}} of id '{id}')",
                    cfg.name
                )
            })?;
        url = url.replace("{doc}", doc);
    }
    Ok(Resolved {
        system: cfg.name.clone(),
        system_type: cfg.system_type,
        id: id.to_string(),
        url,
    })
}
