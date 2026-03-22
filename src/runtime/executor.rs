//! Test executor with parallel execution support

use crate::assertions::AssertionChecker;
use crate::models::{CheckResult, TestCase, TestDescriptor, TestResult};
use crate::runtime::embedded::extract_harness_plugin;
use crate::runtime::workspace::TestWorkspace;
use anyhow::Result;
use rayon::prelude::*;
use rayon::ThreadPoolBuilder;
use std::path::PathBuf;
use std::process::Command;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tracing::{error, info, warn};

/// Test executor with parallel execution support
pub struct TestExecutor {
    threads: usize,
    claude_path: PathBuf,
    _plugin_temp_dir: Option<tempfile::TempDir>, // Kept alive for lifetime of TestExecutor
    harness_plugin: Arc<PathBuf>,
    log_output_dir: Option<PathBuf>,
}

impl TestExecutor {
    /// Create a new test executor
    pub fn new(
        threads: usize,
        log_output_dir: Option<String>,
        plugin_dir: Option<String>,
    ) -> Result<Self> {
        // Find claude binary
        let claude_path = which::which("claude").unwrap_or_else(|_| PathBuf::from("claude"));

        // Use specified plugin_dir or extract embedded harness plugin
        let (temp_dir, plugin_path) = if let Some(dir) = plugin_dir {
            let plugin_path = PathBuf::from(&dir);
            if !plugin_path.exists() {
                anyhow::bail!("Plugin directory does not exist: {}", dir);
            }
            (None, plugin_path)
        } else {
            let (temp_dir, plugin_dir) = extract_harness_plugin()?;
            (Some(temp_dir), plugin_dir)
        };

        // Parse log output directory
        let log_output_dir = log_output_dir.filter(|d| !d.is_empty()).map(PathBuf::from);

        Ok(Self {
            threads,
            claude_path,
            _plugin_temp_dir: temp_dir,
            harness_plugin: Arc::new(plugin_path),
            log_output_dir,
        })
    }

    /// Execute tests in parallel
    pub fn execute_tests(&self, tests: Vec<TestDescriptor>) -> Result<Vec<TestResult>> {
        info!(
            "Executing {} tests with {} threads",
            tests.len(),
            self.threads
        );

        // Configure thread pool
        ThreadPoolBuilder::new()
            .num_threads(self.threads)
            .build_global()
            .map_err(|e| anyhow::anyhow!("Failed to build thread pool: {}", e))?;

        // Execute tests in parallel
        let results: Vec<TestResult> = tests
            .into_par_iter()
            .map(|test| self.execute_single_test(test))
            .collect();

        Ok(results)
    }

    /// Execute a single test
    fn execute_single_test(&self, desc: TestDescriptor) -> TestResult {
        let start = Instant::now();

        info!("Starting test: {}/{}", desc.skill_name, desc.test_name);

        // Create workspace
        let workspace = match TestWorkspace::create(&desc.skill_name, &desc.test_name) {
            Ok(w) => w,
            Err(e) => {
                error!("Failed to create workspace for {}: {}", desc.test_id, e);
                return TestResult {
                    test_id: desc.test_id.clone(),
                    test_name: desc.test_name,
                    skill_name: desc.skill_name,
                    passed: false,
                    duration: start.elapsed(),
                    check_results: vec![],
                    execution_error: Some(format!("Workspace creation failed: {}", e)),
                };
            }
        };

        // Run setup steps
        if let Err(e) = workspace.run_setup(&desc.test.setup) {
            error!("Setup failed for {}: {}", desc.test_id, e);
            return TestResult {
                test_id: desc.test_id.clone(),
                test_name: desc.test_name,
                skill_name: desc.skill_name,
                passed: false,
                duration: start.elapsed(),
                check_results: vec![],
                execution_error: Some(format!("Setup failed: {}", e)),
            };
        }

        // Execute Claude CLI
        let log_path = workspace.log_path().to_path_buf();
        match self.execute_claude(&workspace, &desc.test) {
            Ok(_) => {
                info!("Claude execution completed for {}", desc.test_id);
            }
            Err(e) => {
                warn!("Claude execution error for {}: {}", desc.test_id, e);
                return TestResult {
                    test_id: desc.test_id.clone(),
                    test_name: desc.test_name,
                    skill_name: desc.skill_name,
                    passed: false,
                    duration: start.elapsed(),
                    check_results: vec![],
                    execution_error: Some(format!("Claude execution failed: {}", e)),
                };
            }
        }

        // Run assertions
        let checker = AssertionChecker::new(&log_path, workspace.path());
        let check_results: Vec<CheckResult> = desc
            .test
            .checks
            .iter()
            .map(|check| match checker.evaluate_check(check) {
                Ok(()) => CheckResult {
                    name: check.name.clone(),
                    passed: true,
                    error: None,
                },
                Err(e) => {
                    warn!("Check '{}' failed for {}: {}", check.name, desc.test_id, e);
                    CheckResult {
                        name: check.name.clone(),
                        passed: false,
                        error: Some(e),
                    }
                }
            })
            .collect();

        // Copy log to output directory if specified
        if let Some(ref output_dir) = self.log_output_dir {
            if let Err(e) = self.copy_log_to_output(&log_path, output_dir, &desc) {
                warn!("Failed to copy log for {}: {}", desc.test_id, e);
            }
        }

        let passed = check_results.iter().all(|r| r.passed);
        let duration = start.elapsed();

        info!(
            "Test {} completed: {} ({:.2}s)",
            desc.test_id,
            if passed { "PASS" } else { "FAIL" },
            duration.as_secs_f64()
        );

        TestResult {
            test_id: desc.test_id,
            test_name: desc.test_name,
            skill_name: desc.skill_name,
            passed,
            duration,
            check_results,
            execution_error: None,
        }
    }

    /// Copy log file to output directory
    fn copy_log_to_output(
        &self,
        log_path: &PathBuf,
        output_dir: &PathBuf,
        desc: &TestDescriptor,
    ) -> Result<()> {
        // Create output directory if it doesn't exist
        std::fs::create_dir_all(output_dir)?;

        // Create filename: timestamp_skill_test.log
        let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
        let filename = format!("{}_{}_{}.log", timestamp, desc.skill_name, desc.test_name);
        let dest_path = output_dir.join(&filename);

        // Copy log file
        std::fs::copy(log_path, &dest_path)?;

        info!("Log saved to: {}", dest_path.display());
        Ok(())
    }

    /// Execute Claude CLI
    fn execute_claude(&self, workspace: &TestWorkspace, test: &TestCase) -> Result<()> {
        let timeout = Duration::from_secs(test.timeout);

        let plugin_dir = self
            .harness_plugin
            .to_str()
            .ok_or_else(|| anyhow::anyhow!("Invalid plugin path"))?;

        let mut child = Command::new(&self.claude_path)
            .args([
                "-p",
                "--dangerously-skip-permissions",
                "--verbose",
                "--output-format",
                "stream-json",
                "--plugin-dir",
                plugin_dir,
                "--",
                test.test_prompt.trim(),
            ])
            .current_dir(workspace.path())
            .env("CLAUDECODE", "") // Unset to avoid nested session
            .env("SKILL_BENCH_TEST", "1")
            .stdout(std::fs::File::create(workspace.log_path())?)
            .stderr(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| anyhow::anyhow!("Failed to spawn claude: {}", e))?;

        // Wait with timeout
        let start = Instant::now();
        loop {
            match child.try_wait() {
                Ok(Some(status)) => {
                    if status.success() {
                        return Ok(());
                    } else {
                        return Err(anyhow::anyhow!("Claude exited with status: {}", status));
                    }
                }
                Ok(None) => {
                    if start.elapsed() > timeout {
                        child
                            .kill()
                            .map_err(|e| anyhow::anyhow!("Failed to kill claude: {}", e))?;
                        return Err(anyhow::anyhow!(
                            "Test timed out after {}s",
                            timeout.as_secs()
                        ));
                    }
                    std::thread::sleep(Duration::from_millis(100));
                }
                Err(e) => {
                    return Err(anyhow::anyhow!("Failed to wait for claude: {}", e));
                }
            }
        }
    }
}
