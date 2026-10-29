//! Detection of overlapping `id_pattern`s.
//!
//! Whether two regexes can match a common string is decidable but needs automata products.
//! Instead each pattern yields a bounded set of witness strings (literals, alternation
//! branches, class endpoints, repetition counts min and min+1) and every witness is tried
//! against the other patterns. Everything reported is a real overlap (a concrete string
//! matches both). A pair that only overlaps on strings outside the witness set is not
//! found: this is a documented limit, the common cases (same prefix, broader class, same
//! alternation branch) are caught.

use regex_syntax::hir::{Class, Hir, HirKind};

const MAX_WITNESSES: usize = 256;

/// Anchor a pattern so an id must match entirely.
pub fn full_match(pattern: &str) -> String {
    format!("^(?:{pattern})$")
}

/// First pair `(a, b, witness)` of patterns that both match `witness`.
pub fn find_overlap(patterns: &[(&str, &str)]) -> Option<(String, String, String)> {
    let compiled: Vec<Option<regex::Regex>> = patterns
        .iter()
        .map(|(_, p)| regex::Regex::new(&full_match(p)).ok())
        .collect();
    for (i, (name_i, pat_i)) in patterns.iter().enumerate() {
        let witnesses = witnesses(pat_i);
        for (j, (name_j, _)) in patterns.iter().enumerate() {
            if i == j {
                continue;
            }
            let Some(re_j) = &compiled[j] else { continue };
            if let Some(w) = witnesses.iter().find(|w| re_j.is_match(w)) {
                // The witness comes from pattern i, so it matches i by construction; verify.
                if compiled[i].as_ref().is_some_and(|re| re.is_match(w)) {
                    return Some((name_i.to_string(), name_j.to_string(), w.clone()));
                }
            }
        }
    }
    None
}

/// Strings that (mostly) match `pattern`.
pub fn witnesses(pattern: &str) -> Vec<String> {
    match regex_syntax::parse(pattern) {
        Ok(hir) => generate(&hir),
        Err(_) => Vec::new(),
    }
}

fn generate(hir: &Hir) -> Vec<String> {
    let mut out = match hir.kind() {
        HirKind::Empty | HirKind::Look(_) => vec![String::new()],
        HirKind::Literal(lit) => vec![String::from_utf8_lossy(&lit.0).into_owned()],
        HirKind::Class(Class::Unicode(c)) => {
            let mut chars: Vec<char> = Vec::new();
            for r in c.ranges().iter().take(8) {
                chars.push(r.start());
                chars.push(r.end());
            }
            chars.dedup();
            chars.into_iter().take(6).map(|c| c.to_string()).collect()
        }
        HirKind::Class(Class::Bytes(c)) => c
            .ranges()
            .iter()
            .take(3)
            .flat_map(|r| [r.start() as char, r.end() as char])
            .map(|c| c.to_string())
            .collect(),
        HirKind::Capture(c) => generate(&c.sub),
        HirKind::Alternation(alts) => alts.iter().flat_map(generate).collect(),
        HirKind::Concat(parts) => {
            let mut acc = vec![String::new()];
            for p in parts {
                let sub = generate(p);
                if sub.is_empty() {
                    return Vec::new();
                }
                let mut next = Vec::new();
                'outer: for a in &acc {
                    for s in &sub {
                        next.push(format!("{a}{s}"));
                        if next.len() >= MAX_WITNESSES {
                            break 'outer;
                        }
                    }
                }
                acc = next;
            }
            acc
        }
        HirKind::Repetition(rep) => {
            let sub = generate(&rep.sub);
            let mut counts = vec![rep.min, rep.min + 1];
            if let Some(max) = rep.max {
                counts.retain(|c| *c <= max);
            }
            counts.dedup();
            let mut out = Vec::new();
            for k in counts {
                if sub.is_empty() {
                    break;
                }
                // k copies of the same representative, for each representative
                for s in sub.iter().take(6) {
                    out.push(s.repeat(k as usize));
                }
                // one mixed string using different representatives
                if k > 1 && sub.len() > 1 {
                    out.push(
                        (0..k as usize)
                            .map(|i| sub[i % sub.len()].as_str())
                            .collect(),
                    );
                }
            }
            out
        }
    };
    out.sort();
    out.dedup();
    out.truncate(MAX_WITNESSES);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn disjoint_prefixes_do_not_overlap() {
        let p = [
            ("jira", "(PAY|AUTH)-[0-9]+"),
            ("sd", "(SYS|HLR|LLR)-[0-9.]+"),
        ];
        assert!(find_overlap(&p).is_none());
    }

    #[test]
    fn broader_class_overlaps_specific_prefix() {
        let p = [("generic", "[A-Z]+-[0-9]+"), ("pay", "PAY-[0-9]+")];
        let (a, b, w) = find_overlap(&p).unwrap();
        assert!(a == "generic" || a == "pay");
        assert_ne!(a, b);
        assert!(w.starts_with("PAY-") || w.contains('-'));
    }

    #[test]
    fn shared_alternation_branch_overlaps() {
        let p = [("a", "(PAY|AUTH)-[0-9]+"), ("b", "(AUTH|OPS)-[0-9]+")];
        let (_, _, w) = find_overlap(&p).unwrap();
        assert!(w.starts_with("AUTH-"));
    }

    #[test]
    fn identical_patterns_overlap() {
        let p = [("a", "^HLR-[0-9]+$"), ("b", "^HLR-[0-9]+$")];
        assert!(find_overlap(&p).is_some());
    }
}
