//! Embedded harness plugin resources

use anyhow::Result;
use std::fs;
use std::path::PathBuf;

/// Extract embedded harness plugin to a temporary directory
/// Returns (temp_dir, plugin_dir)
pub fn extract_harness_plugin() -> Result<(tempfile::TempDir, PathBuf)> {
    let temp_dir = tempfile::tempdir()?;
    let plugin_dir = temp_dir.path().join("claude-plugin");
    let claude_plugin_dir = plugin_dir.join(".claude-plugin");
    fs::create_dir_all(&claude_plugin_dir)?;

    // .claude-plugin/plugin.json
    let plugin_json = include_bytes!("../assets/harness-plugin/.claude-plugin/plugin.json");
    fs::write(claude_plugin_dir.join("plugin.json"), plugin_json)?;

    // skills/question-responder/SKILL.md
    let skill_md = include_bytes!("../assets/harness-plugin/skills/question-responder/SKILL.md");
    let skill_dir = plugin_dir.join("skills").join("question-responder");
    fs::create_dir_all(&skill_dir)?;
    fs::write(skill_dir.join("SKILL.md"), skill_md)?;

    Ok((temp_dir, plugin_dir))
}

/// Get the path to the harness plugin (for development fallback)
#[allow(dead_code)]
pub fn get_harness_plugin_path() -> PathBuf {
    PathBuf::from("src/assets/harness-plugin")
}
