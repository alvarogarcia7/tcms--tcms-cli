//! Per-product V&V settings (`<DATA_ROOT>/vv-config.yaml`, TCMS-47.2) and parameter resolution.
//!
//! Precedence is flag over file over default. The file is checked here by Rust rules only
//! (types, unknown keys, ranges); validation against a vv-config JSON schema is blocked because
//! no such schema exists in this repository (TCMS-47.2 AC 7, phase 09 CONTEXT Q2).

use std::fs;
use std::path::Path;

use anyhow::{bail, Context as _, Result};
use serde::Deserialize;

pub const FILE_NAME: &str = "vv-config.yaml";
pub const DEFAULT_GATE: u64 = 90;
pub const DEFAULT_FLAKY: u64 = 4;
pub const DEFAULT_WINDOW: u64 = 12;

/// Keys of vv-config.yaml. `type` and `schema` are the metadata every project YAML carries.
#[derive(Debug, Default, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct VvConfig {
    pub gate_threshold_percent: Option<u64>,
    pub flaky_flip_threshold: Option<u64>,
    #[serde(rename = "type")]
    #[allow(dead_code)]
    kind: Option<String>,
    #[allow(dead_code)]
    schema: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    File,
    Flag,
    Default,
}

impl Source {
    pub fn as_str(self) -> &'static str {
        match self {
            Source::File => "file",
            Source::Flag => "flag",
            Source::Default => "default",
        }
    }
}

/// Values given on the command line.
#[derive(Debug, Default, Clone, Copy)]
pub struct Flags {
    pub gate: Option<u64>,
    pub flaky: Option<u64>,
    pub window: Option<u64>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct Resolved {
    pub gate: (u64, Source),
    pub flaky: (u64, Source),
    pub window: (u64, Source),
}

/// Reads `root/vv-config.yaml`; `Ok(None)` when the file does not exist.
pub fn load(root: &Path) -> Result<Option<VvConfig>> {
    let path = root.join(FILE_NAME);
    if !path.is_file() {
        return Ok(None);
    }
    let text = fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
    let cfg: VvConfig = serde_yaml::from_str(&text)
        .with_context(|| format!("invalid {} ({})", FILE_NAME, path.display()))?;
    if let Some(g) = cfg.gate_threshold_percent {
        if !(50..=100).contains(&g) {
            bail!(
                "invalid {} ({}): gate_threshold_percent must be between 50 and 100, got {g}",
                FILE_NAME,
                path.display()
            );
        }
    }
    if cfg.flaky_flip_threshold == Some(0) {
        bail!(
            "invalid {} ({}): flaky_flip_threshold must be at least 1",
            FILE_NAME,
            path.display()
        );
    }
    Ok(Some(cfg))
}

fn pick(flag: Option<u64>, file: Option<u64>, default: u64) -> (u64, Source) {
    match (flag, file) {
        (Some(v), _) => (v, Source::Flag),
        (None, Some(v)) => (v, Source::File),
        (None, None) => (default, Source::Default),
    }
}

/// Flag over file over default. Flag values are range checked here, as the CLI flags document.
pub fn resolve(cfg: Option<&VvConfig>, flags: Flags) -> Result<Resolved> {
    if let Some(g) = flags.gate {
        if !(50..=100).contains(&g) {
            bail!("--gate must be between 50 and 100, got {g}");
        }
    }
    if flags.flaky == Some(0) {
        bail!("--flaky-threshold must be at least 1");
    }
    if flags.window == Some(0) {
        bail!("--stability-window must be at least 1");
    }
    Ok(Resolved {
        gate: pick(
            flags.gate,
            cfg.and_then(|c| c.gate_threshold_percent),
            DEFAULT_GATE,
        ),
        flaky: pick(
            flags.flaky,
            cfg.and_then(|c| c.flaky_flip_threshold),
            DEFAULT_FLAKY,
        ),
        window: pick(flags.window, None, DEFAULT_WINDOW),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(dir: &Path, body: &str) {
        fs::write(dir.join(FILE_NAME), body).unwrap();
    }

    #[test]
    fn missing_file_returns_none() {
        let d = tempfile::tempdir().unwrap();
        assert!(load(d.path()).unwrap().is_none());
    }

    #[test]
    fn valid_file_loads_with_metadata_keys() {
        let d = tempfile::tempdir().unwrap();
        write(
            d.path(),
            "type: vv-config\nschema: tcms/vv-config.schema.v1.json\ngate_threshold_percent: 95\nflaky_flip_threshold: 3\n",
        );
        let c = load(d.path()).unwrap().unwrap();
        assert_eq!(c.gate_threshold_percent, Some(95));
        assert_eq!(c.flaky_flip_threshold, Some(3));
    }

    fn err_of(body: &str) -> String {
        let d = tempfile::tempdir().unwrap();
        write(d.path(), body);
        format!("{:#}", load(d.path()).unwrap_err())
    }

    #[test]
    fn errors_name_file_and_key() {
        for (body, key) in [
            ("gate_threshold_percent: high\n", "gate_threshold_percent"),
            ("gate_threshold_percent: 49\n", "gate_threshold_percent"),
            ("gate_threshold_percent: 101\n", "gate_threshold_percent"),
            ("flaky_flip_threshold: 0\n", "flaky_flip_threshold"),
            ("flaky_flip_threshold: x\n", "flaky_flip_threshold"),
            ("stability_window: 5\n", "stability_window"),
        ] {
            let e = err_of(body);
            assert!(e.contains(FILE_NAME), "{e}");
            assert!(e.contains(key), "{e}");
        }
    }

    #[test]
    fn precedence_flag_over_file_over_default() {
        let cfg = VvConfig {
            gate_threshold_percent: Some(95),
            flaky_flip_threshold: Some(3),
            ..Default::default()
        };
        let r = resolve(Some(&cfg), Flags::default()).unwrap();
        assert_eq!(r.gate, (95, Source::File));
        assert_eq!(r.flaky, (3, Source::File));
        assert_eq!(r.window, (12, Source::Default));
        let r = resolve(
            Some(&cfg),
            Flags {
                gate: Some(80),
                flaky: Some(6),
                window: Some(5),
            },
        )
        .unwrap();
        assert_eq!(r.gate, (80, Source::Flag));
        assert_eq!(r.flaky, (6, Source::Flag));
        assert_eq!(r.window, (5, Source::Flag));
        let r = resolve(None, Flags::default()).unwrap();
        assert_eq!(r.gate, (90, Source::Default));
        assert_eq!(r.flaky, (4, Source::Default));
    }

    #[test]
    fn out_of_range_flags_are_refused() {
        let f = |gate, flaky, window| {
            resolve(
                None,
                Flags {
                    gate,
                    flaky,
                    window,
                },
            )
        };
        assert!(f(Some(49), None, None).is_err());
        assert!(f(None, Some(0), None).is_err());
        assert!(f(None, None, Some(0)).is_err());
    }
}
