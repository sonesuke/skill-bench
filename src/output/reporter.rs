//! Result reporting (nextest-style)

use crate::models::TestRunSummary;
use std::io::{self, Write};

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
