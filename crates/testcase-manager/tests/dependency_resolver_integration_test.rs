use anyhow::Result;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use testcase_manager::{DependencyResolver, TestCase};

fn load_test_case(file_path: &str) -> Result<TestCase> {
    // Use CARGO_MANIFEST_DIR to get the workspace root
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let mut workspace_root = PathBuf::from(manifest_dir);
    workspace_root.pop(); // Go up from crate dir
    workspace_root.pop(); // Go up from crates dir

    let full_path = workspace_root.join(file_path);

    // Note: This test uses raw YAML parsing to test dependency resolution logic
    let yaml_content = fs::read_to_string(&full_path)
        .map_err(|e| anyhow::anyhow!("Failed to read {}: {}", full_path.display(), e))?;
    let test_case: TestCase = serde_yaml::from_str(&yaml_content)?;
    Ok(test_case)
}

#[test]
fn test_dependency_resolution_integration() -> Result<()> {
    let tc1_path = "testcases/examples/dependencies/TC_VAR_001.yml";
    let tc2_path = "testcases/examples/dependencies/TC_VAR_002.yml";

    let tc1 = load_test_case(tc1_path)?;
    let tc2 = load_test_case(tc2_path)?;

    let mut index = HashMap::new();
    index.insert(tc1.id.clone(), tc1.clone());
    index.insert(tc2.id.clone(), tc2.clone());

    let resolver = DependencyResolver::new(index);

    let resolved = resolver.resolve(&tc1)?;

    assert_eq!(resolved.id, "TC_VAR_001");
    assert!(resolved.general_initial_conditions.include.is_none());
    assert!(resolved.initial_conditions.include.is_none());

    let general_system = resolved
        .general_initial_conditions
        .devices
        .get("system")
        .expect("general_initial_conditions should have 'system' device");

    assert!(
        general_system.len() >= 2,
        "Expected at least 2 conditions in general system"
    );

    let initial_filesystem = resolved
        .initial_conditions
        .devices
        .get("filesystem")
        .expect("initial_conditions should have 'filesystem' device");
    assert!(
        !initial_filesystem.is_empty(),
        "Expected at least 1 condition in initial filesystem"
    );

    let initial_system2 = resolved
        .initial_conditions
        .devices
        .get("system2")
        .expect("initial_conditions should have 'system2' device");

    let has_ref_description = initial_system2.iter().any(|item| match item {
        testcase_models::InitialConditionItem::String(s) => {
            s.contains("Echo JSON and capture token")
        }
        _ => false,
    });
    assert!(
        has_ref_description,
        "Expected ref item to be resolved to description"
    );

    let has_step_descriptions = initial_system2.iter().any(|item| match item {
        testcase_models::InitialConditionItem::String(s) => {
            s.contains("Create test file") || s.contains("Echo JSON")
        }
        _ => false,
    });
    assert!(
        has_step_descriptions,
        "Expected test_sequence ref items to be resolved to step descriptions"
    );

    Ok(())
}

#[test]
fn test_resolved_yaml_is_valid_and_parseable() -> Result<()> {
    let tc1_path = "testcases/examples/dependencies/TC_VAR_001.yml";
    let tc2_path = "testcases/examples/dependencies/TC_VAR_002.yml";

    let tc1 = load_test_case(tc1_path)?;
    let tc2 = load_test_case(tc2_path)?;

    let mut index = HashMap::new();
    index.insert(tc1.id.clone(), tc1.clone());
    index.insert(tc2.id.clone(), tc2.clone());

    let resolver = DependencyResolver::new(index);

    let resolved = resolver.resolve(&tc1)?;

    let yaml_content = serde_yaml::to_string(&resolved)?;

    let reparsed: TestCase = serde_yaml::from_str(&yaml_content)?;

    assert_eq!(reparsed.id, resolved.id);
    assert_eq!(reparsed.description, resolved.description);

    assert_eq!(
        reparsed.general_initial_conditions.devices.len(),
        resolved.general_initial_conditions.devices.len()
    );

    assert!(reparsed.general_initial_conditions.include.is_none());
    assert!(reparsed.initial_conditions.include.is_none());

    Ok(())
}

#[test]
fn test_expansion_merges_conditions() -> Result<()> {
    let tc1_path = "testcases/examples/dependencies/TC_VAR_001.yml";
    let tc2_path = "testcases/examples/dependencies/TC_VAR_002.yml";

    let tc1 = load_test_case(tc1_path)?;
    let tc2 = load_test_case(tc2_path)?;

    let mut index = HashMap::new();
    index.insert(tc1.id.clone(), tc1.clone());
    index.insert(tc2.id.clone(), tc2.clone());

    let resolver = DependencyResolver::new(index);

    let resolved = resolver.resolve(&tc1)?;

    let general_system = resolved
        .general_initial_conditions
        .devices
        .get("system")
        .expect("Should have system device");

    assert!(
        general_system.len() >= 2,
        "Expected merged conditions from include"
    );

    let has_bash = general_system.iter().any(|item| match item {
        testcase_models::InitialConditionItem::String(s) => s.contains("Bash shell is available"),
        _ => false,
    });
    assert!(has_bash, "Expected 'Bash shell is available' condition");

    Ok(())
}

#[test]
fn test_ref_items_replaced_with_descriptions() -> Result<()> {
    let tc1_path = "testcases/examples/dependencies/TC_VAR_001.yml";
    let tc2_path = "testcases/examples/dependencies/TC_VAR_002.yml";

    let tc1 = load_test_case(tc1_path)?;
    let tc2 = load_test_case(tc2_path)?;

    let mut index = HashMap::new();
    index.insert(tc1.id.clone(), tc1.clone());
    index.insert(tc2.id.clone(), tc2.clone());

    let resolver = DependencyResolver::new(index);

    let resolved = resolver.resolve(&tc1)?;

    let initial_system2 = resolved
        .initial_conditions
        .devices
        .get("system2")
        .expect("Should have system2 device");

    let all_strings = initial_system2
        .iter()
        .all(|item| matches!(item, testcase_models::InitialConditionItem::String(_)));
    assert!(
        all_strings,
        "All ref items should be replaced with string descriptions"
    );

    let has_expected_description = initial_system2.iter().any(|item| match item {
        testcase_models::InitialConditionItem::String(s) => {
            s.contains("Echo JSON and capture token")
        }
        _ => false,
    });
    assert!(
        has_expected_description,
        "Expected ref to be resolved to step description"
    );

    Ok(())
}

#[test]
fn test_idempotent_resolution() -> Result<()> {
    let tc1_path = "testcases/examples/dependencies/TC_VAR_001.yml";
    let tc2_path = "testcases/examples/dependencies/TC_VAR_002.yml";

    let tc1 = load_test_case(tc1_path)?;
    let tc2 = load_test_case(tc2_path)?;

    let mut index = HashMap::new();
    index.insert(tc1.id.clone(), tc1.clone());
    index.insert(tc2.id.clone(), tc2.clone());

    let resolver = DependencyResolver::new(index);

    let resolved1 = resolver.resolve(&tc1)?;
    let resolved2 = resolver.resolve(&resolved1)?;

    let yaml1 = serde_yaml::to_string(&resolved1)?;
    let yaml2 = serde_yaml::to_string(&resolved2)?;

    assert_eq!(yaml1, yaml2, "Resolution should be idempotent");

    Ok(())
}

#[test]
fn test_test_sequence_initial_conditions_resolved() -> Result<()> {
    let tc1_path = "testcases/examples/dependencies/TC_VAR_001.yml";
    let tc2_path = "testcases/examples/dependencies/TC_VAR_002.yml";

    let tc1 = load_test_case(tc1_path)?;
    let tc2 = load_test_case(tc2_path)?;

    let mut index = HashMap::new();
    index.insert(tc1.id.clone(), tc1.clone());
    index.insert(tc2.id.clone(), tc2.clone());

    let resolver = DependencyResolver::new(index);

    let resolved = resolver.resolve(&tc1)?;

    assert!(!resolved.test_sequences.is_empty());

    let first_seq = &resolved.test_sequences[0];
    assert!(first_seq.initial_conditions.include.is_none());

    let seq_system = first_seq
        .initial_conditions
        .devices
        .get("system")
        .expect("Test sequence should have system device");

    assert!(
        !seq_system.is_empty(),
        "Expected merged conditions in test sequence"
    );

    Ok(())
}
