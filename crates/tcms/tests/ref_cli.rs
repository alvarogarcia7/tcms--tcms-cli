use std::path::PathBuf;
use std::process::{Command, Output};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn tcms(config: &str, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_tcms"))
        .args([
            "--config",
            &root()
                .join("examples/integration-config")
                .join(config)
                .to_string_lossy(),
        ])
        .args(["--schemas-root", &format!("{}/schemas/", root().display())])
        .args(args)
        .env_remove("TCMS_OPEN_PRINT")
        .output()
        .unwrap()
}

fn out(o: &Output) -> String {
    String::from_utf8_lossy(&o.stdout).into_owned()
}

#[test]
fn resolve_id_and_url_print_system_id_and_url() {
    let o = tcms("jira_cloud.yaml", &["ref", "resolve", "PAY-2231"]);
    assert!(o.status.success());
    let s = out(&o);
    assert!(
        s.contains("system: jira") && s.contains("id:     PAY-2231"),
        "{s}"
    );
    assert!(
        s.contains("url:    https://example.atlassian.net/browse/PAY-2231"),
        "{s}"
    );

    let o = tcms(
        "strictdoc_html.yaml",
        &[
            "ref",
            "resolve",
            "https://pages.example.com/srs/software-requirements.html#HLR-12",
        ],
    );
    assert!(o.status.success());
    let s = out(&o);
    assert!(
        s.contains("system: strictdoc") && s.contains("id:     HLR-12"),
        "{s}"
    );
}

#[test]
fn unknown_id_fails_with_clear_message() {
    let o = tcms("jira_cloud.yaml", &["ref", "resolve", "NOPE-1"]);
    assert!(!o.status.success());
    assert!(String::from_utf8_lossy(&o.stderr).contains("matches no system"));
}

#[test]
fn open_prints_url_instead_of_launching_a_browser() {
    let o = tcms(
        "jira_datacenter.yaml",
        &["ref", "open", "AUTH-7", "--print-only"],
    );
    assert!(o.status.success());
    assert_eq!(out(&o).trim(), "https://jira.example.com/browse/AUTH-7");

    let o = Command::new(env!("CARGO_BIN_EXE_tcms"))
        .args([
            "--config",
            &root()
                .join("examples/integration-config/jira_datacenter.yaml")
                .to_string_lossy(),
        ])
        .args(["--schemas-root", &format!("{}/schemas/", root().display())])
        .args(["ref", "open", "AUTH-7"])
        .env("TCMS_OPEN_PRINT", "1")
        .output()
        .unwrap();
    assert!(o.status.success());
    assert_eq!(out(&o).trim(), "https://jira.example.com/browse/AUTH-7");
}

#[test]
fn fill_derives_urls_without_touching_the_file_unless_asked() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("hlr.yaml");
    let original = "id: HLR-12\nschema: tcms/requirement.schema.json\nsource_system: strictdoc\nsource: docs/srs.sdoc#HLR-12\nexternal_refs:\n- system: strictdoc\n  key: HLR-12\n  url: ''\n";
    std::fs::write(&file, original).unwrap();
    let f = file.to_string_lossy().into_owned();

    let o = tcms("strictdoc_html.yaml", &["ref", "fill", &f]);
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    assert!(out(&o).contains("https://pages.example.com/srs/software-requirements.html#HLR-12"));
    assert_eq!(std::fs::read_to_string(&file).unwrap(), original);

    let o = tcms("strictdoc_html.yaml", &["ref", "fill", &f, "--write"]);
    assert!(o.status.success());
    let written = std::fs::read_to_string(&file).unwrap();
    assert!(
        written.contains("software-requirements.html#HLR-12"),
        "{written}"
    );
}

/// The real opener path: a stand-in `xdg-open` / `open` on PATH records its argument.
#[test]
fn open_invokes_the_platform_opener_with_the_resolved_url() {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::tempdir().unwrap();
    let log = dir.path().join("opened.txt");
    let opener = if cfg!(target_os = "macos") {
        "open"
    } else {
        "xdg-open"
    };
    let script = dir.path().join(opener);
    std::fs::write(
        &script,
        format!("#!/bin/sh\necho \"$1\" > '{}'\n", log.display()),
    )
    .unwrap();
    std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();
    let path = format!(
        "{}:{}",
        dir.path().display(),
        std::env::var("PATH").unwrap_or_default()
    );
    let o = Command::new(env!("CARGO_BIN_EXE_tcms"))
        .args([
            "--config",
            &root()
                .join("examples/integration-config/jira_cloud.yaml")
                .to_string_lossy(),
        ])
        .args(["--schemas-root", &format!("{}/schemas/", root().display())])
        .args(["ref", "open", "PAY-2231"])
        .env("PATH", path)
        .env_remove("TCMS_OPEN_PRINT")
        .output()
        .unwrap();
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    assert_eq!(
        std::fs::read_to_string(&log).unwrap().trim(),
        "https://example.atlassian.net/browse/PAY-2231"
    );
}
