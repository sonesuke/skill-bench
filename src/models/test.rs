//! Test case and execution result models

use super::check::CheckStep;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// Test case definition loaded from TOML
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TestCase {
    pub name: String,
    pub description: String,
    #[serde(default = "default_timeout")]
    pub timeout: u64,
    pub test_prompt: String,
    #[serde(default)]
    pub setup: Vec<SetupStep>,
    #[serde(default)]
    pub checks: Vec<CheckStep>,
    #[serde(default)]
    pub answers: Option<HashMap<String, toml::Value>>,
}

fn default_timeout() -> u64 {
    300
}

/// Setup step - either create a file or execute a script
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum SetupStep {
    Script {
        #[serde(default)]
        name: Option<String>,
        command: String,
    },
    File {
        #[serde(default)]
        name: Option<String>,
        path: String,
        content: String,
    },
}

/// Test descriptor containing metadata and the test case
#[derive(Debug, Clone)]
pub struct TestDescriptor {
    #[allow(dead_code)]
    pub path: PathBuf,
    pub skill_name: String,
    pub test_name: String,
    pub test_id: String,
    pub test: TestCase,
}

impl TestDescriptor {
    pub fn from_path(path: PathBuf) -> Result<Self, anyhow::Error> {
        let skill_name = path
            .parent()
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str())
            .ok_or_else(|| anyhow::anyhow!("Invalid test path: cannot extract skill name"))?
            .to_string();

        let test_name = path
            .file_stem()
            .and_then(|n| n.to_str())
            .ok_or_else(|| anyhow::anyhow!("Invalid test path: cannot extract test name"))?
            .to_string();

        let content = std::fs::read_to_string(&path)?;
        let test: TestCase = toml::from_str(&content)?;

        let test_id = format!("{}/{}", skill_name, test_name);

        Ok(Self {
            path,
            skill_name,
            test_name,
            test_id,
            test,
        })
    }
}

/// Test execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestResult {
    pub test_id: String,
    pub test_name: String,
    pub skill_name: String,
    pub passed: bool,
    pub duration: std::time::Duration,
    pub check_results: Vec<CheckResult>,
    pub execution_error: Option<String>,
}

/// Result of a single check
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckResult {
    pub name: String,
    pub passed: bool,
    pub error: Option<String>,
}

/// Summary of a test run
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestRunSummary {
    pub total: usize,
    pub passed: usize,
    pub failed: usize,
    pub skipped: usize,
    pub duration: std::time::Duration,
    pub results: Vec<TestResult>,
}

impl TestRunSummary {
    pub fn from_results(results: Vec<TestResult>) -> Self {
        let duration = results
            .iter()
            .map(|r| r.duration)
            .fold(std::time::Duration::ZERO, |acc, d| acc + d);

        let passed = results.iter().filter(|r| r.passed).count();
        let failed = results.iter().filter(|r| !r.passed).count();

        Self {
            total: results.len(),
            passed,
            failed,
            skipped: 0,
            duration,
            results,
        }
    }

    pub fn failures(&self) -> Vec<&TestResult> {
        self.results.iter().filter(|r| !r.passed).collect()
    }

    pub fn passed_results(&self) -> Vec<&TestResult> {
        self.results.iter().filter(|r| r.passed).collect()
    }
}
