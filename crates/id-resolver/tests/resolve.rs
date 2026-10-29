use std::path::PathBuf;

use id_resolver::{fill_external_refs, fill_yaml_value, IntegrationConfig, Resolver};
use testcase_models::{ExternalRef, ExternalSystem};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}
fn schemas() -> String {
    format!("{}/schemas/", root().display())
}
fn example(name: &str) -> IntegrationConfig {
    IntegrationConfig::load(
        root().join("examples/integration-config").join(name),
        &schemas(),
    )
    .unwrap_or_else(|e| panic!("{name}: {e:#}"))
}

/// (file, id, expected system, expected url)
const CASES: &[(&str, &str, &str, &str)] = &[
    (
        "jira_cloud.yaml",
        "PAY-2231",
        "jira",
        "https://example.atlassian.net/browse/PAY-2231",
    ),
    (
        "jira_cloud.yaml",
        "AUTH-7",
        "jira",
        "https://example.atlassian.net/browse/AUTH-7",
    ),
    (
        "jira_datacenter.yaml",
        "PAY-2231",
        "jira",
        "https://jira.example.com/browse/PAY-2231",
    ),
    (
        "strictdoc_html.yaml",
        "HLR-12",
        "strictdoc",
        "https://pages.example.com/srs/software-requirements.html#HLR-12",
    ),
    (
        "strictdoc_html.yaml",
        "SYS-4",
        "strictdoc",
        "https://pages.example.com/srs/system-requirements.html#SYS-4",
    ),
    (
        "strictdoc_html.yaml",
        "LLR-3.1",
        "strictdoc",
        "https://pages.example.com/srs/software-design.html#LLR-3.1",
    ),
    (
        "strictdoc_server.yaml",
        "HLR-12",
        "strictdoc",
        "http://localhost:5111/srs/software-requirements.html#HLR-12",
    ),
];

#[test]
fn every_example_config_resolves_id_to_url_and_back() {
    for (file, id, system, url) in CASES {
        let resolver = Resolver::new(&example(file)).unwrap();
        let forward = resolver.resolve_id(id).unwrap();
        assert_eq!(forward.system, *system, "{file} {id}");
        assert_eq!(forward.url, *url, "{file} {id}");
        let back = resolver.resolve_url(url).unwrap();
        assert_eq!(back.id, *id, "{file} url->id");
        assert_eq!(back.system, *system);
        // resolve() accepts both forms
        assert_eq!(resolver.resolve(url).unwrap(), back);
        assert_eq!(resolver.resolve(id).unwrap(), forward);
    }
}

#[test]
fn all_four_deployments_have_an_example() {
    let files: std::collections::BTreeSet<_> = CASES.iter().map(|c| c.0).collect();
    assert_eq!(files.len(), 4);
}

#[test]
fn upstream_sample_without_envelope_loads_and_resolves_both_systems() {
    let cfg = IntegrationConfig::load(
        root().join("schemas/tcms-schemas/samples/integration-config/jira_001.yml"),
        &schemas(),
    )
    .unwrap();
    let r = Resolver::new(&cfg).unwrap();
    assert_eq!(
        r.resolve_id("HLR-12").unwrap().url,
        "https://pages.example.com/srs/software-requirements.html#HLR-12"
    );
    assert_eq!(
        r.resolve_url("https://example.atlassian.net/browse/PAY-2291")
            .unwrap()
            .id,
        "PAY-2291"
    );
}

#[test]
fn id_matching_no_system_is_a_clear_error() {
    let r = Resolver::new(&example("jira_cloud.yaml")).unwrap();
    let e = r.resolve_id("ZZZ-1").unwrap_err().to_string();
    assert!(e.contains("matches no system"), "{e}");
    let e = r.resolve_id("PAY-2231x").unwrap_err().to_string();
    assert!(e.contains("matches no system"), "full match required: {e}");
    let e = r
        .resolve_url("https://other.example.org/browse/PAY-1")
        .unwrap_err()
        .to_string();
    assert!(e.contains("does not belong"), "{e}");
}

fn two_systems(second_pattern: &str) -> String {
    format!(
        "product_id: P\nrequirement_source: jira\nsystems:\n\
         - {{name: jira, type: jira_cloud, base_url: 'https://a.example.net', id_pattern: '^(PAY|AUTH)-[0-9]+$', url_template: '{{base_url}}/browse/{{id}}'}}\n\
         - {{name: other, type: jira_datacenter, base_url: 'https://b.example.net', id_pattern: '{second_pattern}', url_template: '{{base_url}}/browse/{{id}}'}}\n"
    )
}

#[test]
fn overlapping_id_patterns_are_rejected_at_load() {
    for overlapping in ["^[A-Z]+-[0-9]+$", "^PAY-[0-9]+$", "^(AUTH|OPS)-[0-9]+$"] {
        let e = IntegrationConfig::from_yaml_str(&two_systems(overlapping), &schemas(), "t")
            .unwrap_err();
        let msg = format!("{e:#}");
        assert!(msg.contains("overlap"), "{overlapping}: {msg}");
        assert!(msg.contains("jira") && msg.contains("other"), "{msg}");
    }
    // disjoint patterns load
    IntegrationConfig::from_yaml_str(&two_systems("^(OPS|DEV)-[0-9]+$"), &schemas(), "t").unwrap();
}

#[test]
fn config_naming_both_sources_is_rejected() {
    for both in ["[jira, strictdoc]", "'jira,strictdoc'"] {
        let yaml = two_systems("^OPS-[0-9]+$").replace(
            "requirement_source: jira",
            &format!("requirement_source: {both}"),
        );
        let msg = format!(
            "{:#}",
            IntegrationConfig::from_yaml_str(&yaml, &schemas(), "t").unwrap_err()
        );
        assert!(msg.contains("exactly one source"), "{msg}");
    }
}

#[test]
fn source_without_matching_system_and_bad_regex_are_rejected() {
    let yaml = two_systems("^OPS-[0-9]+$")
        .replace("requirement_source: jira", "requirement_source: strictdoc");
    let msg = format!(
        "{:#}",
        IntegrationConfig::from_yaml_str(&yaml, &schemas(), "t").unwrap_err()
    );
    assert!(
        msg.contains("no strictdoc_html or strictdoc_server"),
        "{msg}"
    );
    let msg = format!(
        "{:#}",
        IntegrationConfig::from_yaml_str(&two_systems("^(OPS-[0-9]+$"), &schemas(), "t")
            .unwrap_err()
    );
    assert!(msg.contains("not a valid regex"), "{msg}");
}

#[test]
fn doc_placeholder_without_document_entry_is_an_error() {
    let cfg = example("strictdoc_html.yaml");
    let mut cfg2 = cfg.clone();
    cfg2.systems[0].document_map.remove("LLR");
    let r = Resolver::new(&cfg2).unwrap();
    let e = r.resolve_id("LLR-1").unwrap_err().to_string();
    assert!(e.contains("document_map has no entry"), "{e}");
}

#[test]
fn empty_external_ref_urls_are_filled_and_existing_ones_kept() {
    let r = Resolver::new(
        &IntegrationConfig::load(
            root().join("schemas/tcms-schemas/samples/integration-config/jira_001.yml"),
            &schemas(),
        )
        .unwrap(),
    )
    .unwrap();
    let mut refs = vec![
        ExternalRef {
            system: ExternalSystem::Jira,
            key: "PAY-9".into(),
            url: None,
            synced_at: None,
        },
        ExternalRef {
            system: ExternalSystem::Strictdoc,
            key: "HLR-12".into(),
            url: Some(String::new()),
            synced_at: None,
        },
        ExternalRef {
            system: ExternalSystem::Jira,
            key: "AUTH-1".into(),
            url: Some("https://keep.me".into()),
            synced_at: None,
        },
        ExternalRef {
            system: ExternalSystem::Xray,
            key: "XR-1".into(),
            url: None,
            synced_at: None,
        },
    ];
    assert_eq!(fill_external_refs(&r, &mut refs).unwrap(), 2);
    assert_eq!(
        refs[0].url.as_deref(),
        Some("https://example.atlassian.net/browse/PAY-9")
    );
    assert_eq!(
        refs[1].url.as_deref(),
        Some("https://pages.example.com/srs/software-requirements.html#HLR-12")
    );
    assert_eq!(refs[2].url.as_deref(), Some("https://keep.me"));
    assert_eq!(refs[3].url, None);
    // a jira ref the config cannot resolve is an error
    let mut bad = vec![ExternalRef {
        system: ExternalSystem::Jira,
        key: "NOPE-1".into(),
        url: None,
        synced_at: None,
    }];
    assert!(fill_external_refs(&r, &mut bad).is_err());
}

#[test]
fn fill_yaml_value_fills_requirement_sample_with_url_removed() {
    let r = Resolver::new(
        &IntegrationConfig::load(
            root().join("schemas/tcms-schemas/samples/integration-config/jira_001.yml"),
            &schemas(),
        )
        .unwrap(),
    )
    .unwrap();
    let text =
        std::fs::read_to_string(root().join("schemas/tcms-schemas/samples/requirement/hlr_12.yml"))
            .unwrap();
    let stripped: String = text
        .lines()
        .filter(|l| !l.trim_start().starts_with("url:"))
        .map(|l| format!("{l}\n"))
        .collect();
    assert!(!stripped.contains("url:"));
    let mut v: serde_yaml::Value = serde_yaml::from_str(&stripped).unwrap();
    assert_eq!(fill_yaml_value(&r, &mut v).unwrap(), 1);
    let expected: serde_yaml::Value = serde_yaml::from_str(&text).unwrap();
    assert_eq!(v, expected);
}
