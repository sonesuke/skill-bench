// Skill assertions
// Ported from check-skill-loaded.sh, check-skill-invoked.sh, check-skill-not-invoked.sh

use crate::assertions::AssertionChecker;

/// Check if skill was loaded (present in init skills array or slash_commands)
pub fn check_skill_loaded(checker: &AssertionChecker, skill_name: &str) -> Result<(), String> {
    let init_msg = checker
        .init_message()
        .ok_or_else(|| "No log entries found".to_string())?;

    // Check skills array first
    let skills = init_msg
        .get("skills")
        .and_then(|s| s.as_array())
        .ok_or_else(|| "No skills array in init message".to_string())?;

    let found = skills
        .iter()
        .filter_map(|s| s.as_str())
        .any(|s| s.contains(skill_name));

    if found {
        return Ok(());
    }

    // Also check slash_commands for plugin-provided skills
    let slash_commands = init_msg.get("slash_commands").and_then(|s| s.as_array());

    if let Some(commands) = slash_commands {
        let found = commands
            .iter()
            .filter_map(|s| s.as_str())
            .any(|s| s.contains(skill_name));

        if found {
            return Ok(());
        }
    }

    Err(format!(
        "Skill '{}' not found in init skills array or slash_commands",
        skill_name
    ))
}

/// Check if skill was invoked
pub fn check_skill_invoked(checker: &AssertionChecker, skill_name: &str) -> Result<(), String> {
    // Search log for skill invocation
    // Pattern: "Skill" tool_use with "skill":"patent-kit:<skill-name>"
    let found = checker
        .log_data
        .iter()
        .filter_map(|entry| entry.get("message"))
        .filter_map(|msg| msg.get("content"))
        .filter_map(|content| content.as_array())
        .any(|items| {
            items.iter().any(|item| {
                item.get("type")
                    .and_then(|t| t.as_str())
                    .map(|t| t == "tool_use")
                    .unwrap_or(false)
                    && item
                        .get("name")
                        .and_then(|n| n.as_str())
                        .map(|n| n == "Skill")
                        .unwrap_or(false)
                    && item
                        .get("input")
                        .and_then(|i| i.get("skill"))
                        .and_then(|s| s.as_str())
                        .map(|s| s.contains(skill_name))
                        .unwrap_or(false)
            })
        });

    if found {
        Ok(())
    } else {
        Err(format!("Skill '{}' was not invoked", skill_name))
    }
}
