// Log assertions
// Ported from check-output-contains.sh, check-log-contains.sh, check-text-contains.sh

use serde_json::Value;

/// Check if assistant output contains text
pub fn check_output_contains(log_data: &[Value], text: &str) -> Result<(), String> {
    let found = log_data
        .iter()
        .filter(|entry| {
            entry
                .get("type")
                .and_then(|t| t.as_str())
                .map(|t| t == "assistant")
                .unwrap_or(false)
        })
        .any(|entry| {
            entry
                .get("message")
                .and_then(|m| m.get("content"))
                .map(|c| c.to_string().to_lowercase().contains(&text.to_lowercase()))
                .unwrap_or(false)
        });

    if found {
        Ok(())
    } else {
        Err(format!("Output does not contain expected text '{}'", text))
    }
}

/// Check if log contains pattern (regex)
pub fn check_log_contains(log_data: &[Value], pattern: &str) -> Result<(), String> {
    let regex =
        regex::Regex::new(pattern).map_err(|e| format!("Invalid regex '{}': {}", pattern, e))?;

    let log_text = log_data
        .iter()
        .map(|v| v.to_string())
        .collect::<Vec<_>>()
        .join("\n");

    if regex.is_match(&log_text) {
        Ok(())
    } else {
        Err(format!("Log does not contain pattern '{}'", pattern))
    }
}

/// Check if text appears in any assistant message
#[allow(dead_code)]
pub fn check_text_contains(log_data: &[Value], patterns: &[&str]) -> Result<(), String> {
    // Check if any pattern is found in assistant messages
    for pattern in patterns {
        if check_output_contains(log_data, pattern).is_ok() {
            return Ok(());
        }
    }

    Err(format!(
        "Text does not contain any of the expected patterns: {:?}",
        patterns
    ))
}
