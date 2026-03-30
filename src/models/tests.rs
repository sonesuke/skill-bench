//! Unit tests for data models

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    #[test]
    fn test_setup_step_file_serialization() {
        let setup = crate::models::SetupStep::File {
            name: Some("test_file".to_string()),
            path: "test.txt".to_string(),
            content: "Hello, World!".to_string(),
        };

        // Serialize to TOML
        let toml_str = toml::to_string(&setup).unwrap();
        assert!(toml_str.contains("path"));
        assert!(toml_str.contains("content"));

        // Deserialize back
        let deserialized: crate::models::SetupStep = toml::from_str(&toml_str).unwrap();
        match deserialized {
            crate::models::SetupStep::File { path, content, .. } => {
                assert_eq!(path, "test.txt");
                assert_eq!(content, "Hello, World!");
            }
            _ => panic!("Expected File variant"),
        }
    }

    #[test]
    fn test_setup_step_script_serialization() {
        let setup = crate::models::SetupStep::Script {
            name: Some("test_script".to_string()),
            command: "echo 'hello'".to_string(),
        };

        // Serialize to TOML
        let toml_str = toml::to_string(&setup).unwrap();
        assert!(toml_str.contains("command"));

        // Deserialize back
        let deserialized: crate::models::SetupStep = toml::from_str(&toml_str).unwrap();
        match deserialized {
            crate::models::SetupStep::Script { command, .. } => {
                assert_eq!(command, "echo 'hello'");
            }
            _ => panic!("Expected Script variant"),
        }
    }

    #[test]
    fn test_test_case_deserialization() {
        let toml_str = r#"
name = "test-name"
description = "Test description"
timeout = 120

test_prompt = "Test prompt here..."

[[setup]]
name = "setup_name"
path = "test.txt"
content = "File content"

[[checks]]
name = "check_name"
command = {command = "skill-invoked", skill = "skill-name"}
"#;

        let test_case: crate::models::TestCase = toml::from_str(toml_str).unwrap();
        assert_eq!(test_case.name, "test-name");
        assert_eq!(test_case.description, "Test description");
        assert_eq!(test_case.timeout, 120);
        assert_eq!(test_case.test_prompt, "Test prompt here...");
        assert_eq!(test_case.setup.len(), 1);
        assert_eq!(test_case.checks.len(), 1);
    }

    #[test]
    fn test_test_case_default_timeout() {
        let toml_str = r#"
name = "test-name"
description = "Test description"

test_prompt = "Test prompt here..."
"#;

        let test_case: crate::models::TestCase = toml::from_str(toml_str).unwrap();
        assert_eq!(test_case.timeout, 300); // Default timeout
    }

    #[test]
    fn test_check_step_serialization() {
        let toml_str = r#"
name = "skill-check"
command = {command = "skill-invoked", skill = "test-skill"}
deny = false
"#;

        let check: crate::models::CheckStep = toml::from_str(toml_str).unwrap();
        assert_eq!(check.name, "skill-check");
        assert_eq!(check.command.command, "skill-invoked");
        assert_eq!(check.command.skill.as_deref(), Some("test-skill"));
        assert_eq!(check.deny, false);
    }

    #[test]
    fn test_check_step_with_deny() {
        let toml_str = r#"
name = "negative-check"
command = {command = "log-contains", pattern = "error"}
deny = true
"#;

        let check: crate::models::CheckStep = toml::from_str(toml_str).unwrap();
        assert_eq!(check.deny, true);
        assert_eq!(check.command.pattern.as_deref(), Some("error"));
    }

    #[test]
    fn test_test_descriptor_from_path() {
        let temp_dir = TempDir::new().unwrap();
        let skill_dir = temp_dir.path().join("test-skill");
        std::fs::create_dir(&skill_dir).unwrap();

        let test_file = skill_dir.join("test-case.toml");
        let toml_content = r#"
name = "test-case"
description = "Test case description"
timeout = 60

test_prompt = "Test prompt"
"#;
        std::fs::write(&test_file, toml_content).unwrap();

        let descriptor = crate::models::TestDescriptor::from_path(test_file).unwrap();
        assert_eq!(descriptor.skill_name, "test-skill");
        assert_eq!(descriptor.test_name, "test-case");
        assert_eq!(descriptor.test_id, "test-skill/test-case");
        assert_eq!(descriptor.test.name, "test-case");
    }

    #[test]
    fn test_test_result_serialization() {
        use crate::models::TestStatus;
        let result = crate::models::TestResult {
            test_id: "skill/test".to_string(),
            test_name: "test".to_string(),
            skill_name: "skill".to_string(),
            status: TestStatus::Pass,
            duration: std::time::Duration::from_secs(5),
            check_results: vec![crate::models::CheckResult {
                name: "check1".to_string(),
                passed: true,
                error: None,
            }],
            execution_error: None,
        };

        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("\"test_id\""));

        let deserialized: crate::models::TestResult = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.test_id, "skill/test");
        assert!(deserialized.is_pass());
    }

    #[test]
    fn test_test_run_summary() {
        use crate::models::TestStatus;
        let results = vec![
            crate::models::TestResult {
                test_id: "skill/test1".to_string(),
                test_name: "test1".to_string(),
                skill_name: "skill".to_string(),
                status: TestStatus::Pass,
                duration: std::time::Duration::from_secs(1),
                check_results: vec![],
                execution_error: None,
            },
            crate::models::TestResult {
                test_id: "skill/test2".to_string(),
                test_name: "test2".to_string(),
                skill_name: "skill".to_string(),
                status: TestStatus::Fail,
                duration: std::time::Duration::from_secs(2),
                check_results: vec![],
                execution_error: Some("error".to_string()),
            },
        ];

        let summary = crate::models::TestRunSummary::from_results(results);
        assert_eq!(summary.total, 2);
        assert_eq!(summary.passed, 1);
        assert_eq!(summary.failed, 1);
        assert_eq!(summary.skipped, 0);
        assert_eq!(summary.duration.as_secs(), 3);

        let failures = summary.failures();
        assert_eq!(failures.len(), 1);
        assert_eq!(failures[0].test_name, "test2");
    }

    #[test]
    fn test_check_data_all_fields() {
        let toml_str = r#"
command = "tool-param"
tool = "Read"
param = "file_path"
value = "test.txt"
deny = false
"#;

        let data: crate::models::check::CheckData = toml::from_str(toml_str).unwrap();
        assert_eq!(data.command, "tool-param");
        assert_eq!(data.tool.as_deref(), Some("Read"));
        assert_eq!(data.param.as_deref(), Some("file_path"));
        assert_eq!(data.value.as_deref(), Some("test.txt"));
        assert_eq!(data.deny, Some(false));
    }

    #[test]
    fn test_empty_setup_and_checks() {
        let toml_str = r#"
name = "minimal-test"
description = "Minimal test"
timeout = 30

test_prompt = "Prompt"
"#;

        let test_case: crate::models::TestCase = toml::from_str(toml_str).unwrap();
        assert!(test_case.setup.is_empty());
        assert!(test_case.checks.is_empty());
        assert!(test_case.answers.is_none());
    }

    #[test]
    fn test_answers_field() {
        let toml_str = r#"
name = "test-with-answers"
description = "Test with answers"

test_prompt = "Prompt"

[answers]
"question1" = "answer1"
"question2" = "answer2"
"#;

        let test_case: crate::models::TestCase = toml::from_str(toml_str).unwrap();
        assert!(test_case.answers.is_some());
        let answers = test_case.answers.unwrap();
        assert_eq!(answers.len(), 2);
    }
}
