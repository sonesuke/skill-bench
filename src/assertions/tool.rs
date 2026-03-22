// Tool assertions
// Ported from check-tool-use.sh, check-param.sh

use crate::assertions::AssertionChecker;
use regex::Regex;
use serde_json::Value;

/// Check if tool was called
pub fn check_tool_use(
    checker: &AssertionChecker,
    tool_name: &str,
    param_name: Option<&str>,
    param_pattern: Option<&str>,
) -> Result<(), String> {
    // Find tool_use entries
    let tool_uses: Vec<_> = checker
        .log_data
        .iter()
        .filter_map(|entry| entry.get("message"))
        .filter_map(|msg| msg.get("content"))
        .filter_map(|content| content.as_array())
        .flat_map(|items| items.iter())
        .filter(|item| {
            item.get("type")
                .and_then(|t| t.as_str())
                .map(|t| t == "tool_use")
                .unwrap_or(false)
        })
        .collect();

    // First check if tool was called at all
    let tool_found = tool_uses.iter().any(|tool| {
        tool.get("name")
            .and_then(|n| n.as_str())
            .map(|n| n == tool_name)
            .unwrap_or(false)
    });

    if !tool_found {
        return Err(format!("Tool '{}' was not called", tool_name));
    }

    // Check for specific parameter if requested
    if let Some(param) = param_name {
        let param_found = tool_uses
            .iter()
            .filter(|tool| {
                tool.get("name")
                    .and_then(|n| n.as_str())
                    .map(|n| n == tool_name)
                    .unwrap_or(false)
            })
            .any(|tool| {
                tool.get("input")
                    .and_then(|input| input.get(param))
                    .is_some()
            });

        if !param_found {
            return Err(format!(
                "Parameter '{}' not found in tool '{}' call",
                param, tool_name
            ));
        }

        // Check parameter value pattern if requested
        if let Some(pattern) = param_pattern {
            let regex =
                Regex::new(pattern).map_err(|e| format!("Invalid regex '{}': {}", pattern, e))?;

            let pattern_matches = tool_uses
                .iter()
                .filter(|tool| {
                    tool.get("name")
                        .and_then(|n| n.as_str())
                        .map(|n| n == tool_name)
                        .unwrap_or(false)
                })
                .any(|tool| {
                    tool.get("input")
                        .and_then(|input| input.get(param))
                        .and_then(|p| p.as_str())
                        .map(|p| regex.is_match(p))
                        .unwrap_or(false)
                });

            if !pattern_matches {
                return Err(format!(
                    "Parameter '{}' in tool '{}' does not match pattern '{}'",
                    param, tool_name, pattern
                ));
            }
        }
    }

    Ok(())
}

/// Check if parameter was used with specific value
pub fn check_param(
    checker: &AssertionChecker,
    tool_name: &str,
    param_name: &str,
    expected: Option<&str>,
) -> Result<(), String> {
    // First verify tool was called with parameter
    check_tool_use(checker, tool_name, Some(param_name), None)?;

    // If expected value provided, check it
    if let Some(expected_value) = expected {
        let tool_uses: Vec<_> = checker
            .log_data
            .iter()
            .filter_map(|entry| entry.get("message"))
            .filter_map(|msg| msg.get("content"))
            .filter_map(|content| content.as_array())
            .flat_map(|items| items.iter())
            .filter(|item| {
                item.get("type")
                    .and_then(|t| t.as_str())
                    .map(|t| t == "tool_use")
                    .unwrap_or(false)
            })
            .filter(|tool| {
                tool.get("name")
                    .and_then(|n| n.as_str())
                    .map(|n| n == tool_name)
                    .unwrap_or(false)
            })
            .collect();

        let value_matches = tool_uses.iter().any(|tool| {
            tool.get("input")
                .and_then(|input| input.get(param_name))
                .map(|value| {
                    // Handle both string and array values
                    match value {
                        Value::String(s) => s == expected_value,
                        Value::Array(arr) => arr.iter().any(|item| {
                            item.as_str().map(|s| s == expected_value).unwrap_or(false)
                        }),
                        _ => false,
                    }
                })
                .unwrap_or(false)
        });

        if !value_matches {
            return Err(format!(
                "Parameter '{}' in tool '{}' does not match expected value '{}'",
                param_name, tool_name, expected_value
            ));
        }
    }

    Ok(())
}
