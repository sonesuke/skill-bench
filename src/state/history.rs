//! Test history for tracking failed tests

use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs;

const HISTORY_FILE: &str = ".skill-bench/test-history.json";

/// Test history for tracking failed tests
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestHistory {
    pub last_run: DateTime<Utc>,
    pub failed_tests: Vec<FailedTestRecord>,
}

impl Default for TestHistory {
    fn default() -> Self {
        Self {
            last_run: Utc::now(),
            failed_tests: Vec::new(),
        }
    }
}

/// Record of a failed test
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailedTestRecord {
    pub test_id: String,
    pub test_name: String,
    pub skill_name: String,
    pub failed_at: DateTime<Utc>,
    pub error: String,
}

impl TestHistory {
    /// Load test history from file
    pub fn load() -> Self {
        fs::read_to_string(HISTORY_FILE)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    /// Save test history to file
    pub fn save(&self) -> Result<()> {
        let _ = fs::create_dir_all(".skill-bench");
        let json = serde_json::to_string_pretty(self)?;
        fs::write(HISTORY_FILE, json)?;
        Ok(())
    }

    /// Update history with new test results
    pub fn update(&mut self, results: &[crate::models::TestResult]) {
        self.last_run = Utc::now();
        self.failed_tests.clear();

        for result in results {
            if !result.passed {
                self.failed_tests.push(FailedTestRecord {
                    test_id: result.test_id.clone(),
                    test_name: result.test_name.clone(),
                    skill_name: result.skill_name.clone(),
                    failed_at: Utc::now(),
                    error: result
                        .check_results
                        .iter()
                        .filter(|r| !r.passed)
                        .map(|r| r.error.as_deref().unwrap_or("Unknown error"))
                        .collect::<Vec<_>>()
                        .join("; "),
                });
            }
        }

        let _ = self.save();
    }

    /// Get the set of failed test IDs from the last run
    pub fn get_failed_test_ids(&self) -> HashSet<String> {
        self.failed_tests
            .iter()
            .map(|t| t.test_id.clone())
            .collect()
    }
}
