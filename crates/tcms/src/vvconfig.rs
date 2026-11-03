//! Per-product V&V settings (TCMS-47.2): `<DATA_ROOT>/vv-config.yaml` and the command-line flags that
//! override it. Each parameter is resolved as flag, then file, then default, and keeps the source it
//! came from, so the output can say where a value came from.

use std::fs;
use std::path::Path;

use anyhow::{bail, Context as _, Result};

/// Name of the settings file under the data root.
pub const FILE_NAME: &str = "vv-config.yaml";
/// Gate threshold when neither the flag nor the file sets it (D-89 range 50 to 100).
pub const DEFAULT_GATE: u64 = 90;
/// Flaky threshold when neither the flag nor the file sets it (D-97).
pub const DEFAULT_FLAKY: u64 = 4;
/// Stability window in runs when the flag is not given (TCMS-47.2).
pub const DEFAULT_WINDOW: u64 = 12;

const KEY_GATE: &str = "gate_threshold_percent";
const KEY_FLAKY: &str = "flaky_flip_threshold";
const KEY_WINDOW: &str = "stability_window_runs";
/// Metadata every project YAML carries (CLAUDE.md, "Required YAML Fields"). Accepted, not used.
const METADATA_KEYS: [&str; 2] = ["type", "schema"];

/// The values a `vv-config.yaml` file sets. A key that is absent is `None`.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct VvConfig {
    pub gate_threshold_percent: Option<u64>,
    pub flaky_flip_threshold: Option<u64>,
    pub stability_window_runs: Option<u64>,
}

/// Where a parameter value came from.
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

/// A parameter value and its source.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Param {
    pub value: u64,
    pub source: Source,
}

/// The command-line flags that can override the file. `None` means the flag was not given.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Flags {
    pub gate: Option<u64>,
    pub flaky: Option<u64>,
    pub window: Option<u64>,
}

/// The three parameters of `tcms vv metrics`, each with its source.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Resolved {
    pub gate: Param,
    pub flaky: Param,
    pub window: Param,
}

/// Read `<root>/vv-config.yaml`. `None` when the file does not exist.
pub fn load(root: &Path) -> Result<Option<VvConfig>> {
    let path = root.join(FILE_NAME);
    if !path.is_file() {
        return Ok(None);
    }
    let text =
        fs::read_to_string(&path).with_context(|| format!("cannot read {}", path.display()))?;
    parse(&path, &text).map(Some)
}

/// Parse and check the settings file. Every error names the file and, where there is one, the key.
pub fn parse(path: &Path, text: &str) -> Result<VvConfig> {
    let shown = path.display();
    let doc: serde_yaml::Value =
        serde_yaml::from_str(text).with_context(|| format!("{shown}: not valid YAML"))?;
    if doc.is_null() {
        return Ok(VvConfig::default());
    }
    let Some(map) = doc.as_mapping() else {
        bail!("{shown}: the file must be a mapping of keys to values");
    };
    let mut cfg = VvConfig::default();
    for (key, value) in map {
        let Some(name) = key.as_str() else {
            bail!("{shown}: every key must be a string");
        };
        match name {
            KEY_GATE => {
                cfg.gate_threshold_percent =
                    Some(integer(path, name, value, 50, 100, "between 50 and 100")?);
            }
            KEY_FLAKY => {
                cfg.flaky_flip_threshold =
                    Some(integer(path, name, value, 1, i64::MAX, "at least 1")?);
            }
            KEY_WINDOW => {
                cfg.stability_window_runs =
                    Some(integer(path, name, value, 1, i64::MAX, "at least 1")?);
            }
            known if METADATA_KEYS.contains(&known) => {}
            other => bail!(
                "{shown}: unknown key '{other}' (allowed: {KEY_GATE}, {KEY_FLAKY}, {KEY_WINDOW}, type, schema)"
            ),
        }
    }
    Ok(cfg)
}

/// An integer in `min..=max`, named in `hint` when it is out of range.
fn integer(
    path: &Path,
    key: &str,
    value: &serde_yaml::Value,
    min: i64,
    max: i64,
    hint: &str,
) -> Result<u64> {
    let shown = path.display();
    let Some(n) = value.as_i64() else {
        bail!("{shown}: {key} must be an integer ({hint})");
    };
    if n < min || n > max {
        bail!("{shown}: {key} must be {hint}, got {n}");
    }
    // `n >= min >= 0` for every key here, so the conversion cannot fail.
    Ok(n as u64)
}

/// Resolve the three parameters: flag over file over default. Flags are range-checked here, so a bad
/// flag fails whether or not the file exists. The window is also accepted in the file (D-47-4).
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
    let file = cfg.copied().unwrap_or_default();
    Ok(Resolved {
        gate: pick(flags.gate, file.gate_threshold_percent, DEFAULT_GATE),
        flaky: pick(flags.flaky, file.flaky_flip_threshold, DEFAULT_FLAKY),
        window: pick(flags.window, file.stability_window_runs, DEFAULT_WINDOW),
    })
}

fn pick(flag: Option<u64>, file: Option<u64>, default: u64) -> Param {
    if let Some(value) = flag {
        Param {
            value,
            source: Source::Flag,
        }
    } else if let Some(value) = file {
        Param {
            value,
            source: Source::File,
        }
    } else {
        Param {
            value: default,
            source: Source::Default,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_text(text: &str) -> Result<VvConfig> {
        parse(Path::new("data/vv-config.yaml"), text)
    }

    fn error_of(text: &str) -> String {
        format!("{:#}", parse_text(text).unwrap_err())
    }

    #[test]
    fn missing_file_is_none() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(load(dir.path()).unwrap(), None);
    }

    #[test]
    fn present_file_is_read() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(
            dir.path().join(FILE_NAME),
            "type: vv-config\nschema: tcms/vv-config.schema.v1.json\ngate_threshold_percent: 95\nflaky_flip_threshold: 3\n",
        )
        .unwrap();
        let cfg = load(dir.path()).unwrap().unwrap();
        assert_eq!(cfg.gate_threshold_percent, Some(95));
        assert_eq!(cfg.flaky_flip_threshold, Some(3));
    }

    #[test]
    fn window_below_one_is_rejected_naming_file_and_key() {
        let err = error_of("stability_window_runs: 0\n");
        assert!(err.contains("data/vv-config.yaml"), "{err}");
        assert!(err.contains("stability_window_runs"), "{err}");
        assert!(err.contains("at least 1"), "{err}");
    }

    #[test]
    fn window_key_is_read() {
        let cfg = parse_text("stability_window_runs: 14\n").unwrap();
        assert_eq!(cfg.stability_window_runs, Some(14));
    }

    #[test]
    fn empty_file_sets_nothing() {
        assert_eq!(parse_text("").unwrap(), VvConfig::default());
    }

    #[test]
    fn wrong_type_names_file_and_key() {
        let err = error_of("gate_threshold_percent: \"95\"\n");
        assert!(err.contains("data/vv-config.yaml"), "{err}");
        assert!(err.contains("gate_threshold_percent"), "{err}");
        let err = error_of("flaky_flip_threshold: 2.5\n");
        assert!(err.contains("flaky_flip_threshold"), "{err}");
        let err = error_of("stability_window_runs: \"12\"\n");
        assert!(err.contains("data/vv-config.yaml"), "{err}");
        assert!(err.contains("stability_window_runs"), "{err}");
        assert!(err.contains("must be an integer"), "{err}");
    }

    #[test]
    fn unknown_key_names_file_and_key() {
        let err = error_of("gate_threshold_pct: 95\n");
        assert!(err.contains("data/vv-config.yaml"), "{err}");
        assert!(err.contains("gate_threshold_pct"), "{err}");
    }

    #[test]
    fn out_of_range_names_file_and_key() {
        for text in [
            "gate_threshold_percent: 49\n",
            "gate_threshold_percent: 101\n",
            "flaky_flip_threshold: 0\n",
            "flaky_flip_threshold: -1\n",
        ] {
            let err = error_of(text);
            assert!(err.contains("data/vv-config.yaml"), "{err}");
            let key = text.split(':').next().unwrap();
            assert!(err.contains(key), "{err}");
        }
    }

    #[test]
    fn not_a_mapping_is_an_error() {
        let err = error_of("- 95\n");
        assert!(err.contains("data/vv-config.yaml"), "{err}");
    }

    #[test]
    fn no_file_and_no_flags_gives_defaults() {
        let r = resolve(None, Flags::default()).unwrap();
        assert_eq!(
            r.gate,
            Param {
                value: 90,
                source: Source::Default
            }
        );
        assert_eq!(
            r.flaky,
            Param {
                value: 4,
                source: Source::Default
            }
        );
        assert_eq!(
            r.window,
            Param {
                value: 12,
                source: Source::Default
            }
        );
    }

    #[test]
    fn file_beats_default() {
        let cfg = VvConfig {
            gate_threshold_percent: Some(95),
            flaky_flip_threshold: Some(3),
            stability_window_runs: Some(14),
        };
        let r = resolve(Some(&cfg), Flags::default()).unwrap();
        assert_eq!(
            r.gate,
            Param {
                value: 95,
                source: Source::File
            }
        );
        assert_eq!(
            r.flaky,
            Param {
                value: 3,
                source: Source::File
            }
        );
        assert_eq!(
            r.window,
            Param {
                value: 14,
                source: Source::File
            }
        );
    }

    #[test]
    fn flag_beats_file_and_default() {
        let cfg = VvConfig {
            gate_threshold_percent: Some(95),
            flaky_flip_threshold: Some(3),
            stability_window_runs: Some(14),
        };
        let flags = Flags {
            gate: Some(80),
            flaky: Some(6),
            window: Some(5),
        };
        let r = resolve(Some(&cfg), flags).unwrap();
        assert_eq!(
            r.gate,
            Param {
                value: 80,
                source: Source::Flag
            }
        );
        assert_eq!(
            r.flaky,
            Param {
                value: 6,
                source: Source::Flag
            }
        );
        assert_eq!(
            r.window,
            Param {
                value: 5,
                source: Source::Flag
            }
        );
    }

    #[test]
    fn bad_flags_are_refused_even_without_a_file() {
        let err = resolve(
            None,
            Flags {
                gate: Some(49),
                ..Flags::default()
            },
        )
        .unwrap_err();
        assert!(err
            .to_string()
            .contains("--gate must be between 50 and 100"));
        let err = resolve(
            None,
            Flags {
                flaky: Some(0),
                ..Flags::default()
            },
        )
        .unwrap_err();
        assert!(err
            .to_string()
            .contains("--flaky-threshold must be at least 1"));
        let err = resolve(
            None,
            Flags {
                window: Some(0),
                ..Flags::default()
            },
        )
        .unwrap_err();
        assert!(err
            .to_string()
            .contains("--stability-window must be at least 1"));
    }
}
