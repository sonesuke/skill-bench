//! Test executor with parallel execution support

use crate::assertions::AssertionChecker;
use crate::models::{CheckResult, TestCase, TestDescriptor, TestResult, TestStatus};
use crate::runtime::embedded::extract_harness_plugin_with_answers;
use crate::runtime::workspace::TestWorkspace;
use anyhow::Result;
use rayon::prelude::*;
use rayon::ThreadPoolBuilder;
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tracing::{error, info, warn};

/// Test executor with parallel execution support
pub struct TestExecutor {
    threads: usize,
    claude_path: PathBuf,
    test_plugin_dir: Option<PathBuf>,
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

        // Parse test plugin dir from --plugin-dir
        // Convert to absolute path to avoid issues when Claude runs in different directory
        let test_plugin_dir = plugin_dir
            .filter(|d| !d.is_empty())
            .map(PathBuf::from)
            .filter(|p| p.exists())
            .map(|p| p.canonicalize().unwrap_or(p));

        // Parse log output directory
        let log_output_dir = log_output_dir.filter(|d| !d.is_empty()).map(PathBuf::from);

        Ok(Self {
            threads,
            claude_path,
            test_plugin_dir,
            log_output_dir,
        })
    }

    /// Execute tests in parallel with real-time output callback
    pub fn execute_tests<F>(
        &self,
        tests: Vec<TestDescriptor>,
        on_test_complete: F,
    ) -> Result<Vec<TestResult>>
    where
        F: Fn(&TestResult) + Send + Sync,
    {
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

        let results = Arc::new(Mutex::new(Vec::with_capacity(tests.len())));

        // Execute tests in parallel, printing results as they complete
        tests.into_par_iter().for_each(|test| {
            let result = self.execute_single_test(test);
            on_test_complete(&result);
            results.lock().unwrap().push(result);
        });

        let results = Arc::try_unwrap(results).unwrap().into_inner().unwrap();

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
                    status: TestStatus::Fail,
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
                status: TestStatus::Fail,
                duration: start.elapsed(),
                check_results: vec![],
                execution_error: Some(format!("Setup failed: {}", e)),
            };
        }

        // Determine log output directory: --log, default (.skill-bench/logs), or temp workspace
        let output_dir = if let Some(ref dir) = self.log_output_dir {
            dir.clone()
        } else {
            std::path::PathBuf::from(".skill-bench/logs")
        };
        let _ = std::fs::create_dir_all(&output_dir);

        let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
        let filename = format!("{}_{}_{}.log", desc.skill_name, desc.test_name, timestamp);
        let log_path = output_dir.join(&filename);

        // Execute Claude CLI
        match self.execute_claude(&workspace, &desc.test, &log_path) {
            Ok(_) => {
                info!("Claude execution completed for {}", desc.test_id);
            }
            Err(e) => {
                warn!("Claude execution error for {}: {}", desc.test_id, e);
                return TestResult {
                    test_id: desc.test_id.clone(),
                    test_name: desc.test_name,
                    skill_name: desc.skill_name,
                    status: TestStatus::Fail,
                    duration: start.elapsed(),
                    check_results: vec![],
                    execution_error: Some(format!("Claude execution failed: {}", e)),
                };
            }
        }

        // Run assertions
        let checker =
            AssertionChecker::new(&log_path, workspace.path(), self.log_output_dir.as_deref());
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

        let passed = check_results.iter().all(|r| r.passed);
        let status = if passed {
            TestStatus::Pass
        } else {
            TestStatus::Fail
        };
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
            status,
            duration,
            check_results,
            execution_error: None,
        }
    }

    /// Execute Claude CLI
    fn execute_claude(
        &self,
        workspace: &TestWorkspace,
        test: &TestCase,
        log_path: &PathBuf,
    ) -> Result<()> {
        let timeout = Duration::from_secs(test.timeout);
        let test_start = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs_f64();

        // Build command args
        let mut args: Vec<String> = vec![
            "-p".to_string(),
            "--dangerously-skip-permissions".to_string(),
            "--verbose".to_string(),
            "--output-format".to_string(),
            "stream-json".to_string(),
        ];

        // Add harness plugin with answers if test has [answers] section
        // TempDir is kept alive in this scope so the directory persists during execution
        let _harness_temp = if let Some(ref answers) = test.answers {
            let (temp_dir, plugin_path) = extract_harness_plugin_with_answers(answers)?;
            let plugin_dir = plugin_path
                .to_str()
                .ok_or_else(|| anyhow::anyhow!("Invalid plugin path"))?
                .to_string();
            args.push("--plugin-dir".to_string());
            args.push(plugin_dir);
            Some(temp_dir)
        } else {
            None
        };

        // Add test plugin if specified via --plugin-dir
        if let Some(ref test_plugin) = self.test_plugin_dir {
            let test_plugin_str = test_plugin
                .to_str()
                .ok_or_else(|| anyhow::anyhow!("Invalid test plugin path"))?;
            args.push("--plugin-dir".to_string());
            args.push(test_plugin_str.to_string());
        }

        args.push("--".to_string());
        args.push(test.test_prompt.trim().to_string());

        let mut child = Command::new(&self.claude_path)
            .args(&args)
            .current_dir(workspace.path())
            .env("CLAUDECODE", "") // Unset to avoid nested session
            .env("SKILL_BENCH_TEST", "1")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| anyhow::anyhow!("Failed to spawn claude: {}", e))?;

        // Open log file for writing with timestamps
        let log_file = std::fs::File::create(log_path)?;
        let mut log_writer = std::io::BufWriter::new(log_file);

        // Get stdout reader
        if let Some(stdout) = child.stdout.take() {
            let reader = BufReader::new(stdout);
            for line in reader.lines().map_while(Result::ok) {
                // Add timestamp to each JSON line
                if let Ok(mut json_val) = serde_json::from_str::<serde_json::Value>(&line) {
                    let current_time = SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap()
                        .as_secs_f64();
                    let relative_time = current_time - test_start;

                    // Add timestamp field to JSON
                    if let Some(obj) = json_val.as_object_mut() {
                        obj.insert(
                            "timestamp".to_string(),
                            serde_json::Value::Number(
                                serde_json::Number::from_f64(relative_time).unwrap(),
                            ),
                        );
                    }

                    if let Ok(modified_line) = serde_json::to_string(&json_val) {
                        writeln!(log_writer, "{}", modified_line).ok();
                    }
                } else {
                    // If not valid JSON, write as-is
                    writeln!(log_writer, "{}", line).ok();
                }
                log_writer.flush().ok();
            }
        }

        // Wait for process to complete
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
