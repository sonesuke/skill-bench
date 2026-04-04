//! Unit tests for assertion checker using real log files

#[cfg(test)]
mod tests {
    use crate::assertions::AssertionChecker;
    use crate::models::check::CheckData;
    use std::path::Path;

    fn create_checker(log_file: &str) -> AssertionChecker {
        let log_path = Path::new(log_file);
        let work_dir = tempfile::tempdir().unwrap();
        AssertionChecker::new(log_path, work_dir.path(), None, None)
    }

    #[test]
    fn test_skill_loaded_check() {
        let checker =
            create_checker("tests/fixtures/logs/20260308_133201_functional-analyze-patent.log");

        // claim-analyzing skill should be in the skills list
        let result =
            crate::assertions::skill::check_skill_loaded(&checker, "patent-kit:claim-analyzing");
        assert!(result.is_ok(), "claim-analyzing skill should be loaded");

        // evaluating skill should be loaded
        let result =
            crate::assertions::skill::check_skill_loaded(&checker, "patent-kit:evaluating");
        assert!(result.is_ok(), "evaluating skill should be loaded");

        // Non-existent skill should fail
        let result = crate::assertions::skill::check_skill_loaded(&checker, "non-existent-skill");
        assert!(result.is_err(), "non-existent skill should not be found");
    }

    #[test]
    fn test_mcp_loaded_check() {
        let checker =
            create_checker("tests/fixtures/logs/20260308_133201_functional-analyze-patent.log");

        // google-patent-cli MCP server should be loaded
        let result = crate::assertions::mcp::check_mcp_loaded(
            &checker,
            "plugin:google-patent-cli:google-patent-cli",
        );
        assert!(result.is_ok(), "google-patent-cli MCP should be loaded");

        // arxiv-cli MCP server should be loaded
        let result =
            crate::assertions::mcp::check_mcp_loaded(&checker, "plugin:arxiv-cli:arxiv-cli");
        assert!(result.is_ok(), "arxiv-cli MCP should be loaded");

        // Non-existent MCP should fail
        let result = crate::assertions::mcp::check_mcp_loaded(&checker, "non-existent-mcp");
        assert!(result.is_err(), "non-existent MCP should not be found");
    }

    #[test]
    fn test_log_contains_check() {
        let checker =
            create_checker("tests/fixtures/logs/20260308_133201_functional-analyze-patent.log");

        // The log should contain system messages
        let result = crate::assertions::log::check_log_contains(&checker.log_data, "system");
        assert!(result.is_ok(), "log should contain 'system'");

        // The log should not contain a very unlikely string
        let result =
            crate::assertions::log::check_log_contains(&checker.log_data, "xyz123abc456def789");
        assert!(result.is_err(), "log should not contain random string");
    }

    #[test]
    fn test_output_contains_check() {
        let checker =
            create_checker("tests/fixtures/logs/20260307_020146_uses-question-responder.log");

        // The log should contain assistant messages
        let result = crate::assertions::log::check_output_contains(&checker.log_data, "user");
        assert!(
            result.is_ok(),
            "log should contain 'user' in assistant messages"
        );
    }

    #[test]
    fn test_file_content_check() {
        let work_dir = tempfile::tempdir().unwrap();
        let test_file = work_dir.path().join("test.txt");
        std::fs::write(&test_file, "Hello, World!").unwrap();

        // Check that file contains expected content
        let result =
            crate::assertions::file::check_file_content(work_dir.path(), "test.txt", "Hello", true);
        assert!(result.is_ok(), "file should contain 'Hello'");

        // Check that file does not contain unexpected content
        let result = crate::assertions::file::check_file_content(
            work_dir.path(),
            "test.txt",
            "Goodbye",
            false,
        );
        assert!(result.is_ok(), "file should not contain 'Goodbye'");
    }

    #[test]
    fn test_file_content_not_found() {
        let work_dir = tempfile::tempdir().unwrap();
        let test_file = work_dir.path().join("test.txt");
        std::fs::write(&test_file, "Hello, World!").unwrap();

        // Check that file contains expected content (positive case)
        let result =
            crate::assertions::file::check_file_content(work_dir.path(), "test.txt", "Hello", true);
        assert!(result.is_ok(), "file should contain 'Hello'");

        // Check that file contains unexpected content (should fail)
        let result = crate::assertions::file::check_file_content(
            work_dir.path(),
            "test.txt",
            "Hello",
            false,
        );
        assert!(
            result.is_err(),
            "file should not fail negative check for 'Hello'"
        );
    }

    #[test]
    fn test_workspace_file_check() {
        let work_dir = tempfile::tempdir().unwrap();
        let test_file = work_dir.path().join("output.txt");
        std::fs::write(&test_file, "test content").unwrap();

        let result = crate::assertions::file::check_workspace_file(
            work_dir.path(),
            "output.txt",
            false,
            None,
        );
        assert!(result.is_ok(), "workspace file should exist");

        let result = crate::assertions::file::check_workspace_file(
            work_dir.path(),
            "nonexistent.txt",
            false,
            None,
        );
        assert!(result.is_err(), "nonexistent file should not exist");
    }

    #[test]
    fn test_workspace_dir_check() {
        let work_dir = tempfile::tempdir().unwrap();
        let test_dir = work_dir.path().join("subdir");
        std::fs::create_dir(&test_dir).unwrap();

        let result = crate::assertions::file::check_workspace_dir(work_dir.path(), &["subdir"]);
        assert!(result.is_ok(), "workspace directory should exist");

        let result =
            crate::assertions::file::check_workspace_dir(work_dir.path(), &["nonexistent"]);
        assert!(result.is_err(), "nonexistent directory should not exist");
    }

    #[test]
    fn test_workspace_dir_multiple() {
        let work_dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(work_dir.path().join("dir1")).unwrap();
        std::fs::create_dir(work_dir.path().join("dir2")).unwrap();

        let result =
            crate::assertions::file::check_workspace_dir(work_dir.path(), &["dir1", "dir2"]);
        assert!(result.is_ok(), "all workspace directories should exist");

        let result =
            crate::assertions::file::check_workspace_dir(work_dir.path(), &["dir1", "nonexistent"]);
        assert!(result.is_err(), "should fail if any directory is missing");
    }

    #[test]
    fn test_init_message_extraction() {
        let checker =
            create_checker("tests/fixtures/logs/20260308_133201_functional-analyze-patent.log");

        let init = checker.init_message();
        assert!(init.is_some(), "should have init message");

        let init = init.unwrap();
        assert_eq!(init["type"], "system");
        assert_eq!(init["subtype"], "init");
    }

    #[test]
    fn test_init_message_missing_in_empty_log() {
        let work_dir = tempfile::tempdir().unwrap();
        let empty_log = work_dir.path().join("empty.log");
        std::fs::write(&empty_log, "").unwrap();

        let checker = AssertionChecker::new(&empty_log, work_dir.path(), None, None);
        let init = checker.init_message();
        assert!(init.is_none(), "empty log should not have init message");
    }

    #[test]
    fn test_multiple_skills_loaded() {
        let checker =
            create_checker("tests/fixtures/logs/20260308_133201_functional-analyze-patent.log");

        // Multiple skills should be loaded
        let skills_to_check = vec![
            "patent-kit:claim-analyzing",
            "patent-kit:evaluating",
            "patent-kit:targeting",
            "patent-kit:screening",
        ];

        for skill in skills_to_check {
            let result = crate::assertions::skill::check_skill_loaded(&checker, skill);
            assert!(result.is_ok(), "{} skill should be loaded", skill);
        }
    }

    #[test]
    fn test_concept_interviewing_loaded() {
        let checker =
            create_checker("tests/fixtures/logs/20260307_020146_uses-question-responder.log");

        // concept-interviewing skill should be loaded
        let result = crate::assertions::skill::check_skill_loaded(
            &checker,
            "patent-kit:concept-interviewing",
        );
        assert!(
            result.is_ok(),
            "concept-interviewing skill should be loaded"
        );
    }

    #[test]
    fn test_log_pattern_regex() {
        let checker =
            create_checker("tests/fixtures/logs/20260308_133201_functional-analyze-patent.log");

        // Test pattern matching with regex
        let result = crate::assertions::log::check_log_contains(&checker.log_data, "type");
        assert!(result.is_ok(), "log should contain 'type' pattern");
    }

    #[test]
    fn test_nonexistent_log_file() {
        let work_dir = tempfile::tempdir().unwrap();
        let nonexistent_log = work_dir.path().join("nonexistent.log");

        // Should not panic, just return empty checker
        let checker = AssertionChecker::new(&nonexistent_log, work_dir.path(), None, None);
        assert_eq!(
            checker.log_data.len(),
            0,
            "nonexistent log should produce empty data"
        );
    }

    #[test]
    fn test_file_contains_via_evaluate_check() {
        let work_dir = tempfile::tempdir().unwrap();
        let test_file = work_dir.path().join("report.md");
        std::fs::write(&test_file, "Screened patents: 42").unwrap();

        let empty_log = work_dir.path().join("empty.log");
        std::fs::write(&empty_log, "").unwrap();

        let checker = AssertionChecker::new(&empty_log, work_dir.path(), None, None);

        let check = crate::models::CheckStep {
            name: "contains_patent_data".to_string(),
            command: CheckData {
                command: "file-contains".to_string(),
                file: Some("report.md".to_string()),
                contains: Some("Screened patents".to_string()),
                ..Default::default()
            },
            deny: false,
        };

        let result = checker.evaluate_check(&check);
        assert!(
            result.is_ok(),
            "file-contains should pass when string is present"
        );

        // String not in file should fail
        let check_missing = crate::models::CheckStep {
            name: "contains_missing".to_string(),
            command: CheckData {
                command: "file-contains".to_string(),
                file: Some("report.md".to_string()),
                contains: Some("NOTPRESENT".to_string()),
                ..Default::default()
            },
            deny: false,
        };

        let result = checker.evaluate_check(&check_missing);
        assert!(
            result.is_err(),
            "file-contains should fail when string is absent"
        );
    }

    #[test]
    fn test_copy_to_output_creates_subdirectory() {
        let work_dir = tempfile::tempdir().unwrap();
        let output_dir = tempfile::tempdir().unwrap();
        let test_file = work_dir.path().join("output.txt");
        std::fs::write(&test_file, "test content").unwrap();

        let empty_log = work_dir.path().join("empty.log");
        std::fs::write(&empty_log, "").unwrap();

        let checker = AssertionChecker::new(
            &empty_log,
            work_dir.path(),
            Some(output_dir.path()),
            Some("skill_test_20260404_050943"),
        );

        let check = crate::models::CheckStep {
            name: "copy_file".to_string(),
            command: CheckData {
                command: "workspace-file".to_string(),
                path: Some("output.txt".to_string()),
                copy_to_output: Some(true),
                ..Default::default()
            },
            deny: false,
        };

        let result = checker.evaluate_check(&check);
        assert!(result.is_ok(), "workspace-file with copy should pass");

        let copied = output_dir
            .path()
            .join("skill_test_20260404_050943/output.txt");
        assert!(copied.exists(), "file should be copied to subdirectory");
        let content = std::fs::read_to_string(&copied).unwrap();
        assert_eq!(content, "test content", "copied content should match");
    }
}
