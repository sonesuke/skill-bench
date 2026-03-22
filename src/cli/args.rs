//! CLI argument definitions using clap

use clap::{Parser, Subcommand, ValueEnum};
use std::fmt;

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
        /// Test pattern (e.g., "old-implementation-skill-bench/cases/*/*.toml")
        #[arg(default_value = "old-implementation-skill-bench/cases/*/*.toml")]
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

        /// Log level
        #[arg(long, default_value = "fail")]
        log_level: LogLevel,

        /// Output format
        #[arg(long, default_value = "human")]
        format: OutputFormat,
    },
    /// List discovered tests
    List {
        /// Test pattern
        #[arg(default_value = "old-implementation-skill-bench/cases/*/*.toml")]
        pattern: String,
    },
}

#[derive(Clone, Copy, Debug, Default, ValueEnum)]
pub enum LogLevel {
    /// Show only errors
    Error,
    /// Show failed tests
    #[default]
    Fail,
    /// Show passed tests
    Pass,
    /// Show all tests
    All,
}

impl fmt::Display for LogLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LogLevel::Error => write!(f, "error"),
            LogLevel::Fail => write!(f, "fail"),
            LogLevel::Pass => write!(f, "pass"),
            LogLevel::All => write!(f, "all"),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, ValueEnum)]
pub enum OutputFormat {
    /// Human-readable output
    #[default]
    Human,
    /// JSON output
    Json,
    /// JSON Lines output
    Jsonl,
}

impl fmt::Display for OutputFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OutputFormat::Human => write!(f, "human"),
            OutputFormat::Json => write!(f, "json"),
            OutputFormat::Jsonl => write!(f, "jsonl"),
        }
    }
}
