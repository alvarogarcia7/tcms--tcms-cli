#!/usr/bin/env python3.14
"""
Unit tests for convert_verification_to_tpdg.py

Tests all major functions and workflows including:
- Test case YAML parsing
- Execution log parsing
- Step result building (Pass/Fail/NotExecuted)
- Verification result construction
- TPDG container format conversion
- File scanning and directory processing
- Error handling
"""

import unittest
import json
import tempfile
import shutil
from pathlib import Path

try:
    import yaml  # noqa: F401
    YAML_AVAILABLE = True
except ImportError:
    YAML_AVAILABLE = False

# Import the module under test
import sys
sys.path.insert(0, str(Path(__file__).parent))
from convert_verification_to_tpdg import (
    parse_test_case_yaml,
    parse_execution_log,
    read_actual_log_file,
    build_step_result,
    build_verification_result_from_files,
    convert_test_case_to_tpdg,
    create_tpdg_container,
    parse_verifier_json,
    scan_test_cases,
    process_test_case_directory,
    transform_step_result
)


class TestParseTestCaseYaml(unittest.TestCase):
    """Tests for parse_test_case_yaml function."""
    
    def setUp(self):
        """Create temporary directory for test files."""
        self.temp_dir = tempfile.mkdtemp()
        self.temp_path = Path(self.temp_dir)
    
    def tearDown(self):
        """Clean up temporary directory."""
        shutil.rmtree(self.temp_dir)
    
    def test_parse_valid_test_case(self):
        """Test parsing a valid test case YAML file."""
        yaml_content = """
type: test_case
id: TC-001
description: Test case 001
requirement: REQ-001
item: ITEM-001
tc: TC-001
test_sequences:
  - id: 1
    name: Sequence 1
    steps:
      - step: 1
        description: Step 1 description
        command: echo "test"
        expected:
          success: true
          result: 0
          output: "test"
"""
        yaml_file = self.temp_path / "test.yaml"
        yaml_file.write_text(yaml_content)
        
        result = parse_test_case_yaml(yaml_file)
        
        self.assertIsNotNone(result)
        self.assertEqual(result['id'], 'TC-001')
        self.assertEqual(result['description'], 'Test case 001')
        self.assertEqual(result['requirement'], 'REQ-001')
        self.assertEqual(result['item'], 'ITEM-001')
        self.assertEqual(result['tc'], 'TC-001')
        self.assertEqual(len(result['test_sequences']), 1)
        self.assertEqual(result['test_sequences'][0]['id'], 1)
        self.assertEqual(result['test_sequences'][0]['name'], 'Sequence 1')
        self.assertEqual(len(result['test_sequences'][0]['steps']), 1)
        self.assertEqual(result['test_sequences'][0]['steps'][0]['step'], 1)
        self.assertEqual(result['test_sequences'][0]['steps'][0]['description'], 'Step 1 description')
    
    def test_parse_non_test_case_type(self):
        """Test that non-test_case types return None."""
        yaml_content = """
type: test_plan
id: TP-001
"""
        yaml_file = self.temp_path / "test_plan.yaml"
        yaml_file.write_text(yaml_content)
        
        result = parse_test_case_yaml(yaml_file)
        self.assertIsNone(result)
    
    def test_parse_missing_optional_fields(self):
        """Test parsing test case with missing optional fields."""
        yaml_content = """
type: test_case
id: TC-002
test_sequences:
  - id: 1
    steps:
      - step: 1
        expected: {}
"""
        yaml_file = self.temp_path / "minimal.yaml"
        yaml_file.write_text(yaml_content)
        
        result = parse_test_case_yaml(yaml_file)
        
        self.assertIsNotNone(result)
        self.assertEqual(result['id'], 'TC-002')
        self.assertEqual(result['description'], '')
        self.assertIsNone(result['requirement'])
        self.assertIsNone(result['item'])
        self.assertIsNone(result['tc'])
    
    def test_parse_invalid_yaml(self):
        """Test handling of invalid YAML."""
        yaml_file = self.temp_path / "invalid.yaml"
        yaml_file.write_text("{ invalid yaml content [")
        
        result = parse_test_case_yaml(yaml_file)
        self.assertIsNone(result)
    
    def test_parse_nonexistent_file(self):
        """Test handling of nonexistent file."""
        yaml_file = self.temp_path / "nonexistent.yaml"
        
        result = parse_test_case_yaml(yaml_file)
        self.assertIsNone(result)
    
    def test_parse_multiple_sequences_and_steps(self):
        """Test parsing test case with multiple sequences and steps."""
        yaml_content = """
type: test_case
id: TC-003
test_sequences:
  - id: 1
    name: Sequence 1
    steps:
      - step: 1
        description: Step 1-1
        expected: {}
      - step: 2
        description: Step 1-2
        expected: {}
  - id: 2
    name: Sequence 2
    steps:
      - step: 1
        description: Step 2-1
        expected: {}
"""
        yaml_file = self.temp_path / "multi.yaml"
        yaml_file.write_text(yaml_content)
        
        result = parse_test_case_yaml(yaml_file)
        
        self.assertIsNotNone(result)
        self.assertEqual(len(result['test_sequences']), 2)
        self.assertEqual(len(result['test_sequences'][0]['steps']), 2)
        self.assertEqual(len(result['test_sequences'][1]['steps']), 1)


class TestParseExecutionLog(unittest.TestCase):
    """Tests for parse_execution_log function."""
    
    def setUp(self):
        """Create temporary directory for test files."""
        self.temp_dir = tempfile.mkdtemp()
        self.temp_path = Path(self.temp_dir)
    
    def tearDown(self):
        """Clean up temporary directory."""
        shutil.rmtree(self.temp_dir)
    
    def test_parse_valid_execution_log(self):
        """Test parsing a valid execution log JSON file."""
        log_data = [
            {
                "test_sequence": 1,
                "step": 1,
                "exit_code": 0,
                "output": "test output",
                "result_verification_pass": True,
                "output_verification_pass": True
            },
            {
                "test_sequence": 1,
                "step": 2,
                "exit_code": 1,
                "output": "error output",
                "result_verification_pass": False,
                "output_verification_pass": False
            }
        ]
        
        log_file = self.temp_path / "execution_log.json"
        log_file.write_text(json.dumps(log_data))
        
        result = parse_execution_log(log_file)
        
        self.assertEqual(len(result), 2)
        self.assertEqual(result[0]['test_sequence'], 1)
        self.assertEqual(result[0]['step'], 1)
        self.assertTrue(result[0]['result_verification_pass'])
    
    def test_parse_empty_log(self):
        """Test parsing an empty log file."""
        log_file = self.temp_path / "empty_log.json"
        log_file.write_text("[]")
        
        result = parse_execution_log(log_file)
        self.assertEqual(result, [])
    
    def test_parse_invalid_json(self):
        """Test handling of invalid JSON."""
        log_file = self.temp_path / "invalid.json"
        log_file.write_text("{ invalid json")
        
        result = parse_execution_log(log_file)
        self.assertEqual(result, [])
    
    def test_parse_non_list_json(self):
        """Test handling of non-list JSON."""
        log_file = self.temp_path / "object.json"
        log_file.write_text('{"key": "value"}')
        
        result = parse_execution_log(log_file)
        self.assertEqual(result, [])


class TestReadActualLogFile(unittest.TestCase):
    """Tests for read_actual_log_file function."""
    
    def setUp(self):
        """Create temporary directory for test files."""
        self.temp_dir = tempfile.mkdtemp()
        self.temp_path = Path(self.temp_dir)
    
    def tearDown(self):
        """Clean up temporary directory."""
        shutil.rmtree(self.temp_dir)
    
    def test_read_existing_log_file(self):
        """Test reading an existing actual log file."""
        log_content = "Actual output from test execution"
        log_file = self.temp_path / "TC-001_sequence-1_step-1.actual.log"
        log_file.write_text(log_content)
        
        result = read_actual_log_file(self.temp_path, "TC-001", 1, 1)
        
        self.assertEqual(result, log_content)
    
    def test_read_nonexistent_log_file(self):
        """Test reading a nonexistent log file."""
        result = read_actual_log_file(self.temp_path, "TC-999", 1, 1)
        self.assertIsNone(result)
    
    def test_read_log_with_unicode(self):
        """Test reading log file with unicode content."""
        log_content = "Test with unicode: café, 日本語, 🎉"
        log_file = self.temp_path / "TC-002_sequence-1_step-1.actual.log"
        log_file.write_text(log_content, encoding='utf-8')
        
        result = read_actual_log_file(self.temp_path, "TC-002", 1, 1)
        self.assertEqual(result, log_content)


class TestBuildStepResult(unittest.TestCase):
    """Tests for build_step_result function."""
    
    def setUp(self):
        """Create test data."""
        self.test_case = {
            'id': 'TC-001',
            'test_sequences': [
                {
                    'id': 1,
                    'name': 'Sequence 1',
                    'steps': [
                        {
                            'step': 1,
                            'description': 'Test step 1',
                            'expected': {
                                'success': True,
                                'result': 0,
                                'output': 'expected output'
                            }
                        }
                    ]
                }
            ]
        }
    
    def test_build_pass_step_result(self):
        """Test building a Pass step result."""
        execution_entry = {
            'test_sequence': 1,
            'step': 1,
            'exit_code': 0,
            'output': 'actual output',
            'result_verification_pass': True,
            'output_verification_pass': True
        }
        
        result = build_step_result(self.test_case, 1, 1, execution_entry, None)
        
        self.assertIn('Pass', result)
        self.assertEqual(result['Pass']['step'], 1)
        self.assertEqual(result['Pass']['description'], 'Test step 1')
    
    def test_build_fail_step_result(self):
        """Test building a Fail step result."""
        execution_entry = {
            'test_sequence': 1,
            'step': 1,
            'exit_code': 1,
            'output': 'error output',
            'result_verification_pass': False,
            'output_verification_pass': False
        }
        
        result = build_step_result(self.test_case, 1, 1, execution_entry, None)
        
        self.assertIn('Fail', result)
        self.assertEqual(result['Fail']['step'], 1)
        self.assertEqual(result['Fail']['description'], 'Test step 1')
        self.assertEqual(result['Fail']['expected']['result'], '0')
        self.assertEqual(result['Fail']['actual_result'], '1')
        self.assertEqual(result['Fail']['actual_output'], 'error output')
        self.assertIn('result verification failed', result['Fail']['reason'])
    
    def test_build_fail_result_only(self):
        """Test building a Fail result when only result verification fails."""
        execution_entry = {
            'test_sequence': 1,
            'step': 1,
            'exit_code': 1,
            'output': 'actual output',
            'result_verification_pass': False,
            'output_verification_pass': True
        }
        
        result = build_step_result(self.test_case, 1, 1, execution_entry, None)
        
        self.assertIn('Fail', result)
        self.assertEqual(result['Fail']['reason'], 'result verification failed')
    
    def test_build_fail_output_only(self):
        """Test building a Fail result when only output verification fails."""
        execution_entry = {
            'test_sequence': 1,
            'step': 1,
            'exit_code': 0,
            'output': 'wrong output',
            'result_verification_pass': True,
            'output_verification_pass': False
        }
        
        result = build_step_result(self.test_case, 1, 1, execution_entry, None)
        
        self.assertIn('Fail', result)
        self.assertEqual(result['Fail']['reason'], 'output verification failed')
    
    def test_build_not_executed_step_result(self):
        """Test building a NotExecuted step result."""
        result = build_step_result(self.test_case, 1, 1, None, None)
        
        self.assertIn('NotExecuted', result)
        self.assertEqual(result['NotExecuted']['step'], 1)
        self.assertEqual(result['NotExecuted']['description'], 'Test step 1')
    
    def test_build_not_executed_undefined_step(self):
        """Test building NotExecuted result for undefined step."""
        result = build_step_result(self.test_case, 1, 999, None, None)
        
        self.assertIn('NotExecuted', result)
        self.assertEqual(result['NotExecuted']['step'], 999)
        self.assertIn('not defined in test case', result['NotExecuted']['description'])
    
    def test_build_fail_with_actual_log_file(self):
        """Test building Fail result with actual log file content."""
        temp_dir = tempfile.mkdtemp()
        temp_path = Path(temp_dir)
        
        try:
            log_content = "Content from actual log file"
            log_file = temp_path / "TC-001_sequence-1_step-1.actual.log"
            log_file.write_text(log_content)
            
            execution_entry = {
                'test_sequence': 1,
                'step': 1,
                'exit_code': 1,
                'output': 'execution log output',
                'result_verification_pass': False,
                'output_verification_pass': False
            }
            
            result = build_step_result(self.test_case, 1, 1, execution_entry, temp_path)
            
            self.assertIn('Fail', result)
            self.assertEqual(result['Fail']['actual_output'], log_content)
        finally:
            shutil.rmtree(temp_dir)


class TestBuildVerificationResultFromFiles(unittest.TestCase):
    """Tests for build_verification_result_from_files function."""
    
    def setUp(self):
        """Create test data."""
        self.test_case = {
            'id': 'TC-001',
            'description': 'Test case description',
            'requirement': 'REQ-001',
            'item': 'ITEM-001',
            'tc': 'TC-001',
            'test_sequences': [
                {
                    'id': 1,
                    'name': 'Sequence 1',
                    'steps': [
                        {
                            'step': 1,
                            'description': 'Step 1',
                            'expected': {'success': True, 'result': 0}
                        },
                        {
                            'step': 2,
                            'description': 'Step 2',
                            'expected': {'success': True, 'result': 0}
                        }
                    ]
                }
            ]
        }
    
    def test_build_all_passed(self):
        """Test building result when all steps pass."""
        execution_log = [
            {
                'test_sequence': 1,
                'step': 1,
                'exit_code': 0,
                'result_verification_pass': True,
                'output_verification_pass': True
            },
            {
                'test_sequence': 1,
                'step': 2,
                'exit_code': 0,
                'result_verification_pass': True,
                'output_verification_pass': True
            }
        ]
        
        result = build_verification_result_from_files(self.test_case, execution_log, None)
        
        self.assertEqual(result['test_case_id'], 'TC-001')
        self.assertEqual(result['total_steps'], 2)
        self.assertEqual(result['passed_steps'], 2)
        self.assertEqual(result['failed_steps'], 0)
        self.assertEqual(result['not_executed_steps'], 0)
        self.assertTrue(result['overall_pass'])
        self.assertTrue(result['sequences'][0]['all_steps_passed'])
    
    def test_build_with_failures(self):
        """Test building result with some failures."""
        execution_log = [
            {
                'test_sequence': 1,
                'step': 1,
                'exit_code': 0,
                'result_verification_pass': True,
                'output_verification_pass': True
            },
            {
                'test_sequence': 1,
                'step': 2,
                'exit_code': 1,
                'result_verification_pass': False,
                'output_verification_pass': False
            }
        ]
        
        result = build_verification_result_from_files(self.test_case, execution_log, None)
        
        self.assertEqual(result['passed_steps'], 1)
        self.assertEqual(result['failed_steps'], 1)
        self.assertEqual(result['not_executed_steps'], 0)
        self.assertFalse(result['overall_pass'])
        self.assertFalse(result['sequences'][0]['all_steps_passed'])
    
    def test_build_with_not_executed(self):
        """Test building result with not executed steps."""
        execution_log = [
            {
                'test_sequence': 1,
                'step': 1,
                'exit_code': 0,
                'result_verification_pass': True,
                'output_verification_pass': True
            }
        ]
        
        result = build_verification_result_from_files(self.test_case, execution_log, None)
        
        self.assertEqual(result['passed_steps'], 1)
        self.assertEqual(result['failed_steps'], 0)
        self.assertEqual(result['not_executed_steps'], 1)
        self.assertFalse(result['overall_pass'])
    
    def test_build_with_optional_metadata(self):
        """Test that optional metadata fields are included."""
        execution_log = []
        
        result = build_verification_result_from_files(self.test_case, execution_log, None)
        
        self.assertEqual(result['requirement'], 'REQ-001')
        self.assertEqual(result['item'], 'ITEM-001')
        self.assertEqual(result['tc'], 'TC-001')
    
    def test_build_without_optional_metadata(self):
        """Test that optional metadata fields are omitted when not present."""
        test_case_minimal = {
            'id': 'TC-002',
            'description': 'Minimal test case',
            'test_sequences': []
        }
        
        result = build_verification_result_from_files(test_case_minimal, [], None)
        
        self.assertNotIn('requirement', result)
        self.assertNotIn('item', result)
        self.assertNotIn('tc', result)


class TestConvertTestCaseToTpdg(unittest.TestCase):
    """Tests for convert_test_case_to_tpdg function."""
    
    def test_convert_basic_test_case(self):
        """Test converting a basic test case to TPDG format."""
        test_case = {
            'test_case_id': 'TC-001',
            'description': 'Test case',
            'sequences': [],
            'total_steps': 5,
            'passed_steps': 3,
            'failed_steps': 1,
            'not_executed_steps': 1,
            'overall_pass': False
        }
        
        result = convert_test_case_to_tpdg(test_case)
        
        self.assertEqual(result['test_case_id'], 'TC-001')
        self.assertEqual(result['description'], 'Test case')
        self.assertEqual(result['total_steps'], 5)
        self.assertEqual(result['passed_steps'], 3)
        self.assertEqual(result['failed_steps'], 1)
        self.assertEqual(result['not_executed_steps'], 1)
        self.assertFalse(result['overall_pass'])
    
    def test_convert_with_optional_metadata(self):
        """Test converting test case with optional metadata."""
        test_case = {
            'test_case_id': 'TC-001',
            'description': 'Test case',
            'requirement': 'REQ-001',
            'item': 'ITEM-001',
            'tc': 'TC-001',
            'sequences': [],
            'total_steps': 0,
            'passed_steps': 0,
            'failed_steps': 0,
            'not_executed_steps': 0,
            'overall_pass': True
        }
        
        result = convert_test_case_to_tpdg(test_case)
        
        self.assertEqual(result['requirement'], 'REQ-001')
        self.assertEqual(result['item'], 'ITEM-001')
        self.assertEqual(result['tc'], 'TC-001')
    
    def test_convert_without_optional_metadata(self):
        """Test that None values are not included in TPDG format."""
        test_case = {
            'test_case_id': 'TC-001',
            'description': 'Test case',
            'requirement': None,
            'item': None,
            'tc': None,
            'sequences': [],
            'total_steps': 0,
            'passed_steps': 0,
            'failed_steps': 0,
            'not_executed_steps': 0,
            'overall_pass': True
        }
        
        result = convert_test_case_to_tpdg(test_case)
        
        self.assertNotIn('requirement', result)
        self.assertNotIn('item', result)
        self.assertNotIn('tc', result)


class TestCreateTpdgContainer(unittest.TestCase):
    """Tests for create_tpdg_container function."""
    
    def test_create_basic_container(self):
        """Test creating a basic TPDG container."""
        test_results = [
            {
                'test_case_id': 'TC-001',
                'description': 'Test 1',
                'sequences': [],
                'total_steps': 2,
                'passed_steps': 2,
                'failed_steps': 0,
                'not_executed_steps': 0,
                'overall_pass': True
            },
            {
                'test_case_id': 'TC-002',
                'description': 'Test 2',
                'sequences': [],
                'total_steps': 3,
                'passed_steps': 2,
                'failed_steps': 1,
                'not_executed_steps': 0,
                'overall_pass': False
            }
        ]
        
        container = create_tpdg_container(test_results)

        self.assertEqual(container['type'], 'test_results_container')
        self.assertEqual(container['schema'], 'tcms/test-results-container.schema.v1.json')
        self.assertEqual(len(container['test_results']), 2)
        self.assertEqual(container['metadata']['total_test_cases'], 2)
        self.assertEqual(container['metadata']['passed_test_cases'], 1)
        self.assertEqual(container['metadata']['failed_test_cases'], 1)
    
    def test_create_with_custom_metadata(self):
        """Test creating container with custom title and project."""
        test_results = []
        
        container = create_tpdg_container(
            test_results,
            title="Custom Title",
            project="Custom Project"
        )
        
        self.assertEqual(container['title'], 'Custom Title')
        self.assertEqual(container['project'], 'Custom Project')
    
    def test_create_empty_container(self):
        """Test creating container with no test results."""
        container = create_tpdg_container([])
        
        self.assertEqual(len(container['test_results']), 0)
        self.assertEqual(container['metadata']['total_test_cases'], 0)
        self.assertEqual(container['metadata']['passed_test_cases'], 0)
        self.assertEqual(container['metadata']['failed_test_cases'], 0)
    
    def test_container_has_required_fields(self):
        """Test that container has all required fields."""
        test_results = []
        container = create_tpdg_container(test_results)
        
        self.assertIn('type', container)
        self.assertIn('schema', container)
        self.assertIn('title', container)
        self.assertIn('project', container)
        self.assertIn('test_date', container)
        self.assertIn('test_results', container)
        self.assertIn('metadata', container)


class TestParseVerifierJson(unittest.TestCase):
    """Tests for parse_verifier_json function."""
    
    def test_parse_container_report(self):
        """Test parsing ContainerReport format."""
        data = {
            'test_results': [
                {'test_case_id': 'TC-001'},
                {'test_case_id': 'TC-002'}
            ]
        }
        
        result = parse_verifier_json(data)
        
        self.assertEqual(len(result), 2)
        self.assertEqual(result[0]['test_case_id'], 'TC-001')
    
    def test_parse_batch_verification_report(self):
        """Test parsing BatchVerificationReport format."""
        data = {
            'test_cases': [
                {'test_case_id': 'TC-001'},
                {'test_case_id': 'TC-002'},
                {'test_case_id': 'TC-003'}
            ]
        }
        
        result = parse_verifier_json(data)
        
        self.assertEqual(len(result), 3)
        self.assertEqual(result[2]['test_case_id'], 'TC-003')
    
    def test_parse_single_test_case(self):
        """Test parsing single TestCaseVerificationResult."""
        data = {
            'test_case_id': 'TC-001',
            'description': 'Single test case',
            'overall_pass': True
        }
        
        result = parse_verifier_json(data)
        
        self.assertEqual(len(result), 1)
        self.assertEqual(result[0]['test_case_id'], 'TC-001')
    
    def test_parse_unknown_format(self):
        """Test handling of unknown JSON format."""
        data = {
            'unknown_field': 'value'
        }
        
        with self.assertRaises(ValueError) as context:
            parse_verifier_json(data)
        
        self.assertIn('Unknown JSON structure', str(context.exception))


class TestScanTestCases(unittest.TestCase):
    """Tests for scan_test_cases function."""
    
    def setUp(self):
        """Create temporary directory structure."""
        self.temp_dir = tempfile.mkdtemp()
        self.temp_path = Path(self.temp_dir)
        
        # Create test directory structure
        (self.temp_path / "subdir").mkdir()
        (self.temp_path / "test1.yaml").touch()
        (self.temp_path / "test2.yml").touch()
        (self.temp_path / "test3.txt").touch()
        (self.temp_path / "subdir" / "test4.yaml").touch()
    
    def tearDown(self):
        """Clean up temporary directory."""
        shutil.rmtree(self.temp_dir)
    
    def test_scan_non_recursive(self):
        """Test scanning without recursion."""
        files = scan_test_cases(self.temp_path, recursive=False)
        
        self.assertEqual(len(files), 2)
        file_names = {f.name for f in files}
        self.assertIn('test1.yaml', file_names)
        self.assertIn('test2.yml', file_names)
        self.assertNotIn('test4.yaml', file_names)
    
    def test_scan_recursive(self):
        """Test scanning with recursion."""
        files = scan_test_cases(self.temp_path, recursive=True)
        
        self.assertEqual(len(files), 3)
        file_names = {f.name for f in files}
        self.assertIn('test1.yaml', file_names)
        self.assertIn('test2.yml', file_names)
        self.assertIn('test4.yaml', file_names)
    
    def test_scan_empty_directory(self):
        """Test scanning empty directory."""
        empty_dir = self.temp_path / "empty"
        empty_dir.mkdir()
        
        files = scan_test_cases(empty_dir, recursive=False)
        self.assertEqual(len(files), 0)


class TestProcessTestCaseDirectory(unittest.TestCase):
    """Tests for process_test_case_directory function."""
    
    def setUp(self):
        """Create temporary directory structure with test files."""
        self.temp_dir = tempfile.mkdtemp()
        self.temp_path = Path(self.temp_dir)
        self.test_case_dir = self.temp_path / "testcases"
        self.logs_dir = self.temp_path / "logs"
        self.test_case_dir.mkdir()
        self.logs_dir.mkdir()
        
        # Create test case YAML
        test_case_yaml = """
type: test_case
id: TC-001
description: Test case 001
test_sequences:
  - id: 1
    name: Sequence 1
    steps:
      - step: 1
        description: Step 1
        expected:
          success: true
          result: 0
"""
        (self.test_case_dir / "TC-001.yaml").write_text(test_case_yaml)
        
        # Create execution log
        execution_log = [
            {
                "test_sequence": 1,
                "step": 1,
                "exit_code": 0,
                "output": "success",
                "result_verification_pass": True,
                "output_verification_pass": True
            }
        ]
        (self.logs_dir / "TC-001_execution_log.json").write_text(json.dumps(execution_log))
    
    def tearDown(self):
        """Clean up temporary directory."""
        shutil.rmtree(self.temp_dir)
    
    def test_process_directory(self):
        """Test processing test case directory."""
        results = process_test_case_directory(
            self.test_case_dir,
            self.logs_dir,
            recursive=False,
            verbose=False
        )
        
        self.assertEqual(len(results), 1)
        self.assertEqual(results[0]['test_case_id'], 'TC-001')
        self.assertEqual(results[0]['passed_steps'], 1)
        self.assertTrue(results[0]['overall_pass'])
    
    def test_process_missing_execution_log(self):
        """Test processing when execution log is missing."""
        # Create another test case without execution log
        test_case_yaml = """
type: test_case
id: TC-002
description: Test case 002
test_sequences:
  - id: 1
    name: Sequence 1
    steps:
      - step: 1
        description: Step 1
        expected: {}
"""
        (self.test_case_dir / "TC-002.yaml").write_text(test_case_yaml)
        
        results = process_test_case_directory(
            self.test_case_dir,
            self.logs_dir,
            recursive=False,
            verbose=False
        )
        
        # Should find TC-001 with log and TC-002 without log
        self.assertEqual(len(results), 2)
        
        tc002 = [r for r in results if r['test_case_id'] == 'TC-002'][0]
        self.assertEqual(tc002['not_executed_steps'], 1)
        self.assertFalse(tc002['overall_pass'])
    
    def test_process_empty_directory(self):
        """Test processing empty directory."""
        empty_dir = self.temp_path / "empty"
        empty_dir.mkdir()
        
        results = process_test_case_directory(
            empty_dir,
            self.logs_dir,
            recursive=False,
            verbose=False
        )
        
        self.assertEqual(len(results), 0)


class TestTransformStepResult(unittest.TestCase):
    """Tests for transform_step_result function."""
    
    def test_transform_internally_tagged_pass(self):
        """Test transforming internally-tagged pass step."""
        step_result = {
            "status": "pass",
            "step": 1,
            "description": "Test step"
        }
        
        result = transform_step_result(step_result)
        
        self.assertIn('Pass', result)
        self.assertEqual(result['Pass']['step'], 1)
        self.assertEqual(result['Pass']['description'], 'Test step')
        self.assertNotIn('status', result['Pass'])
    
    def test_transform_internally_tagged_fail(self):
        """Test transforming internally-tagged fail step."""
        step_result = {
            "status": "fail",
            "step": 2,
            "description": "Failed step",
            "expected": {"result": 0},
            "actual_result": "1"
        }
        
        result = transform_step_result(step_result)
        
        self.assertIn('Fail', result)
        self.assertEqual(result['Fail']['step'], 2)
        self.assertEqual(result['Fail']['description'], 'Failed step')
        self.assertEqual(result['Fail']['expected']['result'], 0)
        self.assertEqual(result['Fail']['actual_result'], '1')
        self.assertNotIn('status', result['Fail'])
    
    def test_transform_internally_tagged_not_executed(self):
        """Test transforming internally-tagged not_executed step."""
        step_result = {
            "status": "not_executed",
            "step": 3,
            "description": "Not executed step"
        }
        
        result = transform_step_result(step_result)
        
        self.assertIn('NotExecuted', result)
        self.assertEqual(result['NotExecuted']['step'], 3)
        self.assertEqual(result['NotExecuted']['description'], 'Not executed step')
        self.assertNotIn('status', result['NotExecuted'])
    
    def test_transform_already_externally_tagged_pass(self):
        """Test that already externally-tagged Pass remains unchanged."""
        step_result = {
            "Pass": {
                "step": 1,
                "description": "Already tagged"
            }
        }
        
        result = transform_step_result(step_result)
        
        self.assertEqual(result, step_result)
    
    def test_transform_already_externally_tagged_fail(self):
        """Test that already externally-tagged Fail remains unchanged."""
        step_result = {
            "Fail": {
                "step": 2,
                "description": "Already tagged fail",
                "expected": {},
                "actual_result": "1"
            }
        }
        
        result = transform_step_result(step_result)
        
        self.assertEqual(result, step_result)
    
    def test_transform_already_externally_tagged_not_executed(self):
        """Test that already externally-tagged NotExecuted remains unchanged."""
        step_result = {
            "NotExecuted": {
                "step": 3,
                "description": "Already tagged not executed"
            }
        }
        
        result = transform_step_result(step_result)
        
        self.assertEqual(result, step_result)
    
    def test_transform_unknown_status(self):
        """Test handling of unknown status value."""
        step_result = {
            "status": "unknown",
            "step": 4,
            "description": "Unknown status"
        }
        
        result = transform_step_result(step_result)
        
        # Should return as-is when status is unknown
        self.assertEqual(result, step_result)
    
    def test_transform_no_status_field(self):
        """Test handling of step result without status field."""
        step_result = {
            "step": 5,
            "description": "No status field"
        }
        
        result = transform_step_result(step_result)
        
        # Should return as-is when no status field
        self.assertEqual(result, step_result)
    
    def test_transform_case_insensitive_status(self):
        """Test that status transformation is case-insensitive."""
        step_result = {
            "status": "PASS",
            "step": 1,
            "description": "Uppercase status"
        }
        
        result = transform_step_result(step_result)
        
        self.assertIn('Pass', result)
        self.assertEqual(result['Pass']['step'], 1)


class TestIntegration(unittest.TestCase):
    """Integration tests for the complete workflow."""
    
    def setUp(self):
        """Create temporary directory structure."""
        self.temp_dir = tempfile.mkdtemp()
        self.temp_path = Path(self.temp_dir)
        self.test_case_dir = self.temp_path / "testcases"
        self.logs_dir = self.temp_path / "logs"
        self.test_case_dir.mkdir()
        self.logs_dir.mkdir()
    
    def tearDown(self):
        """Clean up temporary directory."""
        shutil.rmtree(self.temp_dir)
    
    def test_full_workflow_from_files(self):
        """Test complete workflow from test case files to TPDG container."""
        # Create test case
        test_case_yaml = """
type: test_case
id: TC-FULL-001
description: Full workflow test
requirement: REQ-FW-001
test_sequences:
  - id: 1
    name: Main Sequence
    steps:
      - step: 1
        description: First step
        expected:
          success: true
          result: 0
          output: "success"
      - step: 2
        description: Second step
        expected:
          success: true
          result: 0
"""
        (self.test_case_dir / "TC-FULL-001.yaml").write_text(test_case_yaml)
        
        # Create execution log
        execution_log = [
            {
                "test_sequence": 1,
                "step": 1,
                "exit_code": 0,
                "output": "success",
                "result_verification_pass": True,
                "output_verification_pass": True
            },
            {
                "test_sequence": 1,
                "step": 2,
                "exit_code": 1,
                "output": "failure",
                "result_verification_pass": False,
                "output_verification_pass": False
            }
        ]
        (self.logs_dir / "TC-FULL-001_execution_log.json").write_text(json.dumps(execution_log))
        
        # Process directory
        test_results = process_test_case_directory(
            self.test_case_dir,
            self.logs_dir,
            recursive=False,
            verbose=False
        )
        
        # Create TPDG container
        container = create_tpdg_container(
            test_results,
            title="Integration Test",
            project="Test Project"
        )
        
        # Verify container structure
        self.assertEqual(container['type'], 'test_results_container')
        self.assertEqual(container['title'], 'Integration Test')
        self.assertEqual(container['project'], 'Test Project')
        self.assertEqual(len(container['test_results']), 1)
        
        # Verify test result
        test_result = container['test_results'][0]
        self.assertEqual(test_result['test_case_id'], 'TC-FULL-001')
        self.assertEqual(test_result['total_steps'], 2)
        self.assertEqual(test_result['passed_steps'], 1)
        self.assertEqual(test_result['failed_steps'], 1)
        self.assertFalse(test_result['overall_pass'])
        
        # Verify metadata
        self.assertEqual(container['metadata']['total_test_cases'], 1)
        self.assertEqual(container['metadata']['passed_test_cases'], 0)
        self.assertEqual(container['metadata']['failed_test_cases'], 1)
    
    def test_workflow_with_verifier_json(self):
        """Test workflow starting from verifier JSON."""
        verifier_data = {
            "test_results": [
                {
                    "test_case_id": "TC-VER-001",
                    "description": "Verifier test",
                    "sequences": [],
                    "total_steps": 3,
                    "passed_steps": 3,
                    "failed_steps": 0,
                    "not_executed_steps": 0,
                    "overall_pass": True,
                    "requirement": "REQ-VER-001"
                }
            ]
        }
        
        # Parse verifier JSON
        test_results = parse_verifier_json(verifier_data)
        
        # Create TPDG container
        container = create_tpdg_container(test_results)
        
        # Verify
        self.assertEqual(len(container['test_results']), 1)
        self.assertEqual(container['test_results'][0]['test_case_id'], 'TC-VER-001')
        self.assertEqual(container['metadata']['passed_test_cases'], 1)
    
    def test_workflow_with_internally_tagged_json(self):
        """Test workflow with internally-tagged step results."""
        verifier_data = {
            "test_cases": [
                {
                    "test_case_id": "TC-INT-001",
                    "description": "Internally tagged test",
                    "sequences": [
                        {
                            "sequence_id": 1,
                            "name": "Test Sequence",
                            "step_results": [
                                {
                                    "status": "pass",
                                    "step": 1,
                                    "description": "Passed step"
                                },
                                {
                                    "status": "fail",
                                    "step": 2,
                                    "description": "Failed step",
                                    "expected": {"result": 0},
                                    "actual_result": "1"
                                },
                                {
                                    "status": "not_executed",
                                    "step": 3,
                                    "description": "Not executed step"
                                }
                            ],
                            "all_steps_passed": False
                        }
                    ],
                    "total_steps": 3,
                    "passed_steps": 1,
                    "failed_steps": 1,
                    "not_executed_steps": 1,
                    "overall_pass": False
                }
            ]
        }
        
        # Parse verifier JSON
        test_results = parse_verifier_json(verifier_data)
        
        # Create TPDG container (this should transform to externally-tagged)
        container = create_tpdg_container(test_results)
        
        # Verify container
        self.assertEqual(len(container['test_results']), 1)
        test_result = container['test_results'][0]
        self.assertEqual(test_result['test_case_id'], 'TC-INT-001')
        
        # Verify step results are transformed to externally-tagged
        step_results = test_result['sequences'][0]['step_results']
        self.assertEqual(len(step_results), 3)
        
        # Check Pass step
        self.assertIn('Pass', step_results[0])
        self.assertEqual(step_results[0]['Pass']['step'], 1)
        self.assertEqual(step_results[0]['Pass']['description'], 'Passed step')
        self.assertNotIn('status', step_results[0])
        
        # Check Fail step
        self.assertIn('Fail', step_results[1])
        self.assertEqual(step_results[1]['Fail']['step'], 2)
        self.assertEqual(step_results[1]['Fail']['description'], 'Failed step')
        self.assertNotIn('status', step_results[1])
        
        # Check NotExecuted step
        self.assertIn('NotExecuted', step_results[2])
        self.assertEqual(step_results[2]['NotExecuted']['step'], 3)
        self.assertEqual(step_results[2]['NotExecuted']['description'], 'Not executed step')
        self.assertNotIn('status', step_results[2])


def run_tests():
    """Run all tests and return exit code."""
    loader = unittest.TestLoader()
    suite = loader.loadTestsFromModule(sys.modules[__name__])
    runner = unittest.TextTestRunner(verbosity=2)
    result = runner.run(suite)
    return 0 if result.wasSuccessful() else 1


if __name__ == '__main__':
    if not YAML_AVAILABLE:
        print("Error: PyYAML is required. Install with: pip3 install pyyaml")
        sys.exit(1)
    
    sys.exit(run_tests())
