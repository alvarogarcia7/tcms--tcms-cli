//! Targeted edit of the `external_refs` block of a record file (TCMS-42.9, worker C4).
//!
//! `tcms jira sync` stores the key of the entity it found or created (a test case's Jira or
//! Xray entity, a plan's Fix Version id) in the record's `external_refs`. Rewriting the whole
//! record with serde would lose comments, quoting and layout, so only the lines of the
//! `external_refs` block are replaced or added. Every other byte of the file stays the same.
//!
//! Decisions (D-42.4i .. D-42.4k):
//! * An entry is identified by its `system`. The same system and key again: no change at all
//!   (the file is not touched, so a re-sync is idempotent). A different key for the same system:
//!   that entry is replaced. No entry for the system: one is added at the end of the block.
//! * `synced_at` is set when the entry is written, not on every sync, so that a re-sync does not
//!   change the file.
//! * Forms this module does not edit (a non-empty flow list) are refused with an error, so the
//!   record is never corrupted; `external_refs: []` is rewritten as a block.
//! * The edited text is checked before it is returned: it must parse, the new entry must be in
//!   `external_refs`, and every other top-level key must be equal to the original.

use anyhow::{bail, Context as _, Result};
use serde_yaml::Value;

/// The key line `external_refs:` at column 0 (the block form, or `[]` for the empty list).
const KEY: &str = "external_refs:";

/// Store `key` as the entry of `system` in the `external_refs` of `text`.
///
/// Returns `Ok(None)` when the text already holds this system and key, and `Ok(Some(new_text))`
/// when it must change.
pub fn set_external_ref(
    text: &str,
    system: &str,
    key: &str,
    synced_at: &str,
) -> Result<Option<String>> {
    let original: Value = serde_yaml::from_str(text).context("record does not parse as YAML")?;
    let lines: Vec<String> = text.split_inclusive('\n').map(str::to_string).collect();
    let edited = edit_lines(&lines, system, key, synced_at)?;
    let Some(new_text) = edited else {
        return Ok(None);
    };
    check_edit(&original, &new_text, system, key)?;
    Ok(Some(new_text))
}

fn edit_lines(
    lines: &[String],
    system: &str,
    key: &str,
    synced_at: &str,
) -> Result<Option<String>> {
    let Some(i) = lines.iter().position(|l| content(l).starts_with(KEY)) else {
        // no block yet: append one at the end of the file
        let mut out: String = lines.concat();
        if !out.is_empty() && !out.ends_with('\n') {
            out.push('\n');
        }
        out.push_str(KEY);
        out.push('\n');
        out.push_str(&entry(2, system, key, synced_at));
        return Ok(Some(out));
    };

    let rest = content(&lines[i])[KEY.len()..].trim();
    let mut out: Vec<String> = lines.to_vec();
    let (block_start, default_indent) = match rest {
        "" => (i + 1, 2),
        r if r.starts_with('#') => (i + 1, 2),
        "[]" => {
            // rewrite the empty flow list as a block, then append to it
            out[i] = format!("{KEY}\n");
            (i + 1, 2)
        }
        _ => bail!(
            "external_refs is a non-empty flow list; edit it by hand (only the block form is \
             rewritten by tcms)"
        ),
    };

    // the block: lines after the key that are blank, indented, or a column-0 dash
    let mut end = block_start;
    let mut j = block_start;
    while j < out.len() {
        let c = content(&out[j]);
        if c.trim().is_empty() {
            j += 1;
            continue;
        }
        let ind = indent(c);
        if ind == 0 && !c.starts_with('-') {
            break;
        }
        j += 1;
        end = j;
    }

    // entries: the item lines at the indentation of the first one
    let item_ind = (block_start..end)
        .find(|&k| is_item(content(&out[k])))
        .map(|k| indent(content(&out[k])));
    let item_at: Vec<usize> = match item_ind {
        Some(ind) => (block_start..end)
            .filter(|&k| is_item(content(&out[k])) && indent(content(&out[k])) == ind)
            .collect(),
        None => Vec::new(),
    };
    let pad = item_ind.unwrap_or(default_indent);

    for (n, &start) in item_at.iter().enumerate() {
        let stop = item_at.get(n + 1).copied().unwrap_or(end);
        let entry_lines = &out[start..stop];
        let fields: Vec<(String, String)> = entry_lines
            .iter()
            .filter_map(|l| field(content(l)))
            .collect();
        let has = |name: &str, value: &str| fields.iter().any(|(k, v)| k == name && v == value);
        if has("system", system) && has("key", key) {
            return Ok(None);
        }
    }

    // the first entry of this system is replaced; without one, the entry is added at the end
    let same_system = item_at.iter().enumerate().find_map(|(n, &start)| {
        let stop = item_at.get(n + 1).copied().unwrap_or(end);
        let sys_here = out[start..stop]
            .iter()
            .filter_map(|l| field(content(l)))
            .any(|(k, v)| k == "system" && v == system);
        sys_here.then_some((start, stop))
    });
    match same_system {
        Some((start, stop)) => {
            // keep comment and blank lines that follow the entry: the entry ends at its last
            // content line
            let last = (start..stop)
                .rev()
                .find(|&k| !content(&out[k]).trim().is_empty() && !is_comment(&out[k]))
                .unwrap_or(start)
                + 1;
            let mut new_out: Vec<String> = out[..start].to_vec();
            new_out.push(entry(pad, system, key, synced_at));
            new_out.extend_from_slice(&out[last..]);
            Ok(Some(new_out.concat()))
        }
        None => {
            // after the last non-blank line of the block
            let at = end;
            if at > 0 && !out[at - 1].ends_with('\n') {
                out[at - 1].push('\n');
            }
            let mut new_out: Vec<String> = out[..at].to_vec();
            new_out.push(entry(pad, system, key, synced_at));
            new_out.extend_from_slice(&out[at..]);
            Ok(Some(new_out.concat()))
        }
    }
}

/// One entry in the block form, `pad` spaces in (the dash sits at column `pad`).
fn entry(pad: usize, system: &str, key: &str, synced_at: &str) -> String {
    let p = " ".repeat(pad);
    format!(
        "{p}- system: {}\n{p}  key: {}\n{p}  synced_at: \"{synced_at}\"\n",
        scalar(system),
        scalar(key)
    )
}

/// A plain or quoted YAML scalar for `s`, as serde_yaml writes it.
fn scalar(s: &str) -> String {
    serde_yaml::to_string(&Value::String(s.to_string()))
        .map(|t| t.trim_end().to_string())
        .unwrap_or_else(|_| format!("'{s}'"))
}

/// The line without its line break.
fn content(line: &str) -> &str {
    line.trim_end_matches(['\n', '\r'])
}

fn indent(line: &str) -> usize {
    line.len() - line.trim_start_matches(' ').len()
}

fn is_item(line: &str) -> bool {
    let t = line.trim_start();
    t == "-" || t.starts_with("- ")
}

fn is_comment(line: &str) -> bool {
    line.trim_start().starts_with('#')
}

/// `name: value` of an entry line (a leading dash is skipped; quotes are removed).
fn field(line: &str) -> Option<(String, String)> {
    let t = line.trim_start();
    let t = t.strip_prefix("- ").unwrap_or(t).trim_start();
    let (name, value) = t.split_once(':')?;
    let value = value.trim();
    let value = value
        .strip_prefix('"')
        .and_then(|v| v.strip_suffix('"'))
        .or_else(|| value.strip_prefix('\'').and_then(|v| v.strip_suffix('\'')))
        .unwrap_or(value);
    Some((name.trim().to_string(), value.to_string()))
}

/// The edited text must parse, hold the entry, and leave every other top-level key unchanged.
fn check_edit(original: &Value, new_text: &str, system: &str, key: &str) -> Result<()> {
    let new: Value = serde_yaml::from_str(new_text).context("edited record does not parse")?;
    let refs = new
        .get(KEY.trim_end_matches(':'))
        .and_then(Value::as_sequence)
        .context("edited record has no external_refs list")?;
    let found = refs.iter().any(|r| {
        r.get("system").and_then(Value::as_str) == Some(system)
            && r.get("key").and_then(Value::as_str) == Some(key)
    });
    if !found {
        bail!("edited record does not hold {system} {key} in external_refs");
    }
    let (Some(a), Some(b)) = (original.as_mapping(), new.as_mapping()) else {
        bail!("record is not a mapping");
    };
    for (k, v) in a {
        if k.as_str() == Some("external_refs") {
            continue;
        }
        if b.get(k) != Some(v) {
            bail!("edit would change the top-level key {k:?}; not written");
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const AT: &str = "2026-10-28T10:00:00Z";

    #[test]
    fn appends_a_block_and_keeps_every_byte_before_it() {
        let text = "type: test_case\n# a comment kept\nid: TC_1\n";
        let new = set_external_ref(text, "jira", "PAY-7", AT)
            .unwrap()
            .unwrap();
        assert!(new.starts_with(text), "{new}");
        assert!(
            new.contains("external_refs:\n  - system: jira\n    key: PAY-7\n"),
            "{new}"
        );
        assert!(new.contains("synced_at: \"2026-10-28T10:00:00Z\""), "{new}");
    }

    #[test]
    fn same_system_and_key_is_unchanged() {
        let text = "id: TC_1\nexternal_refs:\n  - system: jira\n    key: PAY-7\n    synced_at: \"2026-01-01T00:00:00Z\"\n";
        assert_eq!(set_external_ref(text, "jira", "PAY-7", AT).unwrap(), None);
    }

    #[test]
    fn other_systems_and_comments_are_kept_when_an_entry_is_added() {
        let text = "id: TC_1\nexternal_refs:\n  # the Jira link\n  - system: jira\n    key: PAY-7\n    synced_at: \"2026-01-01T00:00:00Z\"\n\n  - system: strictdoc\n    key: TC-UID-1\nversion: '2'\n";
        let new = set_external_ref(text, "xray", "XRAY-3", AT)
            .unwrap()
            .unwrap();
        assert!(
            new.contains("  - system: strictdoc\n    key: TC-UID-1\n"),
            "{new}"
        );
        assert!(
            new.contains("  # the Jira link\n  - system: jira\n    key: PAY-7"),
            "{new}"
        );
        assert!(new.ends_with("version: '2'\n"), "{new}");
        assert!(new.contains("  - system: xray\n    key: XRAY-3"), "{new}");
    }

    #[test]
    fn a_new_key_for_the_same_system_replaces_only_that_entry() {
        let text = "id: TC_1\nexternal_refs:\n  - system: strictdoc\n    key: UID-1\n  - system: jira\n    key: PAY-7\n    url: https://x/browse/PAY-7\n    synced_at: \"2026-01-01T00:00:00Z\"\nstatus: draft\n";
        let new = set_external_ref(text, "jira", "PAY-8", AT)
            .unwrap()
            .unwrap();
        assert!(
            new.contains("  - system: strictdoc\n    key: UID-1\n"),
            "{new}"
        );
        assert!(new.contains("    key: PAY-8\n"), "{new}");
        assert!(!new.contains("PAY-7"), "{new}");
        assert!(new.ends_with("status: draft\n"), "{new}");
    }

    #[test]
    fn empty_flow_list_becomes_a_block() {
        let text = "id: TC_1\nexternal_refs: []\nstatus: draft\n";
        let new = set_external_ref(text, "jira", "PAY-7", AT)
            .unwrap()
            .unwrap();
        assert!(
            new.starts_with("id: TC_1\nexternal_refs:\n  - system: jira"),
            "{new}"
        );
        assert!(new.ends_with("status: draft\n"), "{new}");
    }

    #[test]
    fn column_zero_dashes_keep_their_indentation() {
        let text = "id: TC_1\nexternal_refs:\n- system: jira\n  key: PAY-7\n";
        let new = set_external_ref(text, "jira", "PAY-9", AT)
            .unwrap()
            .unwrap();
        assert!(new.contains("- system: jira\n  key: PAY-9"), "{new}");
        assert!(!new.contains("  - system"), "{new}");
    }

    #[test]
    fn a_non_empty_flow_list_is_refused() {
        let text = "id: TC_1\nexternal_refs: [ {system: jira, key: PAY-7} ]\n";
        let err = set_external_ref(text, "jira", "PAY-8", AT).unwrap_err();
        assert!(err.to_string().contains("flow list"), "{err}");
    }

    #[test]
    fn a_file_without_a_final_newline_gets_one_before_the_block() {
        let text = "id: TC_1\nexternal_refs:\n  - system: jira\n    key: PAY-7";
        let new = set_external_ref(text, "xray", "XRAY-8", AT)
            .unwrap()
            .unwrap();
        assert!(
            new.contains("    key: PAY-7\n  - system: xray\n    key: XRAY-8"),
            "{new}"
        );
        // the same system with another key replaces the entry, also without a final newline
        let same = set_external_ref(text, "jira", "PAY-8", AT)
            .unwrap()
            .unwrap();
        assert!(
            same.contains("external_refs:\n  - system: jira\n    key: PAY-8"),
            "{same}"
        );
        assert!(!same.contains("PAY-7"), "{same}");
    }
}
