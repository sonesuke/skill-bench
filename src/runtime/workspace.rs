//! Test workspace for isolated test execution

use crate::models::SetupStep;
use anyhow::Result;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::TempDir;

/// Test workspace for isolated test execution
pub struct TestWorkspace {
    #[allow(dead_code)]
    pub temp_dir: TempDir,
    pub work_dir: PathBuf,
    #[allow(dead_code)]
    pub log_file: PathBuf,
    #[allow(dead_code)]
    pub timestamp: String,
}

impl TestWorkspace {
    /// Create a new test workspace
    pub fn create(skill_name: &str, test_name: &str) -> Result<Self> {
        let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
        let workspace_name = format!("skill-bench-{}_{}-{}", timestamp, skill_name, test_name);

        // Create temporary directory
        let temp_dir = tempfile::tempdir()?;
        let work_dir = temp_dir.path().join(&workspace_name);
        fs::create_dir_all(&work_dir)?;

        // Create log file path
        let log_file = work_dir.join("execution.log");

        Ok(Self {
            temp_dir,
            work_dir,
            log_file,
            timestamp: timestamp.to_string(),
        })
    }

    /// Run setup steps in the workspace
    pub fn run_setup(&self, steps: &[SetupStep]) -> Result<()> {
        for step in steps {
            match step {
                SetupStep::File { path, content, .. } => {
                    let file_path = self.work_dir.join(path);
                    if let Some(parent) = file_path.parent() {
                        fs::create_dir_all(parent)?;
                    }
                    fs::write(&file_path, content)?;
                }
                SetupStep::Script { command, .. } => {
                    self.execute_script(command)?;
                }
            }
        }
        Ok(())
    }

    /// Execute a script in the workspace
    fn execute_script(&self, command: &str) -> Result<()> {
        let status = Command::new("bash")
            .args(["-c", command])
            .current_dir(&self.work_dir)
            .status()?;

        if !status.success() {
            anyhow::bail!("Script failed with status: {}", status);
        }

        Ok(())
    }

    /// Copy plugin directory to workspace
    #[allow(dead_code)]
    pub fn copy_plugin(&self, plugin_dir: &Path) -> Result<()> {
        if !plugin_dir.exists() {
            return Ok(());
        }

        let dest = self.work_dir.join("claude-plugin");

        // Use fs_extra to copy directories recursively
        let options = fs_extra::dir::CopyOptions::new();
        fs_extra::copy_items(&[plugin_dir], &dest, &options)?;

        Ok(())
    }

    /// Get the workspace directory path
    pub fn path(&self) -> &Path {
        &self.work_dir
    }

    /// Get the log file path
    #[allow(dead_code)]
    pub fn log_path(&self) -> &Path {
        &self.log_file
    }
}
