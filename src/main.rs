mod assertions;
mod cli;
mod models;
mod output;
mod reference;
mod runtime;
mod state;
mod timeline;

use anyhow::Result;
use clap::Parser;
use cli::{Cli, Commands};
use models::TestRunSummary;
use output::{
    print_failed_details, print_run_header, print_separator, print_summary_line, LivePrinter,
};
use runtime::{TestDiscovery, TestExecutor};
use state::TestHistory;
use std::collections::HashSet;
use std::time::Instant;

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
        Commands::Timeline { log_file, verbose } => {
            timeline::display_timeline(&log_file, verbose)?;
        }
        Commands::Help { check_type } => {
            reference::print_help(check_type.as_deref())?;
        }
    }

    Ok(())
}

fn run_tests(config: TestConfig) -> Result<()> {
    // Discover tests
    let discovery = TestDiscovery::new(config.pattern);
    let mut tests = discovery.discover()?;

    // Apply filters
    if let Some(skill_name) = config.skill {
        tests.retain(|t| t.skill_name == skill_name);
    }

    if let Some(filter_pattern) = config.filter {
        let regex = regex::Regex::new(&filter_pattern)?;
        tests.retain(|t| regex.is_match(&t.test_name));
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

    // Count unique skills
    let skill_count: HashSet<String> = tests.iter().map(|t| t.skill_name.clone()).collect();

    // Generate run ID
    let run_id = uuid::Uuid::new_v4();

    // Print header
    print_run_header(&run_id, tests.len(), skill_count.len());

    // Create live printer
    let printer = LivePrinter::new();

    // Execute tests with real-time output
    let executor = TestExecutor::new(config.threads, Some(config.log.clone()), config.plugin_dir)?;
    let run_start = Instant::now();
    let results = executor.execute_tests(tests, {
        let printer = printer.clone();
        move |result| printer.print_test_result(result)
    })?;
    let wall_clock = run_start.elapsed();

    // Print summary
    let summary = TestRunSummary::from_results(results);

    println!();
    print_separator();
    print_summary_line(
        wall_clock,
        summary.total,
        summary.passed,
        summary.failed,
        summary.skipped,
    );

    // Print failed test details
    let failures = summary.failures();
    if !failures.is_empty() {
        print_failed_details(&failures);
    }

    println!();

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
