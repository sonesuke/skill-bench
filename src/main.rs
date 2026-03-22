mod assertions;
mod cli;
mod models;
mod output;
mod runtime;
mod state;

use anyhow::Result;
use clap::Parser;
use cli::{Cli, Commands};
use models::TestRunSummary;
use runtime::{TestDiscovery, TestExecutor};
use state::TestHistory;

/// Configuration for running tests
struct TestConfig {
    pattern: String,
    filter: Option<String>,
    skill: Option<String>,
    rerun_failed: bool,
    threads: usize,
    plugin_dir: Option<String>,
    log: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into()),
        )
        .init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Run {
            pattern,
            filter,
            skill,
            rerun_failed,
            threads,
            plugin_dir,
            log,
        } => {
            let config = TestConfig {
                pattern,
                filter,
                skill,
                rerun_failed,
                threads,
                plugin_dir,
                log,
            };
            run_tests(config)?;
        }
        Commands::List { pattern } => {
            list_tests(pattern)?;
        }
    }

    Ok(())
}

fn run_tests(config: TestConfig) -> Result<()> {
    // Discover tests
    let discovery = TestDiscovery::new(config.pattern);
    let mut tests = discovery.discover()?;

    println!("Discovered {} tests", tests.len());

    // Apply filters
    if let Some(skill_name) = config.skill {
        tests.retain(|t| t.skill_name == skill_name);
        println!("Filtered by skill: {}", skill_name);
    }

    if let Some(filter_pattern) = config.filter {
        let regex = regex::Regex::new(&filter_pattern)?;
        tests.retain(|t| regex.is_match(&t.test_name));
        println!("Filtered by pattern: {}", filter_pattern);
    }

    // Handle --rerun-failed
    if config.rerun_failed {
        let history = TestHistory::load();
        let failed_ids = history.get_failed_test_ids();

        let before_count = tests.len();
        tests.retain(|t| failed_ids.contains(&t.test_id));

        if tests.is_empty() {
            println!("No failed tests to rerun");
            return Ok(());
        }

        println!(
            "Rerunning {} failed tests (from {} total)",
            tests.len(),
            before_count
        );
    }

    if tests.is_empty() {
        println!("No tests to run");
        return Ok(());
    }

    println!();

    // Execute tests
    let executor = TestExecutor::new(config.threads, Some(config.log.clone()), config.plugin_dir)?;
    let results = executor.execute_tests(tests)?;

    // Create summary
    let summary = TestRunSummary::from_results(results);

    // Print summary
    print_summary(&summary);

    // Update history
    let mut history = TestHistory::load();
    history.update(&summary.results);

    // Exit with error code if any tests failed
    if summary.failed > 0 {
        std::process::exit(1);
    }

    Ok(())
}

fn list_tests(pattern: String) -> Result<()> {
    let discovery = TestDiscovery::new(pattern);
    let tests = discovery.discover()?;

    println!("Discovered {} tests:", tests.len());
    for test in &tests {
        println!(
            "  {}/{} - {}",
            test.skill_name, test.test_name, test.test.description
        );
    }

    Ok(())
}

fn print_summary(summary: &TestRunSummary) {
    println!();
    println!("═{}═", "═".repeat(50));
    println!("SkillBench Test Results");
    println!("═{}═", "═".repeat(50));
    println!(
        "Total: {} | Pass: {} | Fail: {}",
        summary.total, summary.passed, summary.failed
    );
    println!("Duration: {:.2}s", summary.duration.as_secs_f64());

    // List failed tests
    if !summary.failures().is_empty() {
        println!();
        println!("Failed Tests:");
        for failure in summary.failures() {
            println!(
                "  ❌ {}/{} ({:.2}s)",
                failure.skill_name,
                failure.test_name,
                failure.duration.as_secs_f64()
            );

            // Show failed checks
            for check_result in &failure.check_results {
                if !check_result.passed {
                    if let Some(ref error) = check_result.error {
                        println!("    - {}: {}", check_result.name, error);
                    }
                }
            }
        }
    }

    println!("═{}═", "═".repeat(50));
}
