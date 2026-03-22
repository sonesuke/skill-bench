//! Test executor with parallel execution support

use crate::assertions::AssertionChecker;
use crate::models::{CheckResult, TestCase, TestDescriptor, TestResult};
use crate::runtime::embedded::extract_harness_plugin;
use crate::runtime::workspace::TestWorkspace;
use anyhow::Result;
use rayon::prelude::*;
use rayon::ThreadPoolBuilder;
use serde_json::Value;
use std::fs;
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
    skills_dir: Option<PathBuf>,
}

impl TestExecutor {
    /// Create a new test executor
    pub fn new(
        threads: usize,
        log_output_dir: Option<String>,
        skills_dir: Option<String>,
        plugin_dir: Option<String>,
    ) -> Result<Self> {
        // Find claude binary
        let claude_path = which::which("claude").unwrap_or_else(|_| PathBuf::from("claude"));

        // Always extract harness plugin (we'll merge skills_dir into it later if specified)
        let (temp_dir, harness_plugin_path) = extract_harness_plugin()?;

        // Parse log output directory
        let log_output_dir = log_output_dir.filter(|d| !d.is_empty()).map(PathBuf::from);

        // Parse skills directory (plugin_dir option is actually the skills directory)
        let skills_dir = if let Some(dir) = plugin_dir {
            let path = PathBuf::from(&dir);
            if !path.exists() {
                anyhow::bail!("Plugin directory does not exist: {}", dir);
            }
            Some(path)
        } else {
            skills_dir.filter(|d| !d.is_empty()).map(PathBuf::from)
        };

        Ok(Self {
            threads,
            claude_path,
            _plugin_temp_dir: Some(temp_dir),
            harness_plugin: Arc::new(harness_plugin_path),
            log_output_dir,
            skills_dir,
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

        // Merge skills_dir into harness plugin (so they load together via --plugin-dir)
        if let Some(ref skills_dir) = self.skills_dir {
            // Copy skills/ contents (not the directory itself)
            let skills_src = skills_dir.join("skills");
            let harness_skills = self.harness_plugin.join("skills");
            if skills_src.exists() {
                // Read the contents of skills_src and copy each item
                if let Ok(entries) = std::fs::read_dir(&skills_src) {
                    let items: Vec<_> = entries
                        .filter_map(|e| e.ok())
                        .map(|e| e.path())
                        .collect();

                    if !items.is_empty() {
                        let mut copy_options = fs_extra::dir::CopyOptions::new();
                        copy_options.overwrite = true;

                        if let Err(e) = fs_extra::copy_items(
                            &items.iter().map(|p| p.as_path()).collect::<Vec<_>>(),
                            &harness_skills,
                            &copy_options,
                        ) {
                            // Ignore EEXIST errors (files already copied by another test)
                            if !e.to_string().contains("File exists") && !e.to_string().contains("os error 17") {
                                warn!("Failed to merge skills for {}: {}", desc.test_id, e);
                            }
                        }
                    }
                }
            }

            // Copy agents/ contents
            let agents_src = skills_dir.join("agents");
            let harness_agents = self.harness_plugin.join("agents");
            if agents_src.exists() {
                if let Ok(entries) = std::fs::read_dir(&agents_src) {
                    let items: Vec<_> = entries
                        .filter_map(|e| e.ok())
                        .map(|e| e.path())
                        .collect();

                    if !items.is_empty() {
                        let mut copy_options = fs_extra::dir::CopyOptions::new();
                        copy_options.overwrite = true;

                        if let Err(e) = fs_extra::copy_items(
                            &items.iter().map(|p| p.as_path()).collect::<Vec<_>>(),
                            &harness_agents,
                            &copy_options,
                        ) {
                            // Ignore EEXIST errors (files already copied by another test)
                            if !e.to_string().contains("File exists") && !e.to_string().contains("os error 17") {
                                warn!("Failed to merge agents for {}: {}", desc.test_id, e);
                            }
                        }
                    }
                }
            }

            // Merge mcpServers and name from skills_dir plugin.json into harness plugin.json
            let src_plugin_json = skills_dir.join(".claude-plugin/plugin.json");
            let harness_plugin_json = self.harness_plugin.join(".claude-plugin/plugin.json");
            if src_plugin_json.exists() && harness_plugin_json.exists() {
                if let Err(e) = merge_plugin_json(&harness_plugin_json, &src_plugin_json) {
                    warn!("Failed to merge plugin.json for {}: {}", desc.test_id, e);
                }
            }
        }

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

        // Create filename: skill_test_timestamp.log
        let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
        let filename = format!("{}_{}_{}.log", desc.skill_name, desc.test_name, timestamp);
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

/// Merge mcpServers and name from src_plugin_json into harness_plugin_json
fn merge_plugin_json(harness_path: &PathBuf, src_path: &PathBuf) -> Result<()> {
    // Read both plugin.json files
    let harness_content = fs::read_to_string(harness_path)?;
    let src_content = fs::read_to_string(src_path)?;

    let mut harness_json: Value = serde_json::from_str(&harness_content)?;
    let src_json: Value = serde_json::from_str(&src_content)?;

    // Use name from source plugin (so skills get correct prefix)
    if let Some(src_name) = src_json.get("name").and_then(|v| v.as_str()) {
        harness_json["name"] = Value::String(src_name.to_string());
    }

    // Merge mcpServers
    if let Some(src_mcp) = src_json.get("mcpServers").and_then(|v| v.as_object()) {
        if let Some(harness_mcp) = harness_json.get_mut("mcpServers").and_then(|v| v.as_object_mut()) {
            for (key, value) in src_mcp {
                harness_mcp.insert(key.clone(), value.clone());
            }
        }
    }

    // Write merged content back
    let merged = serde_json::to_string_pretty(&harness_json)?;
    fs::write(harness_path, merged)?;

    Ok(())
}
