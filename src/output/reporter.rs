//! Result reporting (nextest-style)

use crate::models::{TestResult, TestRunSummary, TestStatus};
use std::io::{self, Write};
use std::sync::Mutex;
use uuid::Uuid;

/// Thread-safe printer for real-time per-test output
pub struct LivePrinter {
    stdout: Mutex<Box<dyn Write + Send>>,
}

impl LivePrinter {
    pub fn new() -> std::sync::Arc<Self> {
        std::sync::Arc::new(Self {
            stdout: Mutex::new(Box::new(io::stdout())),
        })
    }

    pub fn print_test_result(&self, result: &TestResult) {
        let status_str = match result.status {
            TestStatus::Pass => "PASS",
            TestStatus::Fail => "FAIL",
            TestStatus::Skip => "SKIP",
        };
        let duration_secs = result.duration.as_secs_f64();
        let line = format!(
            "    {} [{:>8.3}s] {} {}",
            status_str, duration_secs, result.skill_name, result.test_name
        );
        let mut out = self.stdout.lock().unwrap();
        writeln!(out, "{}", line).ok();
        out.flush().ok();
    }
}

/// Reporter trait for different output formats
#[allow(dead_code)]
pub trait Reporter {
    /// Report the test run summary
    fn report_run(&self, summary: &TestRunSummary) -> io::Result<()>;
}

/// Human-readable reporter (nextest-style)
#[allow(dead_code)]
pub struct HumanReporter;

impl HumanReporter {
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self
    }
}

impl Reporter for HumanReporter {
    fn report_run(&self, summary: &TestRunSummary) -> io::Result<()> {
        let mut stdout = io::stdout();

        writeln!(stdout, "────────────────────────────────────────")?;
        writeln!(stdout, "SkillBench Test Results")?;
        writeln!(stdout, "────────────────────────────────────────")?;
        writeln!(
            stdout,
            "Total: {} | Pass: {} | Fail: {} | Skip: {}",
            summary.total, summary.passed, summary.failed, summary.skipped
        )?;
        writeln!(stdout, "Duration: {:.2}s", summary.duration.as_secs_f64())?;

        if !summary.failures().is_empty() {
            writeln!(stdout)?;
            writeln!(stdout, "Failed Tests:")?;
            for failure in summary.failures() {
                writeln!(
                    stdout,
                    "  ❌ {}/{} ({:.2}s)",
                    failure.skill_name,
                    failure.test_name,
                    failure.duration.as_secs_f64()
                )?;
            }
        }

        Ok(())
    }
}

/// JSON reporter for CI/automation
#[allow(dead_code)]
pub struct JsonReporter;

impl Reporter for JsonReporter {
    fn report_run(&self, summary: &TestRunSummary) -> io::Result<()> {
        let json = serde_json::to_string_pretty(summary).map_err(io::Error::other)?;
        println!("{}", json);
        Ok(())
    }
}

pub fn print_run_header(run_id: &Uuid, test_count: usize, skill_count: usize) {
    println!(" SkillBench run ID {} with profile: default", run_id);
    println!(
        "    Starting {} tests across {} skill(s)",
        test_count, skill_count
    );
}

pub fn print_separator() {
    println!("------------");
}

pub fn print_summary_line(
    wall_clock: std::time::Duration,
    total: usize,
    passed: usize,
    failed: usize,
    skipped: usize,
) {
    if skipped > 0 {
        println!(
            "     Summary [{:>8.3}s] {} tests run: {} passed, {} failed, {} skipped",
            wall_clock.as_secs_f64(),
            total,
            passed,
            failed,
            skipped,
        );
    } else {
        println!(
            "     Summary [{:>8.3}s] {} tests run: {} passed, {} failed",
            wall_clock.as_secs_f64(),
            total,
            passed,
            failed,
        );
    }
}

pub fn print_failed_details(failures: &[&TestResult]) {
    for failure in failures {
        println!();
        println!(
            "    FAIL [{:.3}s] {} {}",
            failure.duration.as_secs_f64(),
            failure.skill_name,
            failure.test_name,
        );
        if let Some(ref err) = failure.execution_error {
            println!("        execution error: {}", err);
        }
        for check_result in &failure.check_results {
            if !check_result.passed {
                if let Some(ref error) = check_result.error {
                    println!("        - {}: {}", check_result.name, error);
                }
            }
        }
    }
}
