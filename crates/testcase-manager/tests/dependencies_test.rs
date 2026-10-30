use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use testcase_manager::dependency_validator::{
    validate_cross_file_dependencies, DependencyValidator,
};
use testcase_models::{
    IncludeRef, InitialConditionItem, InitialConditions, Step, TestCase, TestSequence,
};

#[test]
fn test_deserialize_dependencies_example_1() {
    // Build path relative to workspace root
    let mut yaml_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    yaml_path.push(".."); // Go to crates/
    yaml_path.push(".."); // Go to workspace root
    yaml_path.push("testcases");
    yaml_path.push("examples");
    yaml_path.push("dependencies");
    yaml_path.push("TC_VAR_001.yml");

    assert!(
        yaml_path.exists(),
        "Test file not found: {}",
        yaml_path.display()
    );

    // Note: This test uses raw YAML parsing to test dependency resolution logic
    let yaml_content = fs::read_to_string(&yaml_path)
        .unwrap_or_else(|e| panic!("Failed to read {}: {}", yaml_path.display(), e));

    let test_case: TestCase = serde_yaml::from_str(&yaml_content)
        .unwrap_or_else(|e| panic!("Failed to deserialize {}: {}", yaml_path.display(), e));

    assert_eq!(test_case.id, "TC_VAR_001");
    assert_eq!(test_case.requirement, "VAR_001");
    assert_eq!(test_case.item, 1);
    assert_eq!(test_case.tc, 1);

    assert!(
        test_case.general_initial_conditions.include.is_some(),
        "Expected include in general_initial_conditions"
    );
    let general_include = test_case
        .general_initial_conditions
        .include
        .as_ref()
        .unwrap();
    assert_eq!(general_include.len(), 1);
    assert_eq!(general_include[0].id, "TC_VAR_001");
    assert_eq!(general_include[0].test_sequence, None);

    assert!(
        test_case.initial_conditions.include.is_some(),
        "Expected include in initial_conditions"
    );
    let initial_include = test_case.initial_conditions.include.as_ref().unwrap();
    assert_eq!(initial_include.len(), 1);
    assert_eq!(initial_include[0].id, "TC_VAR_001");

    // Verify system2 conditions are properly deserialized
    let system2_conditions = test_case
        .initial_conditions
        .devices
        .get("system2")
        .expect("Expected system2 conditions");
    assert_eq!(system2_conditions.len(), 2);

    // First item should be a RefItem
    match &system2_conditions[0] {
        InitialConditionItem::RefItem { reference } => {
            assert_eq!(reference, "1234-af");
        }
        _ => panic!("Expected RefItem variant for system2[0]"),
    }

    // Second item should be a TestSequenceRef
    match &system2_conditions[1] {
        InitialConditionItem::TestSequenceRef { test_sequence } => {
            assert_eq!(test_sequence.id, 1);
            assert_eq!(test_sequence.step, "[1,4]");
        }
        _ => panic!("Expected TestSequenceRef variant for system2[1]"),
    }

    assert_eq!(test_case.test_sequences.len(), 1);
    let test_seq = &test_case.test_sequences[0];
    assert_eq!(test_seq.id, 1);

    let seq_include = test_seq
        .initial_conditions
        .include
        .as_ref()
        .expect("Expected include in test sequence initial_conditions");
    assert_eq!(seq_include.len(), 1);
    assert_eq!(seq_include[0].id, "TC_VAR_001");
    assert_eq!(seq_include[0].test_sequence, Some("1".to_string()));
}

#[test]
fn test_deserialize_dependencies_example_2() {
    // Build path relative to workspace root
    let mut yaml_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    yaml_path.push(".."); // Go to crates/
    yaml_path.push(".."); // Go to workspace root
    yaml_path.push("testcases");
    yaml_path.push("examples");
    yaml_path.push("dependencies");
    yaml_path.push("TC_VAR_002.yml");

    assert!(
        yaml_path.exists(),
        "Test file not found: {}",
        yaml_path.display()
    );

    // Note: This test uses raw YAML parsing to test dependency resolution logic
    let yaml_content = fs::read_to_string(&yaml_path)
        .unwrap_or_else(|e| panic!("Failed to read {}: {}", yaml_path.display(), e));

    let test_case: TestCase = serde_yaml::from_str(&yaml_content)
        .unwrap_or_else(|e| panic!("Failed to deserialize {}: {}", yaml_path.display(), e));

    assert_eq!(test_case.id, "TC_VAR_002");
    assert_eq!(test_case.requirement, "VAR_001");
    assert_eq!(test_case.item, 1);
    assert_eq!(test_case.tc, 1);

    assert_eq!(test_case.test_sequences.len(), 1);
    let test_seq = &test_case.test_sequences[0];
    assert_eq!(test_seq.id, 1);
    assert_eq!(test_seq.reference, Some("1234-af".to_string()));

    let step2 = &test_seq.steps[1];
    assert_eq!(step2.step, 2);
    assert_eq!(
        step2.reference,
        Some("12345-ffaaa11-a2213129af".to_string())
    );
}

#[test]
fn test_defined_refs_are_accepted() {
    let mut validator = DependencyValidator::new();

    let mut test_case1 = TestCase::new(
        "REQ001".to_string(),
        1,
        1,
        "TC001".to_string(),
        "Test".to_string(),
    );
    let mut test_sequence = TestSequence::new(1, "Seq".to_string(), "Desc".to_string());
    test_sequence.reference = Some("ref-123".to_string());
    let mut step = Step::new(
        1,
        "Step".to_string(),
        "cmd".to_string(),
        "0".to_string(),
        "output".to_string(),
    );
    step.reference = Some("step-ref-456".to_string());
    test_sequence.steps.push(step);
    test_case1.test_sequences.push(test_sequence);

    validator.collect_definitions(&PathBuf::from("test1.yaml"), &test_case1);

    let mut test_case2 = TestCase::new(
        "REQ002".to_string(),
        2,
        2,
        "TC002".to_string(),
        "Test".to_string(),
    );
    let mut initial_conditions = InitialConditions::default();
    initial_conditions.devices.insert(
        "device1".to_string(),
        vec![
            InitialConditionItem::RefItem {
                reference: "ref-123".to_string(),
            },
            InitialConditionItem::RefItem {
                reference: "step-ref-456".to_string(),
            },
        ],
    );
    test_case2.initial_conditions = initial_conditions;

    let errors = validator.validate_references(&PathBuf::from("test2.yaml"), &test_case2);
    assert_eq!(
        errors.len(),
        0,
        "Defined refs should be accepted without errors"
    );
}

#[test]
fn test_undefined_refs_produce_errors() {
    let validator = DependencyValidator::new();

    let mut test_case = TestCase::new(
        "REQ001".to_string(),
        1,
        1,
        "TC001".to_string(),
        "Test".to_string(),
    );

    let mut initial_conditions = InitialConditions::default();
    initial_conditions.devices.insert(
        "device1".to_string(),
        vec![InitialConditionItem::RefItem {
            reference: "undefined-ref".to_string(),
        }],
    );
    test_case.initial_conditions = initial_conditions;

    let errors = validator.validate_references(&PathBuf::from("test.yaml"), &test_case);

    assert_eq!(
        errors.len(),
        1,
        "Undefined ref should produce exactly one error"
    );
    assert_eq!(errors[0].reference, "undefined-ref");
    assert!(errors[0].location.contains("initial_conditions.device1[0]"));
}

#[test]
fn test_include_resolution_for_general_initial_conditions() {
    let mut validator = DependencyValidator::new();

    let test_case1 = TestCase::new(
        "REQ001".to_string(),
        1,
        1,
        "TC001".to_string(),
        "Test".to_string(),
    );
    validator.collect_definitions(&PathBuf::from("test1.yaml"), &test_case1);

    let mut test_case2 = TestCase::new(
        "REQ002".to_string(),
        2,
        2,
        "TC002".to_string(),
        "Test".to_string(),
    );
    let initial_conditions = InitialConditions {
        include: Some(vec![IncludeRef {
            id: "TC001".to_string(),
            test_sequence: None,
        }]),
        ..Default::default()
    };
    test_case2.general_initial_conditions = initial_conditions;

    let errors = validator.validate_references(&PathBuf::from("test2.yaml"), &test_case2);
    assert_eq!(
        errors.len(),
        0,
        "Valid include reference should not produce errors"
    );
}

#[test]
fn test_include_resolution_for_initial_conditions() {
    let mut validator = DependencyValidator::new();

    let test_case1 = TestCase::new(
        "REQ001".to_string(),
        1,
        1,
        "TC001".to_string(),
        "Test".to_string(),
    );
    validator.collect_definitions(&PathBuf::from("test1.yaml"), &test_case1);

    let mut test_case2 = TestCase::new(
        "REQ002".to_string(),
        2,
        2,
        "TC002".to_string(),
        "Test".to_string(),
    );
    let initial_conditions = InitialConditions {
        include: Some(vec![IncludeRef {
            id: "TC001".to_string(),
            test_sequence: None,
        }]),
        ..Default::default()
    };
    test_case2.initial_conditions = initial_conditions;

    let errors = validator.validate_references(&PathBuf::from("test2.yaml"), &test_case2);
    assert_eq!(
        errors.len(),
        0,
        "Valid include reference should not produce errors"
    );
}

#[test]
fn test_include_resolution_for_test_sequence_initial_conditions() {
    let mut validator = DependencyValidator::new();

    let test_case1 = TestCase::new(
        "REQ001".to_string(),
        1,
        1,
        "TC001".to_string(),
        "Test".to_string(),
    );
    validator.collect_definitions(&PathBuf::from("test1.yaml"), &test_case1);

    let mut test_case2 = TestCase::new(
        "REQ002".to_string(),
        2,
        2,
        "TC002".to_string(),
        "Test".to_string(),
    );
    let mut test_sequence = TestSequence::new(1, "Seq".to_string(), "Desc".to_string());
    test_sequence.initial_conditions = InitialConditions {
        include: Some(vec![IncludeRef {
            id: "TC001".to_string(),
            test_sequence: Some("1".to_string()),
        }]),
        ..Default::default()
    };
    test_case2.test_sequences.push(test_sequence);

    let errors = validator.validate_references(&PathBuf::from("test2.yaml"), &test_case2);
    assert_eq!(
        errors.len(),
        0,
        "Valid include reference with test_sequence should not produce errors"
    );
}

#[test]
fn test_mixed_initial_condition_items_programmatic() {
    let mut initial_conditions = InitialConditions::default();
    initial_conditions.devices.insert(
        "device1".to_string(),
        vec![
            InitialConditionItem::String("String condition".to_string()),
            InitialConditionItem::RefItem {
                reference: "ref-123".to_string(),
            },
        ],
    );

    let device1_conditions = initial_conditions.devices.get("device1").unwrap();
    assert_eq!(device1_conditions.len(), 2);

    match &device1_conditions[0] {
        InitialConditionItem::String(s) => assert_eq!(s, "String condition"),
        _ => panic!("Expected String variant"),
    }

    match &device1_conditions[1] {
        InitialConditionItem::RefItem { reference } => assert_eq!(reference, "ref-123"),
        _ => panic!("Expected RefItem variant"),
    }
}

#[test]
fn test_duplicate_refs_are_allowed() {
    let mut validator = DependencyValidator::new();

    let mut test_case1 = TestCase::new(
        "REQ001".to_string(),
        1,
        1,
        "TC001".to_string(),
        "Test".to_string(),
    );
    let mut test_sequence1 = TestSequence::new(1, "Seq1".to_string(), "Desc".to_string());
    test_sequence1.reference = Some("ref-dup".to_string());
    test_case1.test_sequences.push(test_sequence1);

    let mut test_sequence2 = TestSequence::new(2, "Seq2".to_string(), "Desc".to_string());
    test_sequence2.reference = Some("ref-dup".to_string());
    test_case1.test_sequences.push(test_sequence2);

    validator.collect_definitions(&PathBuf::from("test1.yaml"), &test_case1);

    let mut test_case2 = TestCase::new(
        "REQ002".to_string(),
        2,
        2,
        "TC002".to_string(),
        "Test".to_string(),
    );
    let mut initial_conditions = InitialConditions::default();
    initial_conditions.devices.insert(
        "device1".to_string(),
        vec![InitialConditionItem::RefItem {
            reference: "ref-dup".to_string(),
        }],
    );
    test_case2.initial_conditions = initial_conditions;

    let errors = validator.validate_references(&PathBuf::from("test2.yaml"), &test_case2);
    assert_eq!(
        errors.len(),
        0,
        "Duplicate refs defined should still allow resolution"
    );
}

#[test]
fn test_self_referencing_within_same_test_case() {
    let yaml_content = r#"
requirement: "REQ001"
item: 1
tc: 1
id: "TC001"
description: "Self-referencing test case"
general_initial_conditions:
  include:
    - id: "TC001"
  device1:
    - "Condition 1"
initial_conditions:
  device1:
    - "Condition 1"
test_sequences:
  - id: 1
    name: "Seq"
    description: "Desc"
    initial_conditions:
      device1:
        - "Seq condition"
    steps:
      - step: 1
        description: "Step 1"
        command: "cmd1"
        expected:
          success: true
          result: "OK"
          output: "Success"
      - step: 2
        description: "Step 2"
        command: "cmd2"
        expected:
          result: "OK"
          output: "Success"
  - id: 2
    name: "Seq2"
    description: "Desc2"
    initial_conditions:
      device1:
        - "Seq condition"
    steps:
      - step: 1
        description: "Step 1"
        command: "cmd1"
        expected:
          success: true
          result: "OK"
          output: "Success"
      - step: 2
        description: "Step 2"
        command: "cmd2"
        expected:
          result: "OK"
          output: "Success"
"#;

    let test_case: TestCase = serde_yaml::from_str(yaml_content)
        .expect("Failed to deserialize self-referencing test case");

    let mut validator = DependencyValidator::new();
    validator.collect_definitions(&PathBuf::from("test.yaml"), &test_case);

    let errors = validator.validate_references(&PathBuf::from("test.yaml"), &test_case);
    assert_eq!(
        errors.len(),
        0,
        "Self-referencing within same test case should be allowed"
    );
}

#[test]
fn test_circular_includes_across_files() {
    let mut validator = DependencyValidator::new();

    let mut test_case1 = TestCase::new(
        "REQ001".to_string(),
        1,
        1,
        "TC001".to_string(),
        "Test1".to_string(),
    );
    test_case1.general_initial_conditions = InitialConditions {
        include: Some(vec![IncludeRef {
            id: "TC002".to_string(),
            test_sequence: None,
        }]),
        ..Default::default()
    };

    let mut test_case2 = TestCase::new(
        "REQ002".to_string(),
        2,
        2,
        "TC002".to_string(),
        "Test2".to_string(),
    );
    test_case2.general_initial_conditions = InitialConditions {
        include: Some(vec![IncludeRef {
            id: "TC001".to_string(),
            test_sequence: None,
        }]),
        ..Default::default()
    };

    validator.collect_definitions(&PathBuf::from("test1.yaml"), &test_case1);
    validator.collect_definitions(&PathBuf::from("test2.yaml"), &test_case2);

    let errors1 = validator.validate_references(&PathBuf::from("test1.yaml"), &test_case1);
    let errors2 = validator.validate_references(&PathBuf::from("test2.yaml"), &test_case2);

    assert_eq!(
        errors1.len(),
        0,
        "Circular includes should be allowed (no errors for TC001)"
    );
    assert_eq!(
        errors2.len(),
        0,
        "Circular includes should be allowed (no errors for TC002)"
    );
}

#[test]
fn test_cross_file_validation_success() {
    let mut test_case1 = TestCase::new(
        "REQ001".to_string(),
        1,
        1,
        "TC001".to_string(),
        "Test1".to_string(),
    );
    let mut test_sequence = TestSequence::new(1, "Seq".to_string(), "Desc".to_string());
    test_sequence.reference = Some("ref-abc".to_string());
    test_case1.test_sequences.push(test_sequence);

    let mut test_case2 = TestCase::new(
        "REQ002".to_string(),
        2,
        2,
        "TC002".to_string(),
        "Test2".to_string(),
    );
    let mut initial_conditions = InitialConditions {
        include: Some(vec![IncludeRef {
            id: "TC001".to_string(),
            test_sequence: None,
        }]),
        devices: HashMap::new(),
    };
    initial_conditions.devices.insert(
        "device1".to_string(),
        vec![InitialConditionItem::RefItem {
            reference: "ref-abc".to_string(),
        }],
    );
    test_case2.initial_conditions = initial_conditions;

    let files = vec![
        (PathBuf::from("test1.yaml"), test_case1),
        (PathBuf::from("test2.yaml"), test_case2),
    ];

    let result = validate_cross_file_dependencies(&files);
    assert!(
        result.is_ok(),
        "Cross-file validation should succeed when all references are defined"
    );
}

#[test]
fn test_cross_file_validation_failure() {
    let test_case1 = TestCase::new(
        "REQ001".to_string(),
        1,
        1,
        "TC001".to_string(),
        "Test1".to_string(),
    );

    let mut test_case2 = TestCase::new(
        "REQ002".to_string(),
        2,
        2,
        "TC002".to_string(),
        "Test2".to_string(),
    );
    let mut initial_conditions = InitialConditions {
        include: Some(vec![IncludeRef {
            id: "TC999".to_string(),
            test_sequence: None,
        }]),
        devices: HashMap::new(),
    };
    initial_conditions.devices.insert(
        "device1".to_string(),
        vec![InitialConditionItem::RefItem {
            reference: "undefined-ref".to_string(),
        }],
    );
    test_case2.general_initial_conditions = initial_conditions;

    let files = vec![
        (PathBuf::from("test1.yaml"), test_case1),
        (PathBuf::from("test2.yaml"), test_case2),
    ];

    let result = validate_cross_file_dependencies(&files);
    assert!(
        result.is_err(),
        "Cross-file validation should fail when references are undefined"
    );

    let errors = result.unwrap_err();
    assert_eq!(
        errors.len(),
        2,
        "Expected 2 errors (TC999 and undefined-ref)"
    );
}

#[test]
fn test_undefined_include_in_general_initial_conditions() {
    let validator = DependencyValidator::new();

    let mut test_case = TestCase::new(
        "REQ001".to_string(),
        1,
        1,
        "TC001".to_string(),
        "Test".to_string(),
    );
    test_case.general_initial_conditions = InitialConditions {
        include: Some(vec![IncludeRef {
            id: "TC999".to_string(),
            test_sequence: None,
        }]),
        ..Default::default()
    };

    let errors = validator.validate_references(&PathBuf::from("test.yaml"), &test_case);

    assert_eq!(
        errors.len(),
        1,
        "Undefined include should produce exactly one error"
    );
    assert_eq!(errors[0].reference, "TC999");
    assert!(errors[0]
        .location
        .contains("general_initial_conditions.include[0]"));
}

#[test]
fn test_multiple_undefined_refs_in_different_locations() {
    let validator = DependencyValidator::new();

    let mut test_case = TestCase::new(
        "REQ001".to_string(),
        1,
        1,
        "TC001".to_string(),
        "Test".to_string(),
    );

    test_case.general_initial_conditions = InitialConditions {
        include: Some(vec![IncludeRef {
            id: "TC_UNDEFINED_1".to_string(),
            test_sequence: None,
        }]),
        devices: HashMap::new(),
    };

    let mut initial_conditions = InitialConditions {
        include: Some(vec![IncludeRef {
            id: "TC_UNDEFINED_2".to_string(),
            test_sequence: None,
        }]),
        devices: HashMap::new(),
    };
    initial_conditions.devices.insert(
        "device1".to_string(),
        vec![InitialConditionItem::RefItem {
            reference: "undefined-ref-1".to_string(),
        }],
    );
    test_case.initial_conditions = initial_conditions;

    let mut test_sequence = TestSequence::new(1, "Seq".to_string(), "Desc".to_string());
    let mut seq_initial_conditions = InitialConditions {
        include: Some(vec![IncludeRef {
            id: "TC_UNDEFINED_3".to_string(),
            test_sequence: None,
        }]),
        devices: HashMap::new(),
    };
    seq_initial_conditions.devices.insert(
        "device2".to_string(),
        vec![InitialConditionItem::RefItem {
            reference: "undefined-ref-2".to_string(),
        }],
    );
    test_sequence.initial_conditions = seq_initial_conditions;
    test_case.test_sequences.push(test_sequence);

    let errors = validator.validate_references(&PathBuf::from("test.yaml"), &test_case);

    assert_eq!(
        errors.len(),
        5,
        "Expected 5 errors: 3 undefined test case IDs and 2 undefined refs"
    );

    let ref_errors: Vec<_> = errors
        .iter()
        .filter(|e| e.reference == "undefined-ref-1" || e.reference == "undefined-ref-2")
        .collect();
    assert_eq!(ref_errors.len(), 2, "Expected 2 ref errors");

    let tc_errors: Vec<_> = errors
        .iter()
        .filter(|e| e.reference.starts_with("TC_UNDEFINED"))
        .collect();
    assert_eq!(tc_errors.len(), 3, "Expected 3 test case ID errors");
}

// ===== InitialConditions Unit Tests =====

#[test]
fn test_initial_conditions_default() {
    let ic = InitialConditions::default();
    assert!(ic.is_empty());
    assert!(ic.include.is_none());
    assert!(ic.devices.is_empty());
}

#[test]
fn test_initial_conditions_is_empty_with_include() {
    let ic = InitialConditions {
        include: Some(vec![IncludeRef {
            id: "TC001".to_string(),
            test_sequence: None,
        }]),
        devices: HashMap::new(),
    };
    assert!(!ic.is_empty());
}

#[test]
fn test_initial_conditions_is_empty_with_devices() {
    let mut ic = InitialConditions {
        include: None,
        devices: HashMap::new(),
    };
    ic.devices.insert(
        "device1".to_string(),
        vec![InitialConditionItem::String("condition".to_string())],
    );
    assert!(!ic.is_empty());
}

#[test]
fn test_initial_conditions_is_empty_with_both() {
    let mut ic = InitialConditions {
        include: Some(vec![IncludeRef {
            id: "TC001".to_string(),
            test_sequence: None,
        }]),
        devices: HashMap::new(),
    };
    ic.devices.insert(
        "device1".to_string(),
        vec![InitialConditionItem::String("condition".to_string())],
    );
    assert!(!ic.is_empty());
}

#[test]
fn test_initial_conditions_yaml_serialization_with_include() {
    let ic = InitialConditions {
        include: Some(vec![
            IncludeRef {
                id: "TC001".to_string(),
                test_sequence: None,
            },
            IncludeRef {
                id: "TC002".to_string(),
                test_sequence: Some("1".to_string()),
            },
        ]),
        devices: HashMap::new(),
    };

    let yaml = serde_yaml::to_string(&ic).unwrap();
    let deserialized: InitialConditions = serde_yaml::from_str(&yaml).unwrap();

    assert_eq!(ic, deserialized);
    assert!(yaml.contains("include:"));
    assert!(yaml.contains("TC001"));
    assert!(yaml.contains("TC002"));
}

#[test]
fn test_initial_conditions_yaml_deserialization_with_include() {
    let yaml = r#"
include:
  - id: "TC001"
  - id: "TC002"
    test_sequence: "1"
"#;
    let ic: InitialConditions = serde_yaml::from_str(yaml).unwrap();

    assert!(!ic.is_empty());
    assert!(ic.include.is_some());
    let include = ic.include.unwrap();
    assert_eq!(include.len(), 2);
    assert_eq!(include[0].id, "TC001");
    assert_eq!(include[0].test_sequence, None);
    assert_eq!(include[1].id, "TC002");
    assert_eq!(include[1].test_sequence, Some("1".to_string()));
}

#[test]
fn test_initial_conditions_yaml_serialization_with_devices() {
    let mut ic = InitialConditions {
        include: None,
        devices: HashMap::new(),
    };
    ic.devices.insert(
        "device1".to_string(),
        vec![
            InitialConditionItem::String("String condition".to_string()),
            InitialConditionItem::RefItem {
                reference: "ref-123".to_string(),
            },
        ],
    );
    ic.devices.insert(
        "device2".to_string(),
        vec![InitialConditionItem::String(
            "Another condition".to_string(),
        )],
    );

    let yaml = serde_yaml::to_string(&ic).unwrap();
    let deserialized: InitialConditions = serde_yaml::from_str(&yaml).unwrap();

    assert_eq!(ic, deserialized);
    assert!(yaml.contains("device1:"));
    assert!(yaml.contains("device2:"));
    assert!(yaml.contains("String condition"));
}

#[test]
fn test_initial_conditions_yaml_deserialization_with_devices() {
    let yaml = r#"
device1:
  - "String condition"
  - ref: "ref-123"
device2:
  - "Another condition"
"#;
    let ic: InitialConditions = serde_yaml::from_str(yaml).unwrap();

    assert!(!ic.is_empty());
    assert!(ic.include.is_none());
    assert_eq!(ic.devices.len(), 2);

    let device1_conditions = ic.devices.get("device1").unwrap();
    assert_eq!(device1_conditions.len(), 2);
    match &device1_conditions[0] {
        InitialConditionItem::String(s) => assert_eq!(s, "String condition"),
        _ => panic!("Expected String variant"),
    }
    match &device1_conditions[1] {
        InitialConditionItem::RefItem { reference } => assert_eq!(reference, "ref-123"),
        _ => panic!("Expected RefItem variant"),
    }

    let device2_conditions = ic.devices.get("device2").unwrap();
    assert_eq!(device2_conditions.len(), 1);
    match &device2_conditions[0] {
        InitialConditionItem::String(s) => assert_eq!(s, "Another condition"),
        _ => panic!("Expected String variant"),
    }
}

#[test]
fn test_initial_conditions_yaml_serialization_with_include_and_devices() {
    let mut ic = InitialConditions {
        include: Some(vec![IncludeRef {
            id: "TC001".to_string(),
            test_sequence: None,
        }]),
        devices: HashMap::new(),
    };
    ic.devices.insert(
        "device1".to_string(),
        vec![InitialConditionItem::String("condition".to_string())],
    );

    let yaml = serde_yaml::to_string(&ic).unwrap();
    let deserialized: InitialConditions = serde_yaml::from_str(&yaml).unwrap();

    assert_eq!(ic, deserialized);
    assert!(yaml.contains("include:"));
    assert!(yaml.contains("device1:"));
}

#[test]
fn test_initial_conditions_yaml_deserialization_with_include_and_devices() {
    let yaml = r#"
include:
  - id: "TC001"
device1:
  - "condition"
device2:
  - ref: "ref-456"
"#;
    let ic: InitialConditions = serde_yaml::from_str(yaml).unwrap();

    assert!(!ic.is_empty());
    assert!(ic.include.is_some());
    assert_eq!(ic.devices.len(), 2);

    let include = ic.include.unwrap();
    assert_eq!(include.len(), 1);
    assert_eq!(include[0].id, "TC001");
}

#[test]
fn test_initial_conditions_json_serialization_with_include() {
    let ic = InitialConditions {
        include: Some(vec![
            IncludeRef {
                id: "TC001".to_string(),
                test_sequence: None,
            },
            IncludeRef {
                id: "TC002".to_string(),
                test_sequence: Some("2".to_string()),
            },
        ]),
        devices: HashMap::new(),
    };

    let json = serde_json::to_string(&ic).unwrap();
    let deserialized: InitialConditions = serde_json::from_str(&json).unwrap();

    assert_eq!(ic, deserialized);
    assert!(json.contains("\"include\""));
    assert!(json.contains("\"TC001\""));
    assert!(json.contains("\"TC002\""));
}

#[test]
fn test_initial_conditions_json_deserialization_with_include() {
    let json = r#"{
        "include": [
            {"id": "TC001"},
            {"id": "TC002", "test_sequence": "3"}
        ]
    }"#;
    let ic: InitialConditions = serde_json::from_str(json).unwrap();

    assert!(!ic.is_empty());
    assert!(ic.include.is_some());
    let include = ic.include.unwrap();
    assert_eq!(include.len(), 2);
    assert_eq!(include[0].id, "TC001");
    assert_eq!(include[0].test_sequence, None);
    assert_eq!(include[1].id, "TC002");
    assert_eq!(include[1].test_sequence, Some("3".to_string()));
}

#[test]
fn test_initial_conditions_json_serialization_with_devices() {
    let mut ic = InitialConditions {
        include: None,
        devices: HashMap::new(),
    };
    ic.devices.insert(
        "device1".to_string(),
        vec![
            InitialConditionItem::String("condition1".to_string()),
            InitialConditionItem::RefItem {
                reference: "ref-abc".to_string(),
            },
        ],
    );

    let json = serde_json::to_string(&ic).unwrap();
    let deserialized: InitialConditions = serde_json::from_str(&json).unwrap();

    assert_eq!(ic, deserialized);
    assert!(json.contains("\"device1\""));
    assert!(json.contains("\"condition1\""));
    assert!(json.contains("\"ref\""));
}

#[test]
fn test_initial_conditions_json_deserialization_with_devices() {
    let json = r#"{
        "device1": [
            "String condition",
            {"ref": "ref-xyz"}
        ],
        "device2": [
            "Another string"
        ]
    }"#;
    let ic: InitialConditions = serde_json::from_str(json).unwrap();

    assert!(!ic.is_empty());
    assert!(ic.include.is_none());
    assert_eq!(ic.devices.len(), 2);

    let device1_conditions = ic.devices.get("device1").unwrap();
    assert_eq!(device1_conditions.len(), 2);
    match &device1_conditions[0] {
        InitialConditionItem::String(s) => assert_eq!(s, "String condition"),
        _ => panic!("Expected String variant"),
    }
    match &device1_conditions[1] {
        InitialConditionItem::RefItem { reference } => assert_eq!(reference, "ref-xyz"),
        _ => panic!("Expected RefItem variant"),
    }
}

#[test]
fn test_initial_conditions_json_serialization_with_include_and_devices() {
    let mut ic = InitialConditions {
        include: Some(vec![IncludeRef {
            id: "TC001".to_string(),
            test_sequence: Some("1".to_string()),
        }]),
        devices: HashMap::new(),
    };
    ic.devices.insert(
        "device1".to_string(),
        vec![InitialConditionItem::String("condition".to_string())],
    );

    let json = serde_json::to_string(&ic).unwrap();
    let deserialized: InitialConditions = serde_json::from_str(&json).unwrap();

    assert_eq!(ic, deserialized);
    assert!(json.contains("\"include\""));
    assert!(json.contains("\"device1\""));
}

#[test]
fn test_initial_conditions_json_deserialization_with_include_and_devices() {
    let json = r#"{
        "include": [
            {"id": "TC001", "test_sequence": "1"}
        ],
        "device1": ["condition1"],
        "device2": [{"ref": "ref-789"}]
    }"#;
    let ic: InitialConditions = serde_json::from_str(json).unwrap();

    assert!(!ic.is_empty());
    assert!(ic.include.is_some());
    assert_eq!(ic.devices.len(), 2);

    let include = ic.include.unwrap();
    assert_eq!(include.len(), 1);
    assert_eq!(include[0].id, "TC001");
    assert_eq!(include[0].test_sequence, Some("1".to_string()));
}

#[test]
fn test_initial_condition_item_string_variant_yaml() {
    let yaml = r#""Plain string condition""#;
    let item: InitialConditionItem = serde_yaml::from_str(yaml).unwrap();

    match &item {
        InitialConditionItem::String(s) => assert_eq!(s, "Plain string condition"),
        _ => panic!("Expected String variant"),
    }

    let serialized = serde_yaml::to_string(&item).unwrap();
    let deserialized: InitialConditionItem = serde_yaml::from_str(&serialized).unwrap();
    assert_eq!(item, deserialized);
}

#[test]
fn test_initial_condition_item_ref_item_variant_yaml() {
    let yaml = r#"
ref: "ref-12345"
"#;
    let item: InitialConditionItem = serde_yaml::from_str(yaml).unwrap();

    match &item {
        InitialConditionItem::RefItem { reference } => assert_eq!(reference, "ref-12345"),
        _ => panic!("Expected RefItem variant"),
    }

    let serialized = serde_yaml::to_string(&item).unwrap();
    let deserialized: InitialConditionItem = serde_yaml::from_str(&serialized).unwrap();
    assert_eq!(item, deserialized);
}

#[test]
fn test_initial_condition_item_test_sequence_ref_variant_yaml() {
    let yaml = r#"
test_sequence:
  id: 42
  step: "step-1"
"#;
    let item: InitialConditionItem = serde_yaml::from_str(yaml).unwrap();

    match &item {
        InitialConditionItem::TestSequenceRef { test_sequence } => {
            assert_eq!(test_sequence.id, 42);
            assert_eq!(test_sequence.step, "step-1");
        }
        _ => panic!("Expected TestSequenceRef variant"),
    }

    let serialized = serde_yaml::to_string(&item).unwrap();
    let deserialized: InitialConditionItem = serde_yaml::from_str(&serialized).unwrap();
    assert_eq!(item, deserialized);
}

#[test]
fn test_initial_condition_item_string_variant_json() {
    let json = r#""Plain JSON string""#;
    let item: InitialConditionItem = serde_json::from_str(json).unwrap();

    match &item {
        InitialConditionItem::String(s) => assert_eq!(s, "Plain JSON string"),
        _ => panic!("Expected String variant"),
    }

    let serialized = serde_json::to_string(&item).unwrap();
    let deserialized: InitialConditionItem = serde_json::from_str(&serialized).unwrap();
    assert_eq!(item, deserialized);
}

#[test]
fn test_initial_condition_item_ref_item_variant_json() {
    let json = r#"{"ref": "ref-json-123"}"#;
    let item: InitialConditionItem = serde_json::from_str(json).unwrap();

    match &item {
        InitialConditionItem::RefItem { reference } => assert_eq!(reference, "ref-json-123"),
        _ => panic!("Expected RefItem variant"),
    }

    let serialized = serde_json::to_string(&item).unwrap();
    let deserialized: InitialConditionItem = serde_json::from_str(&serialized).unwrap();
    assert_eq!(item, deserialized);
}

#[test]
fn test_initial_condition_item_test_sequence_ref_variant_json() {
    let json = r#"{
        "test_sequence": {
            "id": 99,
            "step": "step-abc"
        }
    }"#;
    let item: InitialConditionItem = serde_json::from_str(json).unwrap();

    match &item {
        InitialConditionItem::TestSequenceRef { test_sequence } => {
            assert_eq!(test_sequence.id, 99);
            assert_eq!(test_sequence.step, "step-abc");
        }
        _ => panic!("Expected TestSequenceRef variant"),
    }

    let serialized = serde_json::to_string(&item).unwrap();
    let deserialized: InitialConditionItem = serde_json::from_str(&serialized).unwrap();
    assert_eq!(item, deserialized);
}

#[test]
fn test_mixed_initial_condition_items_yaml_roundtrip() {
    let yaml = r#"
device1:
  - "String condition 1"
  - ref: "ref-abc"
  - test_sequence:
      id: 10
      step: "step-1"
  - "String condition 2"
  - ref: "ref-xyz"
device2:
  - test_sequence:
      id: 20
      step: "step-2"
  - "Another string"
"#;
    let ic: InitialConditions = serde_yaml::from_str(yaml).unwrap();

    // Verify device1 conditions
    let device1_conditions = ic.devices.get("device1").unwrap();
    assert_eq!(device1_conditions.len(), 5);

    match &device1_conditions[0] {
        InitialConditionItem::String(s) => assert_eq!(s, "String condition 1"),
        _ => panic!("Expected String variant at index 0"),
    }
    match &device1_conditions[1] {
        InitialConditionItem::RefItem { reference } => assert_eq!(reference, "ref-abc"),
        _ => panic!("Expected RefItem variant at index 1"),
    }
    match &device1_conditions[2] {
        InitialConditionItem::TestSequenceRef { test_sequence } => {
            assert_eq!(test_sequence.id, 10);
            assert_eq!(test_sequence.step, "step-1");
        }
        _ => panic!("Expected TestSequenceRef variant at index 2"),
    }
    match &device1_conditions[3] {
        InitialConditionItem::String(s) => assert_eq!(s, "String condition 2"),
        _ => panic!("Expected String variant at index 3"),
    }
    match &device1_conditions[4] {
        InitialConditionItem::RefItem { reference } => assert_eq!(reference, "ref-xyz"),
        _ => panic!("Expected RefItem variant at index 4"),
    }

    // Verify device2 conditions
    let device2_conditions = ic.devices.get("device2").unwrap();
    assert_eq!(device2_conditions.len(), 2);

    match &device2_conditions[0] {
        InitialConditionItem::TestSequenceRef { test_sequence } => {
            assert_eq!(test_sequence.id, 20);
            assert_eq!(test_sequence.step, "step-2");
        }
        _ => panic!("Expected TestSequenceRef variant at index 0"),
    }
    match &device2_conditions[1] {
        InitialConditionItem::String(s) => assert_eq!(s, "Another string"),
        _ => panic!("Expected String variant at index 1"),
    }

    // Roundtrip test
    let serialized = serde_yaml::to_string(&ic).unwrap();
    let deserialized: InitialConditions = serde_yaml::from_str(&serialized).unwrap();
    assert_eq!(ic, deserialized);
}

#[test]
fn test_mixed_initial_condition_items_json_roundtrip() {
    let json = r#"{
        "device1": [
            "String condition 1",
            {"ref": "ref-123"},
            {"test_sequence": {"id": 5, "step": "s1"}},
            "String condition 2"
        ],
        "device2": [
            {"test_sequence": {"id": 15, "step": "s2"}},
            {"ref": "ref-456"}
        ]
    }"#;
    let ic: InitialConditions = serde_json::from_str(json).unwrap();

    // Verify device1 conditions
    let device1_conditions = ic.devices.get("device1").unwrap();
    assert_eq!(device1_conditions.len(), 4);

    match &device1_conditions[0] {
        InitialConditionItem::String(s) => assert_eq!(s, "String condition 1"),
        _ => panic!("Expected String variant at index 0"),
    }
    match &device1_conditions[1] {
        InitialConditionItem::RefItem { reference } => assert_eq!(reference, "ref-123"),
        _ => panic!("Expected RefItem variant at index 1"),
    }
    match &device1_conditions[2] {
        InitialConditionItem::TestSequenceRef { test_sequence } => {
            assert_eq!(test_sequence.id, 5);
            assert_eq!(test_sequence.step, "s1");
        }
        _ => panic!("Expected TestSequenceRef variant at index 2"),
    }
    match &device1_conditions[3] {
        InitialConditionItem::String(s) => assert_eq!(s, "String condition 2"),
        _ => panic!("Expected String variant at index 3"),
    }

    // Verify device2 conditions
    let device2_conditions = ic.devices.get("device2").unwrap();
    assert_eq!(device2_conditions.len(), 2);

    match &device2_conditions[0] {
        InitialConditionItem::TestSequenceRef { test_sequence } => {
            assert_eq!(test_sequence.id, 15);
            assert_eq!(test_sequence.step, "s2");
        }
        _ => panic!("Expected TestSequenceRef variant at index 0"),
    }
    match &device2_conditions[1] {
        InitialConditionItem::RefItem { reference } => assert_eq!(reference, "ref-456"),
        _ => panic!("Expected RefItem variant at index 1"),
    }

    // Roundtrip test
    let serialized = serde_json::to_string(&ic).unwrap();
    let deserialized: InitialConditions = serde_json::from_str(&serialized).unwrap();
    assert_eq!(ic, deserialized);
}

#[test]
fn test_initial_conditions_empty_include_serialization() {
    let ic = InitialConditions {
        include: Some(vec![]),
        devices: HashMap::new(),
    };

    // Even though include is Some([]), it should not be empty
    assert!(!ic.is_empty());

    let yaml = serde_yaml::to_string(&ic).unwrap();
    let deserialized: InitialConditions = serde_yaml::from_str(&yaml).unwrap();
    assert_eq!(ic, deserialized);
}

#[test]
fn test_initial_conditions_complex_mixed_roundtrip_yaml() {
    let mut ic = InitialConditions {
        include: Some(vec![
            IncludeRef {
                id: "TC001".to_string(),
                test_sequence: None,
            },
            IncludeRef {
                id: "TC002".to_string(),
                test_sequence: Some("1".to_string()),
            },
        ]),
        devices: HashMap::new(),
    };
    ic.devices.insert(
        "device1".to_string(),
        vec![
            InitialConditionItem::String("Initial setup".to_string()),
            InitialConditionItem::RefItem {
                reference: "ref-init".to_string(),
            },
            InitialConditionItem::TestSequenceRef {
                test_sequence: testcase_models::TestSequenceRefTarget {
                    id: 1,
                    step: "setup".to_string(),
                },
            },
        ],
    );
    ic.devices.insert(
        "device2".to_string(),
        vec![
            InitialConditionItem::TestSequenceRef {
                test_sequence: testcase_models::TestSequenceRefTarget {
                    id: 2,
                    step: "config".to_string(),
                },
            },
            InitialConditionItem::String("Ready state".to_string()),
        ],
    );

    let yaml = serde_yaml::to_string(&ic).unwrap();
    let deserialized: InitialConditions = serde_yaml::from_str(&yaml).unwrap();
    assert_eq!(ic, deserialized);

    // Verify include is preserved
    assert!(deserialized.include.is_some());
    let include = deserialized.include.unwrap();
    assert_eq!(include.len(), 2);

    // Verify devices are preserved
    assert_eq!(deserialized.devices.len(), 2);
}

#[test]
fn test_initial_conditions_complex_mixed_roundtrip_json() {
    let mut ic = InitialConditions {
        include: Some(vec![IncludeRef {
            id: "TC100".to_string(),
            test_sequence: Some("5".to_string()),
        }]),
        devices: HashMap::new(),
    };
    ic.devices.insert(
        "device_a".to_string(),
        vec![
            InitialConditionItem::String("condition A".to_string()),
            InitialConditionItem::RefItem {
                reference: "ref-a".to_string(),
            },
        ],
    );
    ic.devices.insert(
        "device_b".to_string(),
        vec![InitialConditionItem::TestSequenceRef {
            test_sequence: testcase_models::TestSequenceRefTarget {
                id: 100,
                step: "init".to_string(),
            },
        }],
    );

    let json = serde_json::to_string(&ic).unwrap();
    let deserialized: InitialConditions = serde_json::from_str(&json).unwrap();
    assert_eq!(ic, deserialized);

    // Verify include is preserved
    assert!(deserialized.include.is_some());
    let include = deserialized.include.unwrap();
    assert_eq!(include.len(), 1);
    assert_eq!(include[0].id, "TC100");

    // Verify devices are preserved
    assert_eq!(deserialized.devices.len(), 2);
}

// ===== DependencyValidator Edge Cases =====

#[test]
fn test_dependency_validator_ref_collection_with_empty_test_sequences() {
    let mut validator = testcase_manager::dependency_validator::DependencyValidator::new();

    let test_case = TestCase::new(
        "REQ001".to_string(),
        1,
        1,
        "TC_EMPTY".to_string(),
        "Test case with no sequences".to_string(),
    );

    validator.collect_definitions(&PathBuf::from("empty.yaml"), &test_case);

    let errors = validator.validate_references(&PathBuf::from("empty.yaml"), &test_case);
    assert_eq!(
        errors.len(),
        0,
        "Empty test sequences should not produce validation errors"
    );
}

#[test]
fn test_dependency_validator_nested_includes_three_levels() {
    let mut validator = testcase_manager::dependency_validator::DependencyValidator::new();

    let test_case_c = TestCase::new(
        "REQ001".to_string(),
        1,
        1,
        "TC_C".to_string(),
        "Test C".to_string(),
    );

    let mut test_case_b = TestCase::new(
        "REQ001".to_string(),
        1,
        1,
        "TC_B".to_string(),
        "Test B".to_string(),
    );
    test_case_b.general_initial_conditions = InitialConditions {
        include: Some(vec![IncludeRef {
            id: "TC_C".to_string(),
            test_sequence: None,
        }]),
        ..Default::default()
    };

    let mut test_case_a = TestCase::new(
        "REQ001".to_string(),
        1,
        1,
        "TC_A".to_string(),
        "Test A".to_string(),
    );
    test_case_a.general_initial_conditions = InitialConditions {
        include: Some(vec![IncludeRef {
            id: "TC_B".to_string(),
            test_sequence: None,
        }]),
        ..Default::default()
    };

    validator.collect_definitions(&PathBuf::from("c.yaml"), &test_case_c);
    validator.collect_definitions(&PathBuf::from("b.yaml"), &test_case_b);
    validator.collect_definitions(&PathBuf::from("a.yaml"), &test_case_a);

    let errors_a = validator.validate_references(&PathBuf::from("a.yaml"), &test_case_a);
    let errors_b = validator.validate_references(&PathBuf::from("b.yaml"), &test_case_b);
    let errors_c = validator.validate_references(&PathBuf::from("c.yaml"), &test_case_c);

    assert_eq!(
        errors_a.len(),
        0,
        "Nested includes (A->B->C) should validate correctly for A"
    );
    assert_eq!(
        errors_b.len(),
        0,
        "Nested includes (A->B->C) should validate correctly for B"
    );
    assert_eq!(
        errors_c.len(),
        0,
        "Nested includes (A->B->C) should validate correctly for C"
    );
}

#[test]
fn test_dependency_validator_multiple_refs_same_name_different_test_cases() {
    let mut validator = testcase_manager::dependency_validator::DependencyValidator::new();

    let mut test_case1 = TestCase::new(
        "REQ001".to_string(),
        1,
        1,
        "TC001".to_string(),
        "Test 1".to_string(),
    );
    let mut test_sequence1 = TestSequence::new(1, "Seq1".to_string(), "Desc1".to_string());
    test_sequence1.reference = Some("common-ref".to_string());
    test_case1.test_sequences.push(test_sequence1);

    let mut test_case2 = TestCase::new(
        "REQ002".to_string(),
        1,
        1,
        "TC002".to_string(),
        "Test 2".to_string(),
    );
    let mut test_sequence2 = TestSequence::new(1, "Seq1".to_string(), "Desc1".to_string());
    test_sequence2.reference = Some("common-ref".to_string());
    test_case2.test_sequences.push(test_sequence2);

    let mut test_case3 = TestCase::new(
        "REQ003".to_string(),
        1,
        1,
        "TC003".to_string(),
        "Test 3".to_string(),
    );
    let mut test_sequence3 = TestSequence::new(1, "Seq1".to_string(), "Desc1".to_string());
    test_sequence3.reference = Some("common-ref".to_string());
    test_case3.test_sequences.push(test_sequence3);

    validator.collect_definitions(&PathBuf::from("test1.yaml"), &test_case1);
    validator.collect_definitions(&PathBuf::from("test2.yaml"), &test_case2);
    validator.collect_definitions(&PathBuf::from("test3.yaml"), &test_case3);

    let mut test_case_user = TestCase::new(
        "REQ004".to_string(),
        1,
        1,
        "TC_USER".to_string(),
        "Test User".to_string(),
    );
    let mut initial_conditions = InitialConditions::default();
    initial_conditions.devices.insert(
        "device1".to_string(),
        vec![InitialConditionItem::RefItem {
            reference: "common-ref".to_string(),
        }],
    );
    test_case_user.initial_conditions = initial_conditions;

    let errors = validator.validate_references(&PathBuf::from("test_user.yaml"), &test_case_user);
    assert_eq!(
        errors.len(),
        0,
        "Multiple test cases can define the same ref name"
    );
}

#[test]
fn test_dependency_validator_step_refs_without_sequence_refs() {
    let mut validator = testcase_manager::dependency_validator::DependencyValidator::new();

    let mut test_case_provider = TestCase::new(
        "REQ001".to_string(),
        1,
        1,
        "TC_PROVIDER".to_string(),
        "Provider".to_string(),
    );
    let mut test_sequence = TestSequence::new(1, "Seq1".to_string(), "Desc1".to_string());
    let mut step1 = Step::new(
        1,
        "Step 1".to_string(),
        "cmd1".to_string(),
        "0".to_string(),
        "output1".to_string(),
    );
    step1.reference = Some("step-ref-only".to_string());
    test_sequence.steps.push(step1);

    let mut step2 = Step::new(
        2,
        "Step 2".to_string(),
        "cmd2".to_string(),
        "0".to_string(),
        "output2".to_string(),
    );
    step2.reference = Some("another-step-ref".to_string());
    test_sequence.steps.push(step2);
    test_case_provider.test_sequences.push(test_sequence);

    validator.collect_definitions(&PathBuf::from("provider.yaml"), &test_case_provider);

    let mut test_case_consumer = TestCase::new(
        "REQ002".to_string(),
        1,
        1,
        "TC_CONSUMER".to_string(),
        "Consumer".to_string(),
    );
    let mut initial_conditions = InitialConditions::default();
    initial_conditions.devices.insert(
        "device1".to_string(),
        vec![
            InitialConditionItem::RefItem {
                reference: "step-ref-only".to_string(),
            },
            InitialConditionItem::RefItem {
                reference: "another-step-ref".to_string(),
            },
        ],
    );
    test_case_consumer.initial_conditions = initial_conditions;

    let errors =
        validator.validate_references(&PathBuf::from("consumer.yaml"), &test_case_consumer);
    assert_eq!(
        errors.len(),
        0,
        "Step references without sequence references should be valid"
    );
}

#[test]
fn test_dependency_validator_performance_with_100_test_cases() {
    use std::time::Instant;

    let mut validator = testcase_manager::dependency_validator::DependencyValidator::new();
    let mut all_test_cases = Vec::new();

    for i in 0..100 {
        let mut test_case = TestCase::new(
            format!("REQ{:03}", i),
            i,
            i,
            format!("TC{:03}", i),
            format!("Test case {}", i),
        );

        for j in 0..5 {
            let mut test_sequence =
                TestSequence::new(j + 1, format!("Seq{}", j + 1), format!("Desc{}", j + 1));
            test_sequence.reference = Some(format!("ref-{}-{}", i, j));

            for k in 0..3 {
                let mut step = Step::new(
                    k + 1,
                    format!("Step {}", k + 1),
                    format!("cmd{}", k + 1),
                    "0".to_string(),
                    format!("output{}", k + 1),
                );
                step.reference = Some(format!("step-ref-{}-{}-{}", i, j, k));
                test_sequence.steps.push(step);
            }

            test_case.test_sequences.push(test_sequence);
        }

        if i > 0 {
            test_case.general_initial_conditions = InitialConditions {
                include: Some(vec![IncludeRef {
                    id: format!("TC{:03}", i - 1),
                    test_sequence: None,
                }]),
                ..Default::default()
            };

            let mut initial_conditions = InitialConditions::default();
            initial_conditions.devices.insert(
                "device1".to_string(),
                vec![InitialConditionItem::RefItem {
                    reference: format!("ref-{}-0", i - 1),
                }],
            );
            test_case.initial_conditions = initial_conditions;
        }

        all_test_cases.push((PathBuf::from(format!("test{:03}.yaml", i)), test_case));
    }

    let start_collect = Instant::now();
    for (path, test_case) in &all_test_cases {
        validator.collect_definitions(path, test_case);
    }
    let collect_duration = start_collect.elapsed();

    let start_validate = Instant::now();
    let mut all_errors = Vec::new();
    for (path, test_case) in &all_test_cases {
        let errors = validator.validate_references(path, test_case);
        all_errors.extend(errors);
    }
    let validate_duration = start_validate.elapsed();

    assert_eq!(
        all_errors.len(),
        0,
        "All 100 test cases should validate without errors"
    );

    assert!(
        collect_duration.as_millis() < 1000,
        "Collection should complete in under 1 second (took {:?}ms)",
        collect_duration.as_millis()
    );
    assert!(
        validate_duration.as_millis() < 1000,
        "Validation should complete in under 1 second (took {:?}ms)",
        validate_duration.as_millis()
    );
}

#[test]
fn test_dependency_validator_cross_file_validation_with_100_test_cases() {
    use std::time::Instant;

    let mut all_test_cases = Vec::new();

    for i in 0..100 {
        let mut test_case = TestCase::new(
            format!("REQ{:03}", i),
            i,
            i,
            format!("TC{:03}", i),
            format!("Test case {}", i),
        );

        for j in 0..3 {
            let mut test_sequence =
                TestSequence::new(j + 1, format!("Seq{}", j + 1), format!("Desc{}", j + 1));
            test_sequence.reference = Some(format!("ref-{}-{}", i, j));
            test_case.test_sequences.push(test_sequence);
        }

        if i > 0 {
            test_case.general_initial_conditions = InitialConditions {
                include: Some(vec![IncludeRef {
                    id: format!("TC{:03}", i - 1),
                    test_sequence: None,
                }]),
                ..Default::default()
            };
        }

        all_test_cases.push((PathBuf::from(format!("test{:03}.yaml", i)), test_case));
    }

    let start = Instant::now();
    let result =
        testcase_manager::dependency_validator::validate_cross_file_dependencies(&all_test_cases);
    let duration = start.elapsed();

    assert!(
        result.is_ok(),
        "Cross-file validation with 100 test cases should succeed"
    );

    assert!(
        duration.as_millis() < 2000,
        "Cross-file validation should complete in under 2 seconds (took {:?}ms)",
        duration.as_millis()
    );
}

// ===== validate-yaml Integration Tests =====

use std::process::Command;

fn get_validate_yaml_binary_path() -> PathBuf {
    // CARGO_MANIFEST_DIR points to crates/testcase-manager, so go up to workspace root
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push(".."); // Go to crates/
    path.push(".."); // Go to workspace root
    path.push("target");
    path.push("debug");
    path.push("validate-yaml");
    path
}

fn get_schema_path() -> PathBuf {
    // CARGO_MANIFEST_DIR points to crates/testcase-manager, so go up to workspace root
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push(".."); // Go to crates/
    path.push(".."); // Go to workspace root
    path.push("schemas");
    path.push("tcms-schemas");
    path.push("tcms");
    path.push("test-case-main.schema.json");
    path
}

#[test]
fn test_validate_yaml_with_cross_file_dependencies_success() {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp directory");
    let temp_path = temp_dir.path();

    let yaml1_path = temp_path.join("tc1.yaml");
    let yaml1_content = r#"
requirement: "REQ001"
item: 1
tc: 1
id: "TC001"
description: "Test case 1 - defines references"
general_initial_conditions:
  system:
    - "System is ready"
initial_conditions:
  device1:
    - "Device ready"
test_sequences:
  - id: 1
    ref: "ref-seq-001"
    name: "Sequence 1"
    description: "Defines ref-seq-001"
    initial_conditions:
      device1:
        - "Ready"
    steps:
      - step: 1
        ref: "ref-step-001"
        description: "Step 1"
        command: "echo test"
        expected:
          result: "0"
          output: "test"
"#;
    fs::write(&yaml1_path, yaml1_content).expect("Failed to write yaml1");

    let yaml2_path = temp_path.join("tc2.yaml");
    let yaml2_content = r#"
requirement: "REQ002"
item: 1
tc: 1
id: "TC002"
description: "Test case 2 - references TC001"
general_initial_conditions:
  include:
    - id: "TC001"
  device1:
    - "Device ready"
initial_conditions:
  device1:
    - ref: "ref-seq-001"
    - ref: "ref-step-001"
test_sequences:
  - id: 1
    name: "Sequence 1"
    description: "Uses refs from TC001"
    initial_conditions:
      device1:
        - "Ready"
    steps:
      - step: 1
        description: "Step 1"
        command: "echo test"
        expected:
          result: "0"
          output: "test"
"#;
    fs::write(&yaml2_path, yaml2_content).expect("Failed to write yaml2");

    let binary_path = get_validate_yaml_binary_path();
    let schema_path = get_schema_path();

    let output = Command::new(&binary_path)
        .arg(&yaml1_path)
        .arg(&yaml2_path)
        .arg("--schema")
        .arg(&schema_path)
        .output()
        .expect("Failed to execute validate-yaml");

    assert!(
        output.status.success(),
        "validate-yaml should succeed when all dependencies are resolved.\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("tc1.yaml") && stdout.contains("tc2.yaml"),
        "Output should mention both files"
    );
}

#[test]
fn test_validate_yaml_with_unresolved_refs_failure() {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp directory");
    let temp_path = temp_dir.path();

    let yaml1_path = temp_path.join("tc1.yaml");
    let yaml1_content = r#"
requirement: "REQ001"
item: 1
tc: 1
id: "TC001"
description: "Test case 1"
general_initial_conditions:
  system:
    - "System is ready"
initial_conditions:
  device1:
    - "Device ready"
test_sequences:
  - id: 1
    name: "Sequence 1"
    description: "No refs defined"
    initial_conditions:
      device1:
        - "Ready"
    steps:
      - step: 1
        description: "Step 1"
        command: "echo test"
        expected:
          result: "0"
          output: "test"
"#;
    fs::write(&yaml1_path, yaml1_content).expect("Failed to write yaml1");

    let yaml2_path = temp_path.join("tc2.yaml");
    let yaml2_content = r#"
requirement: "REQ002"
item: 1
tc: 1
id: "TC002"
description: "Test case 2 - references undefined refs"
general_initial_conditions:
  device1:
    - "Device ready"
initial_conditions:
  device1:
    - ref: "undefined-ref-123"
    - ref: "another-undefined-ref"
test_sequences:
  - id: 1
    name: "Sequence 1"
    description: "Uses undefined refs"
    initial_conditions:
      device1:
        - "Ready"
    steps:
      - step: 1
        description: "Step 1"
        command: "echo test"
        expected:
          result: "0"
          output: "test"
"#;
    fs::write(&yaml2_path, yaml2_content).expect("Failed to write yaml2");

    let binary_path = get_validate_yaml_binary_path();
    let schema_path = get_schema_path();

    let output = Command::new(&binary_path)
        .arg(&yaml1_path)
        .arg(&yaml2_path)
        .arg("--schema")
        .arg(&schema_path)
        .output()
        .expect("Failed to execute validate-yaml");

    assert!(
        !output.status.success(),
        "validate-yaml should fail when references are unresolved"
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("undefined-ref-123"),
        "Output should mention the unresolved ref 'undefined-ref-123'"
    );
    assert!(
        stdout.contains("another-undefined-ref"),
        "Output should mention the unresolved ref 'another-undefined-ref'"
    );
    assert!(
        stdout.contains("Unresolved reference"),
        "Output should indicate these are unresolved references"
    );
}

#[test]
fn test_validate_yaml_with_unresolved_test_case_id_failure() {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp directory");
    let temp_path = temp_dir.path();

    let yaml1_path = temp_path.join("tc1.yaml");
    let yaml1_content = r#"
requirement: "REQ001"
item: 1
tc: 1
id: "TC001"
description: "Test case 1"
general_initial_conditions:
  system:
    - "System is ready"
initial_conditions:
  device1:
    - "Device ready"
test_sequences:
  - id: 1
    name: "Sequence 1"
    description: "Sequence"
    initial_conditions:
      device1:
        - "Ready"
    steps:
      - step: 1
        description: "Step 1"
        command: "echo test"
        expected:
          result: "0"
          output: "test"
"#;
    fs::write(&yaml1_path, yaml1_content).expect("Failed to write yaml1");

    let yaml2_path = temp_path.join("tc2.yaml");
    let yaml2_content = r#"
requirement: "REQ002"
item: 1
tc: 1
id: "TC002"
description: "Test case 2 - references undefined test case"
general_initial_conditions:
  include:
    - id: "TC_UNDEFINED_999"
  device1:
    - "Device ready"
initial_conditions:
  include:
    - id: "TC_ANOTHER_UNDEFINED"
  device1:
    - "Device ready"
test_sequences:
  - id: 1
    name: "Sequence 1"
    description: "Sequence"
    initial_conditions:
      device1:
        - "Ready"
    steps:
      - step: 1
        description: "Step 1"
        command: "echo test"
        expected:
          result: "0"
          output: "test"
"#;
    fs::write(&yaml2_path, yaml2_content).expect("Failed to write yaml2");

    let binary_path = get_validate_yaml_binary_path();
    let schema_path = get_schema_path();

    let output = Command::new(&binary_path)
        .arg(&yaml1_path)
        .arg(&yaml2_path)
        .arg("--schema")
        .arg(&schema_path)
        .output()
        .expect("Failed to execute validate-yaml");

    assert!(
        !output.status.success(),
        "validate-yaml should fail when test case IDs are unresolved"
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("TC_UNDEFINED_999"),
        "Output should mention the unresolved test case ID 'TC_UNDEFINED_999'"
    );
    assert!(
        stdout.contains("TC_ANOTHER_UNDEFINED"),
        "Output should mention the unresolved test case ID 'TC_ANOTHER_UNDEFINED'"
    );
    assert!(
        stdout.contains("Unresolved test case ID"),
        "Output should indicate these are unresolved test case IDs"
    );
}

#[test]
fn test_validate_yaml_with_mixed_errors() {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp directory");
    let temp_path = temp_dir.path();

    let yaml1_path = temp_path.join("tc1.yaml");
    let yaml1_content = r#"
requirement: "REQ001"
item: 1
tc: 1
id: "TC001"
description: "Test case 1 - defines one ref"
general_initial_conditions:
  system:
    - "System is ready"
initial_conditions:
  device1:
    - "Device ready"
test_sequences:
  - id: 1
    ref: "valid-ref-001"
    name: "Sequence 1"
    description: "Defines only one ref"
    initial_conditions:
      device1:
        - "Ready"
    steps:
      - step: 1
        description: "Step 1"
        command: "echo test"
        expected:
          result: "0"
          output: "test"
"#;
    fs::write(&yaml1_path, yaml1_content).expect("Failed to write yaml1");

    let yaml2_path = temp_path.join("tc2.yaml");
    let yaml2_content = r#"
requirement: "REQ002"
item: 1
tc: 1
id: "TC002"
description: "Test case 2 - has both valid and invalid references"
general_initial_conditions:
  include:
    - id: "TC001"
    - id: "TC_NONEXISTENT"
  device1:
    - "Device ready"
initial_conditions:
  device1:
    - ref: "valid-ref-001"
    - ref: "invalid-ref-999"
test_sequences:
  - id: 1
    name: "Sequence 1"
    description: "Mixed refs"
    initial_conditions:
      device1:
        - "Ready"
    steps:
      - step: 1
        description: "Step 1"
        command: "echo test"
        expected:
          result: "0"
          output: "test"
"#;
    fs::write(&yaml2_path, yaml2_content).expect("Failed to write yaml2");

    let binary_path = get_validate_yaml_binary_path();
    let schema_path = get_schema_path();

    let output = Command::new(&binary_path)
        .arg(&yaml1_path)
        .arg(&yaml2_path)
        .arg("--schema")
        .arg(&schema_path)
        .output()
        .expect("Failed to execute validate-yaml");

    assert!(
        !output.status.success(),
        "validate-yaml should fail when some references are unresolved"
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("TC_NONEXISTENT"),
        "Output should mention the unresolved test case ID"
    );
    assert!(
        stdout.contains("invalid-ref-999"),
        "Output should mention the unresolved ref"
    );
    assert!(
        !stdout.contains("valid-ref-001") || !stdout.contains("Unresolved"),
        "Valid references should not be reported as errors"
    );
}

#[test]
fn test_validate_yaml_single_file_no_dependency_validation() {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp directory");
    let temp_path = temp_dir.path();

    let yaml1_path = temp_path.join("tc1.yaml");
    let yaml1_content = r#"
requirement: "REQ001"
item: 1
tc: 1
id: "TC001"
description: "Single test case"
general_initial_conditions:
  system:
    - "System is ready"
initial_conditions:
  device1:
    - "Device ready"
test_sequences:
  - id: 1
    name: "Sequence 1"
    description: "Simple sequence"
    initial_conditions:
      device1:
        - "Ready"
    steps:
      - step: 1
        description: "Step 1"
        command: "echo test"
        expected:
          result: "0"
          output: "test"
"#;
    fs::write(&yaml1_path, yaml1_content).expect("Failed to write yaml1");

    let binary_path = get_validate_yaml_binary_path();
    let schema_path = get_schema_path();

    let output = Command::new(&binary_path)
        .arg(&yaml1_path)
        .arg("--schema")
        .arg(&schema_path)
        .output()
        .expect("Failed to execute validate-yaml");

    assert!(
        output.status.success(),
        "validate-yaml should succeed for single valid file.\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("tc1.yaml"),
        "Output should mention the validated file"
    );
}

#[test]
fn test_validate_yaml_three_files_with_chained_dependencies() {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp directory");
    let temp_path = temp_dir.path();

    let yaml1_path = temp_path.join("tc1.yaml");
    let yaml1_content = r#"
requirement: "REQ001"
item: 1
tc: 1
id: "TC001"
description: "Base test case"
general_initial_conditions:
  system:
    - "System is ready"
initial_conditions:
  device1:
    - "Device ready"
test_sequences:
  - id: 1
    ref: "ref-base"
    name: "Base Sequence"
    description: "Base sequence"
    initial_conditions:
      device1:
        - "Ready"
    steps:
      - step: 1
        ref: "ref-base-step"
        description: "Base step"
        command: "echo base"
        expected:
          result: "0"
          output: "base"
"#;
    fs::write(&yaml1_path, yaml1_content).expect("Failed to write yaml1");

    let yaml2_path = temp_path.join("tc2.yaml");
    let yaml2_content = r#"
requirement: "REQ002"
item: 1
tc: 1
id: "TC002"
description: "Middle test case - uses TC001, provides new refs"
general_initial_conditions:
  include:
    - id: "TC001"
  device1:
    - ref: "ref-base"
initial_conditions:
  device1:
    - ref: "ref-base-step"
test_sequences:
  - id: 1
    ref: "ref-middle"
    name: "Middle Sequence"
    description: "Middle sequence"
    initial_conditions:
      device1:
        - "Ready"
    steps:
      - step: 1
        ref: "ref-middle-step"
        description: "Middle step"
        command: "echo middle"
        expected:
          result: "0"
          output: "middle"
"#;
    fs::write(&yaml2_path, yaml2_content).expect("Failed to write yaml2");

    let yaml3_path = temp_path.join("tc3.yaml");
    let yaml3_content = r#"
requirement: "REQ003"
item: 1
tc: 1
id: "TC003"
description: "Final test case - uses TC002 and all refs"
general_initial_conditions:
  include:
    - id: "TC002"
  device1:
    - ref: "ref-middle"
initial_conditions:
  device1:
    - ref: "ref-base"
    - ref: "ref-base-step"
    - ref: "ref-middle-step"
test_sequences:
  - id: 1
    name: "Final Sequence"
    description: "Final sequence"
    initial_conditions:
      device1:
        - "Ready"
    steps:
      - step: 1
        description: "Final step"
        command: "echo final"
        expected:
          result: "0"
          output: "final"
"#;
    fs::write(&yaml3_path, yaml3_content).expect("Failed to write yaml3");

    let binary_path = get_validate_yaml_binary_path();
    let schema_path = get_schema_path();

    let output = Command::new(&binary_path)
        .arg(&yaml1_path)
        .arg(&yaml2_path)
        .arg(&yaml3_path)
        .arg("--schema")
        .arg(&schema_path)
        .output()
        .expect("Failed to execute validate-yaml");

    assert!(
        output.status.success(),
        "validate-yaml should succeed with chained dependencies.\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("tc1.yaml") && stdout.contains("tc2.yaml") && stdout.contains("tc3.yaml"),
        "Output should mention all three files"
    );
}

#[test]
fn test_validate_yaml_error_format_includes_file_location() {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp directory");
    let temp_path = temp_dir.path();

    let yaml1_path = temp_path.join("first.yaml");
    let yaml1_content = r#"
requirement: "REQ001"
item: 1
tc: 1
id: "TC001"
description: "First test case"
general_initial_conditions:
  system:
    - "System is ready"
initial_conditions:
  device1:
    - "Device ready"
test_sequences:
  - id: 1
    name: "Sequence"
    description: "Sequence"
    initial_conditions:
      device1:
        - "Ready"
    steps:
      - step: 1
        description: "Step"
        command: "echo test"
        expected:
          result: "0"
          output: "test"
"#;
    fs::write(&yaml1_path, yaml1_content).expect("Failed to write yaml1");

    let yaml2_path = temp_path.join("second.yaml");
    let yaml2_content = r#"
requirement: "REQ002"
item: 1
tc: 1
id: "TC002"
description: "Second test case with error"
general_initial_conditions:
  device1:
    - ref: "undefined-in-general"
initial_conditions:
  include:
    - id: "UNDEFINED_TC_ID"
  device2:
    - ref: "undefined-in-initial"
test_sequences:
  - id: 1
    name: "Sequence"
    description: "Sequence"
    initial_conditions:
      device3:
        - ref: "undefined-in-sequence"
    steps:
      - step: 1
        description: "Step"
        command: "echo test"
        expected:
          result: "0"
          output: "test"
"#;
    fs::write(&yaml2_path, yaml2_content).expect("Failed to write yaml2");

    let binary_path = get_validate_yaml_binary_path();
    let schema_path = get_schema_path();

    let output = Command::new(&binary_path)
        .arg(&yaml1_path)
        .arg(&yaml2_path)
        .arg("--schema")
        .arg(&schema_path)
        .output()
        .expect("Failed to execute validate-yaml");

    assert!(
        !output.status.success(),
        "validate-yaml should fail with multiple errors"
    );

    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(
        stdout.contains("second.yaml") && stdout.contains("general_initial_conditions"),
        "Output should include file name and location for general_initial_conditions error"
    );
    assert!(
        stdout.contains("undefined-in-general"),
        "Output should mention the undefined ref in general_initial_conditions"
    );
    assert!(
        stdout.contains("initial_conditions"),
        "Output should include location for initial_conditions error"
    );
    assert!(
        stdout.contains("UNDEFINED_TC_ID"),
        "Output should mention the undefined test case ID"
    );
    assert!(
        stdout.contains("undefined-in-initial"),
        "Output should mention the undefined ref in initial_conditions"
    );
    assert!(
        stdout.contains("test_sequences") && stdout.contains("initial_conditions"),
        "Output should include location for test sequence initial_conditions error"
    );
    assert!(
        stdout.contains("undefined-in-sequence"),
        "Output should mention the undefined ref in sequence initial_conditions"
    );
}
