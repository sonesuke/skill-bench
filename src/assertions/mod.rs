// Assertion library - structured checks

pub mod database;
pub mod file;
pub mod log;
pub mod mcp;
pub mod skill;
pub mod tool;

#[cfg(test)]
mod tests;

use crate::models::CheckStep;
use serde_json::Value;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

/// Assertion checker that validates test results
pub struct AssertionChecker {
    #[cfg(test)]
    pub(crate) log_data: Vec<Value>,
    #[cfg(not(test))]
    log_data: Vec<Value>,
    work_dir: std::path::PathBuf,
}

impl AssertionChecker {
    /// Create a new assertion checker
    pub fn new(log_file: &Path, work_dir: &Path) -> Self {
        let log_data = Self::load_log_file(log_file);
        Self {
            log_data,
            work_dir: work_dir.to_path_buf(),
        }
    }

    /// Load and parse JSONL log file
    fn load_log_file(path: &Path) -> Vec<Value> {
        if !path.exists() {
            return Vec::new();
        }

        let file = match File::open(path) {
            Ok(f) => f,
            Err(_) => return Vec::new(),
        };

        let reader = BufReader::new(file);
        let mut entries = Vec::new();

        for line_text in reader.lines().map_while(Result::ok) {
            if let Ok(value) = serde_json::from_str::<Value>(&line_text) {
                entries.push(value);
            }
        }

        entries
    }

    /// Get the init message (first log entry)
    pub fn init_message(&self) -> Option<&Value> {
        self.log_data.first()
    }

    /// Evaluate a check step
    pub fn evaluate_check(&self, check: &CheckStep) -> Result<(), String> {
        let cmd = &check.command.command;
        let should_deny = check.deny || check.command.deny.unwrap_or(false);

        let result = match cmd.as_str() {
            "workspace-file" => {
                let path = check.command.path.as_ref().ok_or("Missing path")?;
                file::check_workspace_file(&self.work_dir, path)
            }
            "workspace-dir" => {
                let path = check.command.path.as_ref().ok_or("Missing path")?;
                file::check_workspace_dir(&self.work_dir, &[path])
            }
            "file-contains" => {
                let file = check.command.file.as_ref().ok_or("Missing file")?;
                let contains = check.command.contains.as_ref().ok_or("Missing contains")?;
                let deny = check.command.deny.unwrap_or(false);
                let effective_deny = should_deny || deny;
                file::check_file_content(&self.work_dir, file, contains, !effective_deny)
            }
            "log-contains" => {
                let pattern = check.command.pattern.as_ref().ok_or("Missing pattern")?;
                log::check_log_contains(&self.log_data, pattern)
            }
            "message-contains" => {
                let text = check.command.text.as_ref().ok_or("Missing text")?;
                log::check_output_contains(&self.log_data, text)
            }
            "skill-loaded" => {
                let skill = check.command.skill.as_ref().ok_or("Missing skill")?;
                skill::check_skill_loaded(self, skill)
            }
            "skill-invoked" => {
                let skill = check.command.skill.as_ref().ok_or("Missing skill")?;
                let deny = check.command.deny.unwrap_or(false);
                let effective_deny = should_deny || deny;
                skill::check_skill_invoked(self, skill, effective_deny)
            }
            "mcp-loaded" => {
                let server = check.command.server.as_ref().ok_or("Missing server")?;
                mcp::check_mcp_loaded(self, server)
            }
            "mcp-tool-invoked" => {
                let tool = check.command.tool.as_ref().ok_or("Missing tool")?;
                mcp::check_mcp_tool_invoked(self, tool)
            }
            "mcp-success" => {
                let tool = check.command.tool.as_ref().ok_or("Missing tool")?;
                mcp::check_mcp_success(self, tool, false)
            }
            "tool-use" => {
                let tool = check.command.tool.as_ref().ok_or("Missing tool")?;
                tool::check_tool_use(self, tool, None, None)
            }
            "tool-param" => {
                let tool = check.command.tool.as_ref().ok_or("Missing tool")?;
                let param = check.command.param.as_ref().ok_or("Missing param")?;
                tool::check_param(self, tool, param, check.command.value.as_deref())
            }
            "db-query" => {
                let expected = check.command.expected.as_ref().ok_or("Missing expected")?;
                let query = check.command.query.as_ref().ok_or("Missing query")?;
                let db = check.command.db.as_deref().unwrap_or("");
                database::check_db_query(&self.work_dir, db, expected, query)
            }
            _ => Err(format!("Unknown check command: {}", cmd)),
        };

        // Skip double inversion for checks that handle deny internally
        let needs_inversion =
            should_deny && !matches!(cmd.as_str(), "skill-invoked" | "file-contains");
        if needs_inversion {
            match result {
                Ok(_) => Err("Check passed but was marked as deny".to_string()),
                Err(_) => Ok(()),
            }
        } else {
            result
        }
    }
}
