//! CLI argument definitions using clap

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "skill-bench")]
#[command(about = "TOML-based test runner for skill testing", long_about = None)]
#[command(version)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Run tests
    Run {
        /// Test pattern (directory or glob, e.g., "cases" or "cases/**/*.toml")
        #[arg(default_value = "cases")]
        pattern: String,

        /// Filter by test name (regex)
        #[arg(short, long)]
        filter: Option<String>,

        /// Filter by skill name
        #[arg(long)]
        skill: Option<String>,

        /// Rerun only failed tests from last run
        #[arg(long)]
        rerun_failed: bool,

        /// Number of parallel threads
        #[arg(short = 'j', long, default_value_t = num_cpus::get())]
        threads: usize,

        /// Skills source directory (contains skills/ and agents/ subdirectories)
        #[arg(short, long)]
        skills_dir: Option<String>,

        /// Plugin directory to install (contains .claude-plugin/)
        #[arg(long)]
        plugin_dir: Option<String>,

        /// Log output directory for Claude session logs
        #[arg(short, long, default_value = "")]
        log: String,
    },
    /// List discovered tests
    List {
        /// Test pattern (directory or glob, e.g., "cases" or "cases/**/*.toml")
        #[arg(default_value = "cases")]
        pattern: String,
    },
}
