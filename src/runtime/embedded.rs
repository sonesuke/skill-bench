//! Embedded harness plugin resources

use anyhow::Result;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

/// Extract harness plugin with answers injected into question-responder SKILL.md
/// Returns (temp_dir, plugin_dir)
pub fn extract_harness_plugin_with_answers(
    answers: &HashMap<String, toml::Value>,
) -> Result<(tempfile::TempDir, PathBuf)> {
    extract_harness_plugin_inner(Some(answers))
}

fn extract_harness_plugin_inner(
    answers: Option<&HashMap<String, toml::Value>>,
) -> Result<(tempfile::TempDir, PathBuf)> {
    let temp_dir = tempfile::tempdir()?;
    let plugin_dir = temp_dir.path().join("claude-plugin");
    let claude_plugin_dir = plugin_dir.join(".claude-plugin");
    fs::create_dir_all(&claude_plugin_dir)?;

    // .claude-plugin/plugin.json
    let plugin_json = include_bytes!("../assets/harness-plugin/.claude-plugin/plugin.json");
    fs::write(claude_plugin_dir.join("plugin.json"), plugin_json)?;

    // skills/question-responder/SKILL.md (with optional answers)
    let skill_md_template = include_bytes!("../assets/harness-plugin/skills/question-responder/SKILL.md");
    let skill_md = String::from_utf8_lossy(skill_md_template);

    let skill_md = if let Some(answers) = answers {
        let answers_section = format_answers_section(answers);
        skill_md.replace("{{ANSWERS_SECTION}}", &answers_section)
    } else {
        skill_md.replace("{{ANSWERS_SECTION}}", "No pre-configured answers available for this test.")
    };

    let skill_dir = plugin_dir.join("skills").join("question-responder");
    fs::create_dir_all(&skill_dir)?;
    fs::write(skill_dir.join("SKILL.md"), skill_md.as_bytes())?;

    Ok((temp_dir, plugin_dir))
}

/// Format answers HashMap into a markdown section
fn format_answers_section(answers: &HashMap<String, toml::Value>) -> String {
    let mut lines = String::from("## Available Answers\n\nThe following information is available for this test:\n\n");

    for (key, value) in answers {
        let formatted = match value {
            toml::Value::Array(arr) => {
                let items: Vec<String> = arr
                    .iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect();
                items.join(", ")
            }
            toml::Value::String(s) => s.clone(),
            other => other.to_string(),
        };
        lines.push_str(&format!("- **{}**: {}\n", key, formatted));
    }

    lines
}

/// Get the path to the harness plugin (for development fallback)
#[allow(dead_code)]
pub fn get_harness_plugin_path() -> PathBuf {
    PathBuf::from("src/assets/harness-plugin")
}
