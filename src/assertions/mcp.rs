// MCP assertions
// Ported from check-mcp-loaded.sh, check-mcp-tool-invoked.sh, check-mcp-success.sh

use crate::assertions::AssertionChecker;

/// Check if MCP server loaded successfully
pub fn check_mcp_loaded(checker: &AssertionChecker, mcp_name: &str) -> Result<(), String> {
    let init_msg = checker
        .init_message()
        .ok_or_else(|| "No log entries found".to_string())?;

    let mcp_servers = init_msg
        .get("mcp_servers")
        .and_then(|s| s.as_array())
        .ok_or_else(|| "No mcp_servers array in init message".to_string())?;

    // Find the MCP server by name
    let status = mcp_servers
        .iter()
        .find(|server| {
            server
                .get("name")
                .and_then(|n| n.as_str())
                .map(|n| n.contains(mcp_name))
                .unwrap_or(false)
        })
        .and_then(|server| server.get("status"))
        .and_then(|s| s.as_str())
        .unwrap_or("not_found");

    match status {
        "connected" | "loaded" => Ok(()),
        "failed" => Err(format!(
            "MCP server '{}' failed to load (status: failed)",
            mcp_name
        )),
        "not_found" => Err(format!("MCP server '{}' not found in log", mcp_name)),
        _ => Err(format!("Unknown MCP server status: {}", status)),
    }
}

/// Check if MCP tool was invoked
pub fn check_mcp_tool_invoked(checker: &AssertionChecker, tool_name: &str) -> Result<(), String> {
    // Search for MCP tool invocation
    // Pattern: "name":"mcp__server__tool_name"
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
                        .map(|n| n.starts_with("mcp__") && n.ends_with(tool_name))
                        .unwrap_or(false)
            })
        });

    if found {
        Ok(())
    } else {
        Err(format!("MCP tool '{}' was not invoked", tool_name))
    }
}

/// Check if MCP tool calls succeeded
pub fn check_mcp_success(
    checker: &AssertionChecker,
    tool_name: &str,
    optional: bool,
) -> Result<(), String> {
    // Find all tool_use blocks for this MCP tool
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
                && item
                    .get("name")
                    .and_then(|n| n.as_str())
                    .map(|n| n.starts_with("mcp__") && n.ends_with(tool_name))
                    .unwrap_or(false)
        })
        .collect();

    if tool_uses.is_empty() {
        if optional {
            return Ok(());
        } else {
            return Err(format!("No MCP tool calls found for '{}'", tool_name));
        }
    }

    // Check for errors in corresponding tool_results
    for tool_use in tool_uses {
        let tool_id = tool_use
            .get("id")
            .and_then(|i| i.as_str())
            .ok_or_else(|| "Missing tool_use id".to_string())?;

        // Search for tool_result with matching tool_use_id
        let has_error = checker
            .log_data
            .iter()
            .filter_map(|entry| entry.get("message"))
            .filter_map(|msg| msg.get("content"))
            .filter_map(|content| content.as_array())
            .flat_map(|items| items.iter())
            .filter(|item| {
                item.get("type")
                    .and_then(|t| t.as_str())
                    .map(|t| t == "tool_result")
                    .unwrap_or(false)
            })
            .any(|item| {
                item.get("tool_use_id")
                    .and_then(|id| id.as_str())
                    .map(|id| id == tool_id)
                    .unwrap_or(false)
                    && item
                        .get("is_error")
                        .and_then(|e| e.as_bool())
                        .unwrap_or(false)
            });

        if has_error {
            return Err(format!(
                "MCP tool '{}' (id: {}) returned an error",
                tool_name, tool_id
            ));
        }
    }

    Ok(())
}
